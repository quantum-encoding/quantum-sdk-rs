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

/// Strongly-typed view of the API's stable error-code taxonomy.
/// Match on this rather than on `ApiError::message`: the message is
/// human-readable and may change between releases, while the code is
/// part of the wire contract and is never repurposed.
///
/// `Unknown` covers a code this SDK version does not recognise (one
/// added after the SDK was built) and a response with no code field
/// at all. In both cases the raw string is preserved on
/// `ApiError::code` so callers can still match on it.
///
/// Each variant is the wire code in CamelCase, so
/// `KEY_FROZEN_BY_BUDGET` is `KeyFrozenByBudget`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorCode {
    // Auth / identity
    AuthHeaderMissing,
    AuthHeaderEmpty,
    KeyBearerMalformed,
    KeyNotFound,
    KeyExpired,
    KeyRevokedByAdmin,
    KeyRevokedByOwner,
    /// The partner's budget kill-switch fired. Unlike a self-revoke or
    /// admin-revoke the user's account is fine; the partner's billing
    /// is not, and the remedy is for the partner to top up.
    KeyFrozenByBudget,
    KeyPartnerRejected,
    SessionExpired,
    EphemeralExpired,

    // Authz / scope
    ScopeEndpointDenied,
    AdminRequired,
    ServiceAccountRequired,

    // Billing / credits
    InsufficientBalance,
    TrialExpired,
    SubscriptionLapsed,
    SpendCapExceeded,
    /// Runtime variant of partner budget freeze — fired mid-request
    /// vs. KeyFrozenByBudget which fires at auth time.
    BudgetFrozen,
    PaymentNotConfigured,
    BillingPortalNoHistory,

    // Rate / quota
    RateLimitedPerKey,
    RateLimitedPerIP,
    QuotaExceeded,

    // Provider / upstream
    ProviderRateLimited,
    ProviderUnavailable,
    ProviderAuthFailed,
    ProviderInvalidRequest,
    /// Moderation block on the request content, not on account state:
    /// the user can retry with different content.
    ContentRejected,
    ModelNotAvailable,

    // Request shape / validation
    InvalidRequestBody,
    MissingRequiredField,
    FieldTooLong,
    InvalidAttachment,
    AttachmentTooLarge,
    UnsupportedCapability,

    // System
    InternalError,
    ServiceUnavailable,
    StripeApiError,
    IdempotencyConflict,

    // Per-product paywall codes
    RecipeBoxPaywall,

    /// Unrecognised or absent code; the raw string is on
    /// `ApiError::code`.
    Unknown,
}

impl ErrorCode {
    /// Parse the wire code string into a typed variant. Unknown
    /// strings (including empty) yield `ErrorCode::Unknown`. Match
    /// is case-sensitive — the backend guarantees uppercase
    /// snake_case for canonical codes.
    pub fn from_wire(code: &str) -> Self {
        match code {
            "AUTH_HEADER_MISSING" => Self::AuthHeaderMissing,
            "AUTH_HEADER_EMPTY" => Self::AuthHeaderEmpty,
            "KEY_BEARER_MALFORMED" => Self::KeyBearerMalformed,
            "KEY_NOT_FOUND" => Self::KeyNotFound,
            "KEY_EXPIRED" => Self::KeyExpired,
            "KEY_REVOKED_BY_ADMIN" => Self::KeyRevokedByAdmin,
            "KEY_REVOKED_BY_OWNER" => Self::KeyRevokedByOwner,
            "KEY_FROZEN_BY_BUDGET" => Self::KeyFrozenByBudget,
            "KEY_PARTNER_REJECTED" => Self::KeyPartnerRejected,
            "SESSION_EXPIRED" => Self::SessionExpired,
            "EPHEMERAL_EXPIRED" => Self::EphemeralExpired,
            "SCOPE_ENDPOINT_DENIED" => Self::ScopeEndpointDenied,
            "ADMIN_REQUIRED" => Self::AdminRequired,
            "SERVICE_ACCOUNT_REQUIRED" => Self::ServiceAccountRequired,
            "INSUFFICIENT_BALANCE" => Self::InsufficientBalance,
            "TRIAL_EXPIRED" => Self::TrialExpired,
            "SUBSCRIPTION_LAPSED" => Self::SubscriptionLapsed,
            "SPEND_CAP_EXCEEDED" => Self::SpendCapExceeded,
            "BUDGET_FROZEN" => Self::BudgetFrozen,
            "PAYMENT_NOT_CONFIGURED" => Self::PaymentNotConfigured,
            "BILLING_PORTAL_NO_HISTORY" => Self::BillingPortalNoHistory,
            "RATE_LIMITED_PER_KEY" => Self::RateLimitedPerKey,
            "RATE_LIMITED_PER_IP" => Self::RateLimitedPerIP,
            "QUOTA_EXCEEDED" => Self::QuotaExceeded,
            "PROVIDER_RATE_LIMITED" => Self::ProviderRateLimited,
            "PROVIDER_UNAVAILABLE" => Self::ProviderUnavailable,
            "PROVIDER_AUTH_FAILED" => Self::ProviderAuthFailed,
            "PROVIDER_INVALID_REQUEST" => Self::ProviderInvalidRequest,
            "CONTENT_REJECTED" => Self::ContentRejected,
            "MODEL_NOT_AVAILABLE" => Self::ModelNotAvailable,
            "INVALID_REQUEST_BODY" => Self::InvalidRequestBody,
            "MISSING_REQUIRED_FIELD" => Self::MissingRequiredField,
            "FIELD_TOO_LONG" => Self::FieldTooLong,
            "INVALID_ATTACHMENT" => Self::InvalidAttachment,
            "ATTACHMENT_TOO_LARGE" => Self::AttachmentTooLarge,
            "UNSUPPORTED_CAPABILITY" => Self::UnsupportedCapability,
            "INTERNAL_ERROR" => Self::InternalError,
            "SERVICE_UNAVAILABLE" => Self::ServiceUnavailable,
            "STRIPE_API_ERROR" => Self::StripeApiError,
            "IDEMPOTENCY_CONFLICT" => Self::IdempotencyConflict,
            "RECIPE_BOX_PAYWALL" => Self::RecipeBoxPaywall,
            _ => Self::Unknown,
        }
    }
}

impl ApiError {
    /// Returns the strongly-typed error code. Convenience wrapper
    /// over `ErrorCode::from_wire(&self.code)`.
    pub fn typed_code(&self) -> ErrorCode {
        ErrorCode::from_wire(&self.code)
    }
}
