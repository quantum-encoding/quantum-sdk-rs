use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiErrorBody, Error, Result};
use crate::region::Region;

/// Max retries for transient errors (502, 503, 429).
const MAX_RETRIES: u32 = 3;
/// Initial backoff delay.
const INITIAL_BACKOFF_MS: u64 = 500;

/// Check if a status code is retryable.
fn is_retryable(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 502 | 503 | 504)
}

/// Check if an error response body contains a permanent (non-retryable) error
/// even when wrapped in a retryable status code (e.g. 502 wrapping a provider 400).
fn is_permanent_error(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("content moderation")
        || lower.contains("content_policy")
        || lower.contains("safety_block")
        || lower.contains("invalid argument")
        || lower.contains("invalid_request")
        || (lower.contains("status 400") && lower.contains("rejected"))
}

/// The default Quantum AI API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.quantumencoding.ai";

/// The number of ticks in one US dollar (10 billion).
pub const TICKS_PER_USD: i64 = 10_000_000_000;

/// Common response metadata parsed from HTTP headers.
#[derive(Debug, Clone, Default)]
pub struct ResponseMeta {
    /// Cost in ticks from X-QAI-Cost-Ticks header.
    pub cost_ticks: i64,
    /// Post-deduction credit balance in ticks from X-QAI-Balance-After header.
    /// Zero if the server didn't include the header (e.g. cached / free calls).
    pub balance_after: i64,
    /// Request identifier from X-QAI-Request-Id header.
    pub request_id: String,
    /// Model identifier from X-QAI-Model header.
    pub model: String,
}

/// Builder for constructing a [`Client`] with custom configuration.
pub struct ClientBuilder {
    api_key: String,
    base_url: String,
    timeout: Duration,
    app: Option<String>,
    region: Option<Region>,
    extra_headers: Vec<(String, String)>,
}

/// Header names that callers may not override via `extra_header` / `app`.
/// Attempts to set these return an error at `build()` so auth can never be
/// silently clobbered by a caller-supplied header.
fn is_reserved_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization") || name.eq_ignore_ascii_case("x-api-key")
}

fn invalid_header_error(message: String) -> Error {
    Error::Api(ApiError {
        status_code: 0,
        code: "invalid_header".to_string(),
        message,
        request_id: String::new(),
    })
}

impl ClientBuilder {
    /// Creates a new builder with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            // 120s could abort long buffered media generation (image/video return
            // a single JSON blob only when the provider finishes). 600s clears the
            // backend's 5-minute media deadline so the server errors first.
            // Streaming uses a separate no-timeout client.
            timeout: Duration::from_secs(600),
            app: None,
            region: None,
            extra_headers: Vec::new(),
        }
    }

    /// Sets the API base URL (default: `https://api.quantumencoding.ai`).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Sets the request timeout for non-streaming requests (default: 120s).
    ///
    /// Media generation endpoints (video, dubbing, 3D) can take 1–5 minutes.
    /// For these, use `Duration::from_secs(300)` or longer. Alternatively,
    /// use the async jobs API (`create_job` / `poll_job`) which doesn't block.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Tags every request with the calling app's identifier.
    ///
    /// Sent as `X-Quantum-App: <app>` on every HTTP request (including streaming).
    /// The server uses this to route requests through app-specific paywall,
    /// quota, or dispatch logic — for example, the Recipe Box trial-paywall
    /// gate on `/qai/v1/chat`.
    ///
    /// Thin convenience wrapper around [`extra_header`](Self::extra_header).
    /// If both `app(...)` and `extra_header("X-Quantum-App", ...)` are set,
    /// the value from `app(...)` wins.
    pub fn app(mut self, app: impl Into<String>) -> Self {
        self.app = Some(app.into());
        self
    }

    /// Routes this client's gateway CHAT calls through a region
    /// (region-scoped inference routing — EU AI Act Art 50).
    ///
    /// The region rides `provider_options.region` on every chat request the
    /// client sends — streaming included — unless the request itself already
    /// sets one ([`crate::ChatRequest::region`] wins). Only `/qai/v1/chat`
    /// honors the override; the agent endpoint routes by the key's scope.
    /// Non-chat endpoints are unaffected.
    ///
    /// For keys minted with a region scope the scope already routes every
    /// request — this hook is for the per-client choice (e.g. an app's user
    /// picking their region) on top of an unscoped key.
    pub fn region(mut self, region: Region) -> Self {
        self.region = Some(region);
        self
    }

    /// Adds an extra HTTP header to every request from this client.
    ///
    /// Useful for app identification, request tagging, A/B routing, etc.
    /// Standard headers (`Authorization`, `X-API-Key`) are managed by the
    /// builder and cannot be overridden — passing either here causes
    /// [`build`](Self::build) to return an `invalid_header` error.
    ///
    /// Header names and values are validated at `build()` time, not here.
    pub fn extra_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.extra_headers.push((name.into(), value.into()));
        self
    }

    /// Builds the [`Client`].
    pub fn build(self) -> Result<Client> {
        let auth_value = format!("Bearer {}", self.api_key);
        let auth_header = HeaderValue::from_str(&auth_value).map_err(|_| {
            Error::Api(ApiError {
                status_code: 0,
                code: "invalid_api_key".to_string(),
                message: "API key contains invalid header characters".to_string(),
                request_id: String::new(),
            })
        })?;

        // Resolve caller-supplied headers, with app() winning over any
        // duplicate extra_header("X-Quantum-App", ...).
        let mut caller_headers = self.extra_headers.clone();
        if let Some(app) = self.app.as_ref() {
            caller_headers.push(("X-Quantum-App".to_string(), app.clone()));
        }

        // Parse + validate caller headers up front so we can return a single
        // typed error rather than failing partway through HeaderMap mutation.
        let mut extra_headers_map = HeaderMap::new();
        for (name, value) in &caller_headers {
            if is_reserved_header(name) {
                return Err(invalid_header_error(format!(
                    "header '{name}' is reserved by the SDK and cannot be overridden via extra_header"
                )));
            }
            let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
                invalid_header_error(format!("invalid header name '{name}': {e}"))
            })?;
            let header_value = HeaderValue::from_str(value).map_err(|e| {
                invalid_header_error(format!("invalid header value for '{name}': {e}"))
            })?;
            extra_headers_map.insert(header_name, header_value);
        }

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth_header.clone());
        // Also send X-API-Key for proxies that claim the Authorization header (e.g. Cloudflare -> Cloud Run IAM).
        if let Ok(v) = HeaderValue::from_str(&self.api_key) {
            headers.insert("X-API-Key", v);
        }
        // Caller-supplied headers are inserted *after* auth so the reserved
        // guard above is the only way to override standard SDK headers.
        for (name, value) in &extra_headers_map {
            headers.insert(name.clone(), value.clone());
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(self.timeout)
            .build()?;

        Ok(Client {
            inner: Arc::new(ClientInner {
                base_url: self.base_url,
                http,
                auth_header,
                extra_headers: extra_headers_map,
                region: self.region,
            }),
        })
    }
}

struct ClientInner {
    base_url: String,
    http: reqwest::Client,
    auth_header: HeaderValue,
    /// Client-level routing region applied to chat requests (see
    /// [`ClientBuilder::region`]).
    region: Option<Region>,
    /// Caller-supplied headers (via `ClientBuilder::extra_header` /
    /// `ClientBuilder::app`). Already merged into the non-streaming
    /// client's `default_headers`; the streaming paths build fresh
    /// `reqwest::Client`s without defaults and must apply these
    /// per-request.
    extra_headers: HeaderMap,
}

/// The Quantum AI API client.
///
/// `Client` is cheaply cloneable (backed by `Arc`) and safe to share across tasks.
///
/// # Example
///
/// ```no_run
/// let client = quantum_sdk::Client::new("qai_key_xxx");
/// ```
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl Client {
    /// Creates a new client with the given API key and default settings.
    pub fn new(api_key: impl Into<String>) -> Self {
        ClientBuilder::new(api_key)
            .build()
            .expect("default client configuration is valid")
    }

    /// Returns a [`ClientBuilder`] for custom configuration.
    pub fn builder(api_key: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Returns the base URL for this client.
    pub(crate) fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// The routing region applied to this client's chat requests, if one
    /// was configured via [`ClientBuilder::region`].
    pub fn region(&self) -> Option<Region> {
        self.inner.region
    }

    /// Returns the auth header value (e.g. "Bearer qai_xxx").
    pub(crate) fn auth_header(&self) -> &HeaderValue {
        &self.inner.auth_header
    }

    /// Sends a JSON POST request and deserializes the response.
    ///
    /// An `Idempotency-Key` header is automatically generated and reused across
    /// retries, preventing duplicate charges when a 502/504 masks a successful
    /// backend operation.
    pub async fn post_json<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<(Resp, ResponseMeta)> {
        self.post_json_with_idempotency(path, body, None).await
    }

    /// Like [`post_json`](Self::post_json) but with a caller-supplied
    /// idempotency key.
    ///
    /// When `idempotency_key` is `Some`, that value is sent as the
    /// `Idempotency-Key` header on every retry attempt (the backend
    /// deduplicates on it). When `None`, a random UUID is generated
    /// per call — matching the default [`post_json`](Self::post_json)
    /// behavior.
    ///
    /// Pass a deterministic key for fan-out / queue / retry scenarios
    /// where the same logical request may be issued by multiple workers
    /// (or re-issued after a crash) and must not create duplicate
    /// charges. The key is reused across retries so a transient 502/504
    /// masking a successful backend operation won't double-charge.
    pub async fn post_json_with_idempotency<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
        idempotency_key: Option<String>,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let body_bytes = serde_json::to_vec(body)?;
        // Caller key wins; otherwise generate one. Either way the same
        // key is reused across retries so the backend deduplicates.
        let idempotency_key =
            idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string());

        let mut last_err = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                eprintln!("[sdk] Retry {attempt}/{MAX_RETRIES} for POST {path} in {delay}ms");
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            let resp = self
                .inner
                .http
                .post(&url)
                .header(CONTENT_TYPE, "application/json")
                .header("Idempotency-Key", &idempotency_key)
                .body(body_bytes.clone())
                .send()
                .await?;

            let status = resp.status();
            let meta = parse_response_meta(&resp);

            if status.is_success() {
                let body_text = resp.text().await?;
                let result: Resp = serde_json::from_str(&body_text).map_err(|e| {
                    let preview = if body_text.len() > 300 { &body_text[..300] } else { &body_text };
                    eprintln!("[sdk] JSON decode error on {path}: {e}\n  body preview: {preview}");
                    e
                })?;
                return Ok((result, meta));
            }

            if is_retryable(status) && attempt < MAX_RETRIES {
                // Read body to check if it's a permanent error wrapped in 502
                let body_text = resp.text().await.unwrap_or_default();
                if is_permanent_error(&body_text) {
                    eprintln!("[sdk] POST {path} returned {status} but error is permanent, not retrying");
                    let err = parse_api_error_from_text(status, &body_text, &meta.request_id);
                    return Err(err);
                }
                eprintln!("[sdk] POST {path} returned {status}, will retry");
                let err = parse_api_error_from_text(status, &body_text, &meta.request_id);
                last_err = Some(err);
                continue;
            }

            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        Err(last_err.unwrap_or_else(|| Error::Api(ApiError {
            status_code: 502,
            code: "retry_exhausted".into(),
            message: format!("max retries ({MAX_RETRIES}) exceeded"),
            request_id: String::new(),
        })))
    }

    /// Sends a POST request and returns the raw JSON response.
    /// Useful for fire-and-forget endpoints (logging, telemetry) where
    /// the response type isn't worth defining a struct for.
    pub async fn post_raw(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let (resp, _meta): (serde_json::Value, _) = self.post_json(path, body).await?;
        Ok(resp)
    }

    /// Sends a GET request and deserializes the response.
    pub async fn get_json<Resp: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);

        let mut last_err = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                eprintln!("[sdk] Retry {attempt}/{MAX_RETRIES} for GET {path} in {delay}ms");
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            let resp = self.inner.http.get(&url).send().await?;
            let status = resp.status();
            let meta = parse_response_meta(&resp);

            if status.is_success() {
                let body_text = resp.text().await?;
                let result: Resp = serde_json::from_str(&body_text).map_err(|e| {
                    let preview = if body_text.len() > 300 { &body_text[..300] } else { &body_text };
                    eprintln!("[sdk] JSON decode error on {path}: {e}\n  body preview: {preview}");
                    e
                })?;
                return Ok((result, meta));
            }

            if is_retryable(status) && attempt < MAX_RETRIES {
                let body_text = resp.text().await.unwrap_or_default();
                if is_permanent_error(&body_text) {
                    eprintln!("[sdk] GET {path} returned {status} but error is permanent, not retrying");
                    return Err(parse_api_error_from_text(status, &body_text, &meta.request_id));
                }
                eprintln!("[sdk] GET {path} returned {status}, will retry");
                last_err = Some(parse_api_error_from_text(status, &body_text, &meta.request_id));
                continue;
            }

            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        Err(last_err.unwrap_or_else(|| Error::Api(ApiError {
            status_code: 502,
            code: "retry_exhausted".into(),
            message: format!("max retries ({MAX_RETRIES}) exceeded"),
            request_id: String::new(),
        })))
    }

    /// Sends a DELETE request and deserializes the response.
    pub async fn delete_json<Resp: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let resp = self.inner.http.delete(&url).send().await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        let result: Resp = resp.json().await?;
        Ok((result, meta))
    }

    /// Sends a POST request with an empty body and deserializes the response.
    pub async fn post_json_empty<Resp: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<(Resp, ResponseMeta)> {
        self.post_json_empty_with_idempotency(path, None).await
    }

    /// Like [`post_json_empty`](Self::post_json_empty) but with a
    /// caller-supplied idempotency key. See
    /// [`post_json_with_idempotency`](Self::post_json_with_idempotency)
    /// for the rationale.
    pub async fn post_json_empty_with_idempotency<Resp: DeserializeOwned>(
        &self,
        path: &str,
        idempotency_key: Option<String>,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let key = idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string());
        let resp = self.inner.http.post(&url)
            .header("content-type", "application/json")
            .header("Idempotency-Key", key)
            .body("{}")
            .send()
            .await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        let result: Resp = resp.json().await?;
        Ok((result, meta))
    }

    /// Sends a PUT request with a JSON body and deserializes the response.
    pub async fn put_json<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let resp = self.inner.http.put(&url).json(body).send().await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        let result: Resp = resp.json().await?;
        Ok((result, meta))
    }

    /// Sends a multipart POST request and deserializes the response.
    pub async fn post_multipart<Resp: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<(Resp, ResponseMeta)> {
        self.post_multipart_with_idempotency(path, form, None).await
    }

    /// Like [`post_multipart`](Self::post_multipart) but with a
    /// caller-supplied idempotency key. See
    /// [`post_json_with_idempotency`](Self::post_json_with_idempotency)
    /// for the rationale.
    pub async fn post_multipart_with_idempotency<Resp: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
        idempotency_key: Option<String>,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let key = idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string());
        let resp = self.inner.http.post(&url)
            .header("Idempotency-Key", key)
            .multipart(form)
            .send()
            .await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        let result: Resp = resp.json().await?;
        Ok((result, meta))
    }

    /// Sends a GET request expecting an SSE stream response.
    /// Returns the raw reqwest::Response for the caller to read events from.
    /// Uses a separate client without timeout — cancellation is via drop.
    pub async fn get_stream_raw(
        &self,
        path: &str,
    ) -> Result<(reqwest::Response, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);

        let stream_client = reqwest::Client::builder().build()?;

        let mut req = stream_client
            .get(&url)
            .header(AUTHORIZATION, self.inner.auth_header.clone())
            .header("Accept", "text/event-stream");
        for (name, value) in &self.inner.extra_headers {
            req = req.header(name, value);
        }
        let resp = req.send().await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        Ok((resp, meta))
    }

    /// Sends a JSON POST request expecting an SSE stream response.
    /// Returns the raw reqwest::Response for the caller to read events from.
    /// Uses a separate client without timeout -- cancellation is via drop.
    pub async fn post_stream_raw(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<(reqwest::Response, ResponseMeta)> {
        self.post_stream_raw_with_idempotency(path, body, None).await
    }

    /// Like [`post_stream_raw`](Self::post_stream_raw) but with a
    /// caller-supplied idempotency key. See
    /// [`post_json_with_idempotency`](Self::post_json_with_idempotency)
    /// for the rationale. Streaming requests are single-attempt (no
    /// retry), but the key still lets the backend deduplicate a
    /// re-issued stream request after a client crash or reconnect.
    pub async fn post_stream_raw_with_idempotency(
        &self,
        path: &str,
        body: &impl Serialize,
        idempotency_key: Option<String>,
    ) -> Result<(reqwest::Response, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);

        // Build a client without timeout for streaming.
        let stream_client = reqwest::Client::builder().build()?;

        let key = idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string());
        let mut req = stream_client
            .post(&url)
            .header(AUTHORIZATION, self.inner.auth_header.clone())
            .header(CONTENT_TYPE, "application/json")
            .header("Accept", "text/event-stream")
            .header("Idempotency-Key", key);
        for (name, value) in &self.inner.extra_headers {
            req = req.header(name, value);
        }
        let resp = req.json(body).send().await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        Ok((resp, meta))
    }
}

/// Extracts response metadata from HTTP headers.
fn parse_response_meta(resp: &reqwest::Response) -> ResponseMeta {
    let headers = resp.headers();
    let request_id = headers
        .get("X-QAI-Request-Id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let model = headers
        .get("X-QAI-Model")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let cost_ticks = headers
        .get("X-QAI-Cost-Ticks")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    let balance_after = headers
        .get("X-QAI-Balance-After")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);

    ResponseMeta {
        cost_ticks,
        balance_after,
        request_id,
        model,
    }
}

/// Parses an API error from a non-2xx response.
async fn parse_api_error(resp: reqwest::Response, request_id: &str) -> Error {
    let status_code = resp.status().as_u16();
    let status_text = resp
        .status()
        .canonical_reason()
        .unwrap_or("Unknown")
        .to_string();

    let body = resp.text().await.unwrap_or_default();

    let (code, message) = if let Ok(err_body) = serde_json::from_str::<ApiErrorBody>(&body) {
        let msg = if err_body.error.message.is_empty() {
            body.clone()
        } else {
            err_body.error.message
        };
        let c = if !err_body.error.code.is_empty() {
            err_body.error.code
        } else if !err_body.error.error_type.is_empty() {
            err_body.error.error_type
        } else {
            status_text
        };
        (c, msg)
    } else {
        (status_text, body)
    };

    Error::Api(ApiError {
        status_code,
        code,
        message,
        request_id: request_id.to_string(),
    })
}

fn parse_api_error_from_text(status: reqwest::StatusCode, body: &str, request_id: &str) -> Error {
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

    let (code, message) = if let Ok(err_body) = serde_json::from_str::<ApiErrorBody>(body) {
        let msg = if err_body.error.message.is_empty() { body.to_string() } else { err_body.error.message };
        let c = if !err_body.error.code.is_empty() { err_body.error.code }
                else if !err_body.error.error_type.is_empty() { err_body.error.error_type }
                else { status_text };
        (c, msg)
    } else {
        (status_text, body.to_string())
    };

    Error::Api(ApiError { status_code, code, message, request_id: request_id.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_headers_rejected_at_build() {
        for name in ["Authorization", "authorization", "X-API-Key", "x-api-key"] {
            let result = ClientBuilder::new("qai_test")
                .extra_header(name, "anything")
                .build();
            match result {
                Err(Error::Api(api)) => assert_eq!(api.code, "invalid_header"),
                Ok(_) => panic!("expected reject for reserved header '{name}'"),
                Err(other) => panic!("unexpected error variant for '{name}': {other:?}"),
            }
        }
    }

    #[test]
    fn invalid_header_name_rejected_at_build() {
        let result = ClientBuilder::new("qai_test")
            .extra_header("bad name with spaces", "v")
            .build();
        match result {
            Err(Error::Api(api)) => assert_eq!(api.code, "invalid_header"),
            Ok(_) => panic!("expected reject for invalid header name"),
            Err(other) => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn app_and_extra_header_build_succeeds() {
        let _client = ClientBuilder::new("qai_test")
            .app("recipe-box")
            .extra_header("X-Correlation-Id", "abc-123")
            .build()
            .expect("valid builder should construct a Client");
    }
}
