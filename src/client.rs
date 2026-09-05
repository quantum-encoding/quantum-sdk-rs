use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::error::{ApiError, ApiErrorBody, Error, Result};
use crate::region::Region;

/// How many times one request may be replayed after its first attempt.
const MAX_RETRIES: u32 = 3;
/// Backoff before the first replay; doubles on each further replay.
const INITIAL_BACKOFF_MS: u64 = 500;
/// Longest `Retry-After` the SDK honours. A larger value is clamped so a
/// misbehaving server cannot park a caller indefinitely.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);

/// Which responses a request may be replayed on.
///
/// The gateway bills chat, session chat and every media route through a
/// reserve→settle rail that never reads `Idempotency-Key`, and key-minting
/// and Stripe checkout routes have no dedupe at all. Replaying such a POST
/// after a 502/503/504 that masked a completed operation runs it — and
/// charges for it — again. So a POST is replayed on 429 only, unless the
/// caller opted in with a key on a route that honours it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Replay {
    /// Replay on 429 only. A 429 is answered before any provider call or
    /// charge, so replaying it can never duplicate work.
    RateLimitOnly,
    /// Replay on 429, 502, 503 and 504.
    Transient,
}

fn is_retryable(status: reqwest::StatusCode, replay: Replay) -> bool {
    match replay {
        Replay::RateLimitOnly => status.as_u16() == 429,
        Replay::Transient => matches!(status.as_u16(), 429 | 502 | 503 | 504),
    }
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

/// The `Retry-After` delay a response asks for, when it carries one in
/// the delay-seconds form the gateway uses (it sends `5` per credential
/// and `10` per IP). The HTTP-date form is not parsed. Clamped to
/// [`MAX_RETRY_AFTER`].
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let secs: u64 = headers
        .get("Retry-After")?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(secs).min(MAX_RETRY_AFTER))
}

/// Delay before replay number `attempt` (1-based) when the response
/// carried no usable `Retry-After`.
fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(INITIAL_BACKOFF_MS * 2u64.pow(attempt.saturating_sub(1)))
}

/// The default Quantum AI API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.quantumencoding.ai";

/// The number of ticks in one US dollar (10 billion).
pub const TICKS_PER_USD: i64 = 10_000_000_000;

/// Common response metadata parsed from HTTP headers.
#[derive(Debug, Clone, Default)]
pub struct ResponseMeta {
    /// Cost in ticks from the `X-QAI-Cost-Ticks` header. Zero when the
    /// route sends no cost header — a semantic-cache hit on chat, or a
    /// route that does not bill.
    pub cost_ticks: i64,
    /// Post-deduction credit balance in ticks from the
    /// `X-QAI-Balance-After` header. Only the media routes (image, video,
    /// audio, avatar) send it; on chat, session chat, search, keys,
    /// credits and account calls this is always zero. Use
    /// `credit_balance` / `account_balance` for the balance after a chat.
    pub balance_after: i64,
    /// Request identifier from the `X-QAI-Request-Id` header, set on every
    /// response.
    pub request_id: String,
    /// Model identifier from the `X-QAI-Model` header (chat routes).
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

fn invalid_api_key_error() -> Error {
    Error::Api(ApiError {
        status_code: 0,
        code: "invalid_api_key".to_string(),
        message: "API key contains characters not allowed in an HTTP header \
                  (a trailing newline read from a file is the usual cause)"
            .to_string(),
        request_id: String::new(),
    })
}

impl ClientBuilder {
    /// Creates a new builder with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            // Buffered media generation (image/video) returns a single JSON blob
            // only when the provider finishes; 600s outlasts the backend's
            // 5-minute media deadline so the server errors first. Streaming uses
            // a separate no-timeout client.
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

    /// Sets the request timeout for non-streaming requests (default: 600s).
    ///
    /// The default outlasts the backend's 5-minute media deadline, so buffered
    /// media generation (video, dubbing, 3D) fails server-side rather than
    /// here. Lower it for latency-sensitive callers, or use the async jobs API
    /// (`create_job` / `poll_job`) which doesn't block.
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

    /// Routes this client's chat calls through a region
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
    pub fn extra_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.push((name.into(), value.into()));
        self
    }

    /// Builds the [`Client`].
    ///
    /// Fails with an `invalid_api_key` error when the key cannot be sent as
    /// a header value, and with `invalid_header` when a caller-supplied
    /// header is reserved or malformed.
    pub fn build(self) -> Result<Client> {
        let auth_header = HeaderValue::from_str(&format!("Bearer {}", self.api_key))
            .map_err(|_| invalid_api_key_error())?;
        let key_header =
            HeaderValue::from_str(&self.api_key).map_err(|_| invalid_api_key_error())?;

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
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|e| invalid_header_error(format!("invalid header name '{name}': {e}")))?;
            let header_value = HeaderValue::from_str(value).map_err(|e| {
                invalid_header_error(format!("invalid header value for '{name}': {e}"))
            })?;
            extra_headers_map.insert(header_name, header_value);
        }

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth_header.clone());
        // X-API-Key duplicates the credential for proxies that consume the
        // Authorization header before it reaches the gateway. The gateway
        // reads X-API-Key first and falls back to the bearer.
        headers.insert("X-API-Key", key_header);
        // Caller-supplied headers are inserted *after* auth so the reserved
        // guard above is the only way to override standard SDK headers.
        for (name, value) in &extra_headers_map {
            headers.insert(name.clone(), value.clone());
        }

        let http = reqwest::Client::builder()
            .default_headers(headers.clone())
            .timeout(self.timeout)
            .build()?;
        // Same credential and caller headers, no timeout: an SSE stream is
        // open for as long as the model talks, and cancellation is drop.
        let stream_http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Client {
            inner: Arc::new(ClientInner {
                base_url: self.base_url,
                http,
                stream_http,
                auth_header,
                region: self.region,
            }),
        })
    }
}

struct ClientInner {
    base_url: String,
    /// Non-streaming client: credential headers, caller headers, timeout.
    http: reqwest::Client,
    /// Streaming client: the same default headers, no timeout. Built once
    /// so streams share a connection pool instead of a fresh TLS handshake
    /// per call.
    stream_http: reqwest::Client,
    auth_header: HeaderValue,
    /// Client-level routing region applied to chat requests (see
    /// [`ClientBuilder::region`]).
    region: Option<Region>,
}

/// The Quantum AI API client.
///
/// `Client` is cheaply cloneable (backed by `Arc`) and safe to share across tasks.
///
/// # Example
///
/// ```no_run
/// # fn example() -> quantum_sdk::Result<()> {
/// let client = quantum_sdk::Client::new("qai_key_xxx")?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl Client {
    /// Creates a new client with the given API key and default settings.
    ///
    /// Fails with an `invalid_api_key` error when the key cannot be sent
    /// as an HTTP header value — a trailing newline read from a file is
    /// the usual cause. Equivalent to `Client::builder(key).build()`.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        ClientBuilder::new(api_key).build()
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

    /// The shared HTTP client: carries the credential headers, the caller's
    /// extra headers and the configured timeout. Modules that need a verb
    /// the typed helpers do not offer (a DELETE answering 204, say) build on
    /// this rather than a fresh `reqwest::Client`.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }

    /// Sends a request, replaying it per `replay`, and returns the first
    /// 2xx response. A non-2xx that is not replayable — or the last
    /// replay's failure — comes back as [`Error::Api`].
    ///
    /// `build` produces a fresh request for every attempt.
    async fn send_retrying<F>(
        &self,
        build: F,
        replay: Replay,
    ) -> Result<(reqwest::Response, ResponseMeta)>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let mut attempt = 0;
        loop {
            let resp = build().send().await?;
            let status = resp.status();
            let meta = parse_response_meta(&resp);

            if status.is_success() {
                return Ok((resp, meta));
            }
            if !is_retryable(status, replay) || attempt >= MAX_RETRIES {
                return Err(parse_api_error(resp, &meta.request_id).await);
            }

            let asked = retry_after(resp.headers());
            let body_text = resp.text().await.unwrap_or_default();
            if is_permanent_error(&body_text) {
                return Err(parse_api_error_from_text(
                    status,
                    &body_text,
                    &meta.request_id,
                ));
            }

            attempt += 1;
            tokio::time::sleep(asked.unwrap_or_else(|| backoff(attempt))).await;
        }
    }

    /// Sends a JSON POST request and deserializes the response.
    ///
    /// # Retries
    ///
    /// The request is replayed only on 429, waiting for the response's
    /// `Retry-After` when it carries one (the gateway sends 5 s per
    /// credential, 10 s per IP) and 0.5 s / 1 s / 2 s otherwise, up to
    /// three times. A 502, 503 or 504 is returned as an error, never
    /// replayed: the gateway bills chat, session chat and every media
    /// route through a reserve→settle rail that does not read
    /// `Idempotency-Key`, and key-minting and Stripe checkout routes have
    /// no dedupe at all, so a replay after a 5xx that masked a completed
    /// operation would run — and charge for — it a second time. To opt
    /// into 5xx replay on a route that does dedupe, see
    /// [`post_json_with_idempotency`](Self::post_json_with_idempotency).
    ///
    /// A random `Idempotency-Key` is sent and reused across the 429
    /// replays so routes that dedupe see one logical request.
    pub async fn post_json<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<(Resp, ResponseMeta)> {
        self.post_json_with_idempotency(path, body, None).await
    }

    /// Like [`post_json`](Self::post_json) but with a caller-supplied
    /// idempotency key, which also opts the request into replay on
    /// 502/503/504.
    ///
    /// # What the gateway does with the key
    ///
    /// Only routes billed through the gateway's `DeductAndTrack` rail read
    /// `Idempotency-Key`: agent, batch, jobs, search, scanner, rag,
    /// documents, vision, voice, compute and deployments, inference,
    /// missions, cloudrun and security. On those the billing result is
    /// cached for 24 hours under (key, account) — the request body is not
    /// part of the cache key, so a key reused for a *different* payload
    /// returns the first request's billing result while the provider still
    /// runs. Use one key per logical request, never per worker.
    ///
    /// The key is ignored on `/chat`, `/chat/session`, `/chat/estimate`,
    /// every image, video, audio and avatar route, and on keys, credits,
    /// auth and account. Pass `Some` on those only if a duplicate charge
    /// (or a duplicate key / checkout session) after a masked success is
    /// acceptable to you. With `None` the behaviour is exactly
    /// [`post_json`](Self::post_json): 429 only.
    pub async fn post_json_with_idempotency<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
        idempotency_key: Option<String>,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let body_bytes = serde_json::to_vec(body)?;
        let replay = if idempotency_key.is_some() {
            Replay::Transient
        } else {
            Replay::RateLimitOnly
        };
        let key = idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string());

        let (resp, meta) = self
            .send_retrying(
                || {
                    self.inner
                        .http
                        .post(&url)
                        .header(CONTENT_TYPE, "application/json")
                        .header("Idempotency-Key", &key)
                        .body(body_bytes.clone())
                },
                replay,
            )
            .await?;
        Ok((decode_json(resp).await?, meta))
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
    ///
    /// A GET is replayed on 429 (honouring `Retry-After`), 502, 503 and
    /// 504, up to three times: reads bill nothing, so a replay cannot
    /// duplicate a charge.
    pub async fn get_json<Resp: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let (resp, meta) = self
            .send_retrying(|| self.inner.http.get(&url), Replay::Transient)
            .await?;
        Ok((decode_json(resp).await?, meta))
    }

    /// Sends a DELETE request and deserializes the response. Single
    /// attempt.
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

        Ok((decode_json(resp).await?, meta))
    }

    /// Sends a POST request with an empty body and deserializes the
    /// response. Same retry policy as [`post_json`](Self::post_json).
    pub async fn post_json_empty<Resp: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<(Resp, ResponseMeta)> {
        self.post_json_empty_with_idempotency(path, None).await
    }

    /// Like [`post_json_empty`](Self::post_json_empty) but with a
    /// caller-supplied idempotency key. See
    /// [`post_json_with_idempotency`](Self::post_json_with_idempotency)
    /// for which routes honour it and what opting in means.
    pub async fn post_json_empty_with_idempotency<Resp: DeserializeOwned>(
        &self,
        path: &str,
        idempotency_key: Option<String>,
    ) -> Result<(Resp, ResponseMeta)> {
        self.post_json_with_idempotency(path, &serde_json::json!({}), idempotency_key)
            .await
    }

    /// Sends a PUT request with a JSON body and deserializes the response.
    /// Single attempt.
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

        Ok((decode_json(resp).await?, meta))
    }

    /// Sends a multipart POST request and deserializes the response.
    /// Single attempt: a multipart body cannot be replayed.
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
    /// for which routes honour it. The request is still single attempt;
    /// the key lets a route that dedupes recognise a re-issued upload.
    pub async fn post_multipart_with_idempotency<Resp: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
        idempotency_key: Option<String>,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let key = idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string());
        let resp = self
            .inner
            .http
            .post(&url)
            .header("Idempotency-Key", key)
            .multipart(form)
            .send()
            .await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        Ok((decode_json(resp).await?, meta))
    }

    /// Sends a GET request expecting an SSE stream response.
    /// Returns the raw reqwest::Response for the caller to read events from.
    /// Uses the shared no-timeout streaming client, which carries the same
    /// credential and caller headers as every other request; cancellation
    /// is via drop. Single attempt.
    pub async fn get_stream_raw(&self, path: &str) -> Result<(reqwest::Response, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let resp = self
            .inner
            .stream_http
            .get(&url)
            .header("Accept", "text/event-stream")
            .send()
            .await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        Ok((resp, meta))
    }

    /// Sends a JSON POST request expecting an SSE stream response.
    /// Returns the raw reqwest::Response for the caller to read events from.
    /// Uses the shared no-timeout streaming client, which carries the same
    /// credential and caller headers as every other request; cancellation
    /// is via drop. Single attempt.
    pub async fn post_stream_raw(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<(reqwest::Response, ResponseMeta)> {
        self.post_stream_raw_with_idempotency(path, body, None)
            .await
    }

    /// Like [`post_stream_raw`](Self::post_stream_raw) but with a
    /// caller-supplied idempotency key. Streaming requests are single
    /// attempt; the key only matters on the routes listed under
    /// [`post_json_with_idempotency`](Self::post_json_with_idempotency),
    /// and the streaming chat routes are not among them.
    pub async fn post_stream_raw_with_idempotency(
        &self,
        path: &str,
        body: &impl Serialize,
        idempotency_key: Option<String>,
    ) -> Result<(reqwest::Response, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let key = idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string());
        let resp = self
            .inner
            .stream_http
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .header("Accept", "text/event-stream")
            .header("Idempotency-Key", key)
            .json(body)
            .send()
            .await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        Ok((resp, meta))
    }
}

/// Decodes a 2xx body as JSON. A decode failure is [`Error::Json`] and
/// carries only serde's position, never the body: sign-in and key-minting
/// responses open with live credentials, and library code must not copy
/// those into an error message or a log.
async fn decode_json<Resp: DeserializeOwned>(resp: reqwest::Response) -> Result<Resp> {
    let bytes = resp.bytes().await?;
    Ok(serde_json::from_slice(&bytes)?)
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
pub(crate) async fn parse_api_error(resp: reqwest::Response, request_id: &str) -> Error {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    parse_api_error_from_text(status, &body, request_id)
}

/// The flat error envelope: `error` is the code itself.
#[derive(serde::Deserialize)]
struct FlatApiErrorBody {
    error: String,
    #[serde(default)]
    message: String,
}

fn parse_api_error_from_text(status: reqwest::StatusCode, body: &str, request_id: &str) -> Error {
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

    // Two envelopes exist: the usual `{"error": {message, type, code}}` and
    // the flat `{"error": "<code>", "message": "…"}` a few routes
    // (/qai/v1/agent among them) write.
    let (code, message) = if let Ok(err_body) = serde_json::from_str::<ApiErrorBody>(body) {
        let msg = if err_body.error.message.is_empty() {
            body.to_string()
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
    } else if let Ok(flat) = serde_json::from_str::<FlatApiErrorBody>(body) {
        let msg = if flat.message.is_empty() {
            body.to_string()
        } else {
            flat.message
        };
        let c = if flat.error.is_empty() {
            status_text
        } else {
            flat.error
        };
        (c, msg)
    } else {
        (status_text, body.to_string())
    };

    Error::Api(ApiError {
        status_code,
        code,
        message,
        request_id: request_id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn flat_error_envelope_yields_its_code() {
        let err = parse_api_error_from_text(
            reqwest::StatusCode::BAD_REQUEST,
            r#"{"error":"invalid_request","message":"model is required"}"#,
            "req_1",
        );
        match err {
            Error::Api(e) => {
                assert_eq!(e.code, "invalid_request");
                assert_eq!(e.message, "model is required");
                assert_eq!(e.status_code, 400);
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }
    use super::*;

    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::time::Instant;

    /// One request as the mock saw it. Header names are lowercased.
    struct Recorded {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
    }

    impl Recorded {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
        }
    }

    struct Canned {
        status: u16,
        reason: &'static str,
        headers: Vec<(&'static str, String)>,
        body: String,
    }

    fn canned(status: u16, reason: &'static str, body: &str) -> Canned {
        Canned {
            status,
            reason,
            headers: Vec::new(),
            body: body.to_string(),
        }
    }

    /// A one-thread HTTP/1.1 server that answers each connection with the
    /// next scripted response and records what it was asked. Every
    /// response closes the connection so a replay is a new accept.
    struct MockServer {
        base_url: String,
        requests: Arc<Mutex<Vec<Recorded>>>,
    }

    impl MockServer {
        fn start(script: Vec<Canned>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded = Arc::clone(&requests);
            std::thread::spawn(move || {
                for reply in script {
                    let Ok((mut stream, _)) = listener.accept() else {
                        return;
                    };
                    let req = read_request(&mut stream);
                    recorded.lock().unwrap().push(req);
                    let mut head = format!(
                        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
                        reply.status,
                        reply.reason,
                        reply.body.len()
                    );
                    for (name, value) in &reply.headers {
                        head.push_str(&format!("{name}: {value}\r\n"));
                    }
                    head.push_str("\r\n");
                    let _ = stream.write_all(head.as_bytes());
                    let _ = stream.write_all(reply.body.as_bytes());
                    let _ = stream.flush();
                }
            });
            Self { base_url, requests }
        }

        fn client(&self) -> Client {
            Client::builder("qai_k_test")
                .base_url(&self.base_url)
                .app("recipe-box")
                .extra_header("X-Correlation-Id", "abc-123")
                .build()
                .unwrap()
        }

        fn requests(&self) -> Vec<Recorded> {
            std::mem::take(&mut *self.requests.lock().unwrap())
        }
    }

    fn read_request(stream: &mut std::net::TcpStream) -> Recorded {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let head_end = loop {
            let n = stream.read(&mut chunk).expect("read request");
            if n == 0 {
                panic!("connection closed before the request head ended");
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos + 4;
            }
        };
        let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
        let mut lines = head.lines();
        let request_line = lines.next().unwrap_or_default();
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_string();
        let path = parts.next().unwrap_or_default().to_string();
        let headers: Vec<(String, String)> = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        let content_length: usize = headers
            .iter()
            .find(|(n, _)| n == "content-length")
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(0);
        let mut body = buf[head_end..].to_vec();
        while body.len() < content_length {
            let n = stream.read(&mut chunk).expect("read body");
            if n == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..n]);
        }
        Recorded {
            method,
            path,
            headers,
        }
    }

    #[derive(serde::Deserialize, Debug)]
    struct OkBody {
        ok: bool,
    }

    fn api_status(err: &Error) -> u16 {
        match err {
            Error::Api(e) => e.status_code,
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_post_answered_502_is_not_replayed() {
        let server = MockServer::start(vec![canned(
            502,
            "Bad Gateway",
            r#"{"error":{"message":"upstream","type":"provider_error"}}"#,
        )]);
        let err = server
            .client()
            .post_json::<_, OkBody>("/qai/v1/chat", &serde_json::json!({"model": "m"}))
            .await
            .unwrap_err();
        assert_eq!(api_status(&err), 502);
        let seen = server.requests();
        assert_eq!(
            seen.len(),
            1,
            "a 502 on a POST must reach the caller unreplayed"
        );
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].path, "/qai/v1/chat");
    }

    #[tokio::test]
    async fn a_post_answered_429_waits_for_retry_after_then_replays() {
        let mut limited = canned(
            429,
            "Too Many Requests",
            r#"{"error":{"message":"slow down","type":"rate_limit_exceeded","code":"RATE_LIMITED_PER_KEY"}}"#,
        );
        limited.headers.push(("Retry-After", "1".to_string()));
        let server = MockServer::start(vec![limited, canned(200, "OK", r#"{"ok":true}"#)]);

        let started = Instant::now();
        let (resp, _meta) = server
            .client()
            .post_json::<_, OkBody>("/qai/v1/chat", &serde_json::json!({}))
            .await
            .unwrap();
        assert!(resp.ok);
        assert!(
            started.elapsed() >= Duration::from_secs(1),
            "must wait the server's Retry-After before replaying"
        );
        let seen = server.requests();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            seen[0].header("idempotency-key"),
            seen[1].header("idempotency-key"),
            "the replay carries the same Idempotency-Key"
        );
    }

    #[tokio::test]
    async fn an_explicit_idempotency_key_opts_a_post_into_5xx_replay() {
        let server = MockServer::start(vec![
            canned(503, "Service Unavailable", "busy"),
            canned(200, "OK", r#"{"ok":true}"#),
        ]);
        let (resp, _meta) = server
            .client()
            .post_json_with_idempotency::<_, OkBody>(
                "/qai/v1/search",
                &serde_json::json!({}),
                Some("job-42".to_string()),
            )
            .await
            .unwrap();
        assert!(resp.ok);
        let seen = server.requests();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].header("idempotency-key"), Some("job-42"));
        assert_eq!(seen[1].header("idempotency-key"), Some("job-42"));
    }

    #[tokio::test]
    async fn a_get_answered_503_is_replayed() {
        let server = MockServer::start(vec![
            canned(503, "Service Unavailable", "busy"),
            canned(200, "OK", r#"{"ok":true}"#),
        ]);
        let (resp, _meta) = server
            .client()
            .get_json::<OkBody>("/qai/v1/models")
            .await
            .unwrap();
        assert!(resp.ok);
        assert_eq!(server.requests().len(), 2);
    }

    #[tokio::test]
    async fn a_5xx_wrapping_a_permanent_error_is_not_replayed_on_get() {
        let server = MockServer::start(vec![canned(
            502,
            "Bad Gateway",
            r#"{"error":{"message":"content moderation blocked this","type":"provider_error"}}"#,
        )]);
        let err = server
            .client()
            .get_json::<OkBody>("/qai/v1/models")
            .await
            .unwrap_err();
        assert_eq!(api_status(&err), 502);
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn streaming_sends_the_same_headers_as_every_other_request() {
        let mut sse = canned(200, "OK", "data: [DONE]\n\n");
        sse.headers
            .push(("Content-Type", "text/event-stream".to_string()));
        let server = MockServer::start(vec![sse]);
        let (_resp, _meta) = server
            .client()
            .post_stream_raw("/qai/v1/chat", &serde_json::json!({"stream": true}))
            .await
            .unwrap();
        let seen = server.requests();
        let req = &seen[0];
        assert_eq!(req.header("authorization"), Some("Bearer qai_k_test"));
        assert_eq!(req.header("x-api-key"), Some("qai_k_test"));
        assert_eq!(req.header("x-quantum-app"), Some("recipe-box"));
        assert_eq!(req.header("x-correlation-id"), Some("abc-123"));
        assert_eq!(req.header("accept"), Some("text/event-stream"));
    }

    #[tokio::test]
    async fn a_decode_failure_never_carries_the_body() {
        let server = MockServer::start(vec![canned(
            200,
            "OK",
            r#"{"token":"qai_s_LIVE_SESSION","user":42}"#,
        )]);
        let err = server
            .client()
            .post_json::<_, OkBody>("/qai/v1/auth/google", &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Json(_)), "got {err:?}");
        let shown = format!("{err} / {err:?}");
        assert!(
            !shown.contains("LIVE_SESSION"),
            "a decode error must not echo the response body: {shown}"
        );
    }

    #[test]
    fn retry_after_reads_delay_seconds_and_clamps() {
        let mut h = HeaderMap::new();
        assert_eq!(retry_after(&h), None);
        h.insert("Retry-After", HeaderValue::from_static("5"));
        assert_eq!(retry_after(&h), Some(Duration::from_secs(5)));
        h.insert("Retry-After", HeaderValue::from_static("86400"));
        assert_eq!(retry_after(&h), Some(MAX_RETRY_AFTER));
        h.insert(
            "Retry-After",
            HeaderValue::from_static("Wed, 21 Oct 2026 07:28:00 GMT"),
        );
        assert_eq!(retry_after(&h), None, "the HTTP-date form is not parsed");
    }

    #[test]
    fn a_bad_key_is_an_error_not_a_panic() {
        match Client::new("qai_k_from_a_file\n") {
            Err(Error::Api(api)) => assert_eq!(api.code, "invalid_api_key"),
            Ok(_) => panic!("a key with a newline cannot be a header value"),
            Err(other) => panic!("unexpected error variant: {other:?}"),
        }
    }

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
