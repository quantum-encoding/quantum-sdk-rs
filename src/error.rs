use serde::Deserialize;
use std::fmt;

/// Result type alias for Quantum AI SDK operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types returned by the Quantum AI SDK.
#[derive(Debug)]
pub enum Error {
    /// The API returned a non-2xx status code.
    Api(ApiError),
    /// An HTTP transport error occurred.
    Http(reqwest::Error),
    /// A serialization or deserialization error occurred.
    Json(serde_json::Error),
    /// A WebSocket error occurred (realtime sessions).
    WebSocket(tokio_tungstenite::tungstenite::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Api(e) => write!(f, "{e}"),
            Error::Http(e) => write!(f, "qai: http error: {e}"),
            Error::Json(e) => write!(f, "qai: json error: {e}"),
            Error::WebSocket(e) => write!(f, "qai: websocket error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Api(_) => None,
            Error::Http(e) => Some(e),
            Error::Json(e) => Some(e),
            Error::WebSocket(e) => Some(e),
        }
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

/// An error returned by the Quantum AI API (non-2xx response).
#[derive(Debug, Clone)]
pub struct ApiError {
    /// The HTTP status code from the response.
    pub status_code: u16,
    /// The error type from the API (e.g. "invalid_request", "rate_limit").
    pub code: String,
    /// The human-readable error description.
    pub message: String,
    /// The unique request identifier from the X-QAI-Request-Id header.
    pub request_id: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.request_id.is_empty() {
            write!(
                f,
                "qai: {} {}: {}",
                self.status_code, self.code, self.message
            )
        } else {
            write!(
                f,
                "qai: {} {}: {} (request_id={})",
                self.status_code, self.code, self.message, self.request_id
            )
        }
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    /// Returns true if this is a 429 rate limit response.
    pub fn is_rate_limit(&self) -> bool {
        self.status_code == 429
    }

    /// Returns true if this is a 401 or 403 authentication/authorization failure.
    pub fn is_auth(&self) -> bool {
        self.status_code == 401 || self.status_code == 403
    }

    /// Returns true if this is a 404 not found response.
    pub fn is_not_found(&self) -> bool {
        self.status_code == 404
    }
}

/// Checks whether an error is a rate limit APIError.
pub fn is_rate_limit_error(err: &Error) -> bool {
    matches!(err, Error::Api(e) if e.is_rate_limit())
}

/// Checks whether an error is an authentication APIError.
pub fn is_auth_error(err: &Error) -> bool {
    matches!(err, Error::Api(e) if e.is_auth())
}

/// Checks whether an error is a not found APIError.
pub fn is_not_found_error(err: &Error) -> bool {
    matches!(err, Error::Api(e) if e.is_not_found())
}

/// Raw API error body envelope for JSON parsing.
#[derive(Deserialize)]
pub(crate) struct ApiErrorBody {
    pub error: ApiErrorInner,
}

#[derive(Deserialize)]
pub(crate) struct ApiErrorInner {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub code: String,
    #[serde(rename = "type", default)]
    pub error_type: String,
}
