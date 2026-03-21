use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{ApiError, ApiErrorBody, Error, Result};

/// The default Quantum AI API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.quantumencoding.ai";

/// The number of ticks in one US dollar (10 billion).
pub const TICKS_PER_USD: i64 = 10_000_000_000;

/// Common response metadata parsed from HTTP headers.
#[derive(Debug, Clone, Default)]
pub struct ResponseMeta {
    /// Cost in ticks from X-QAI-Cost-Ticks header.
    pub cost_ticks: i64,
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
}

impl ClientBuilder {
    /// Creates a new builder with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: Duration::from_secs(60),
        }
    }

    /// Sets the API base URL (default: `https://api.quantumencoding.ai`).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Sets the request timeout for non-streaming requests (default: 60s).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
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

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth_header.clone());
        // Also send X-API-Key for proxies that claim the Authorization header (e.g. Cloudflare -> Cloud Run IAM).
        if let Ok(v) = HeaderValue::from_str(&self.api_key) {
            headers.insert("X-API-Key", v);
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
            }),
        })
    }
}

struct ClientInner {
    base_url: String,
    http: reqwest::Client,
    auth_header: HeaderValue,
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

    /// Returns the auth header value (e.g. "Bearer qai_xxx").
    pub(crate) fn auth_header(&self) -> &HeaderValue {
        &self.inner.auth_header
    }

    /// Sends a JSON POST request and deserializes the response.
    pub async fn post_json<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let resp = self
            .inner
            .http
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        // Read body text first for better error messages on parse failure
        let body_text = resp.text().await?;
        let result: Resp = serde_json::from_str(&body_text).map_err(|e| {
            let preview = if body_text.len() > 300 { &body_text[..300] } else { &body_text };
            eprintln!("[sdk] JSON decode error on {path}: {e}\n  body preview: {preview}");
            e
        })?;
        Ok((result, meta))
    }

    /// Sends a GET request and deserializes the response.
    pub async fn get_json<Resp: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let resp = self.inner.http.get(&url).send().await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        let body_text = resp.text().await?;
        let result: Resp = serde_json::from_str(&body_text).map_err(|e| {
            let preview = if body_text.len() > 300 { &body_text[..300] } else { &body_text };
            eprintln!("[sdk] JSON decode error on {path}: {e}\n  body preview: {preview}");
            e
        })?;
        Ok((result, meta))
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

    /// Sends a multipart POST request and deserializes the response.
    pub async fn post_multipart<Resp: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<(Resp, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);
        let resp = self.inner.http.post(&url).multipart(form).send().await?;

        let meta = parse_response_meta(&resp);

        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &meta.request_id).await);
        }

        let result: Resp = resp.json().await?;
        Ok((result, meta))
    }

    /// Sends a JSON POST request expecting an SSE stream response.
    /// Returns the raw reqwest::Response for the caller to read events from.
    /// Uses a separate client without timeout -- cancellation is via drop.
    pub async fn post_stream_raw(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<(reqwest::Response, ResponseMeta)> {
        let url = format!("{}{}", self.inner.base_url, path);

        // Build a client without timeout for streaming.
        let stream_client = reqwest::Client::builder().build()?;

        let resp = stream_client
            .post(&url)
            .header(AUTHORIZATION, self.inner.auth_header.clone())
            .header(CONTENT_TYPE, "application/json")
            .header("Accept", "text/event-stream")
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

    ResponseMeta {
        cost_ticks,
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
