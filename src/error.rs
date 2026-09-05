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
    /// A WebSocket error occurred (realtime sessions). Boxed: the
    /// tungstenite error carries a full HTTP response and would otherwise
    /// make every `Result` in the crate that size.
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),
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
        Error::WebSocket(Box::new(err))
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

/// Strongly-typed view of the API's error codes.
/// Match on this rather than on `ApiError::message`: the message is
/// human-readable and may change between releases, while the code is
/// part of the wire contract and is never repurposed.
///
/// The gateway has two generations of code. Canonical codes are
/// uppercase snake_case (`KEY_FROZEN_BY_BUDGET`) and each is its own
/// variant, named in CamelCase. Most 4xx responses, though, still come
/// from a legacy writer that copies a lowercase `type` into `code`
/// (`invalid_request`, `authentication_error`, `not_found`, `forbidden`,
/// `provider_error`, `invalid_state`, …); those are folded onto the
/// variant with the same meaning, and a family with no canonical
/// counterpart gets its own generic variant ([`AuthenticationError`],
/// [`InvalidState`], [`ProviderError`], [`RateLimited`]).
///
/// `Unknown` covers a code this SDK version does not recognise (one
/// added after the SDK was built) and a response with no code field
/// at all. In every case the raw string is preserved on
/// `ApiError::code` so callers can still match on it.
///
/// [`AuthenticationError`]: ErrorCode::AuthenticationError
/// [`InvalidState`]: ErrorCode::InvalidState
/// [`ProviderError`]: ErrorCode::ProviderError
/// [`RateLimited`]: ErrorCode::RateLimited
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorCode {
    // Auth / identity
    AuthHeaderMissing,
    AuthHeaderEmpty,
    KeyBearerMalformed,
    /// `KEY_NOT_FOUND`, and the legacy `invalid_key` from `verify_key`.
    KeyNotFound,
    KeyExpired,
    KeyRevokedByAdmin,
    KeyRevokedByOwner,
    /// `KEY_ROTATED`: the key was replaced by `rotate_key` and its grace
    /// period (if any) has elapsed.
    KeyRotated,
    /// The partner's budget kill-switch fired. Unlike a self-revoke or
    /// admin-revoke the user's account is fine; the partner's billing
    /// is not, and the remedy is for the partner to top up.
    KeyFrozenByBudget,
    KeyPartnerRejected,
    SessionExpired,
    EphemeralExpired,
    /// `ACCOUNT_DELETED`: the account behind the credential was deleted;
    /// the credential is dead for good.
    AccountDeleted,
    /// The legacy `authentication_error` / `unauthorized` / `auth_error`
    /// types: the credential was missing, rejected or is not allowed to
    /// do this, with no finer canonical code attached.
    AuthenticationError,

    // Authz / scope
    ScopeEndpointDenied,
    AdminRequired,
    ServiceAccountRequired,
    /// `APP_SCOPE_MISMATCH`: the key is scoped to a different app than
    /// the one `X-Quantum-App` names.
    AppScopeMismatch,
    /// `PERMISSION_DENIED`, and the legacy `forbidden` /
    /// `permission_error` types: the caller is authenticated but may not
    /// touch this resource.
    PermissionDenied,

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
    /// The legacy `rate_limited` / `rate_limit_exceeded` types with no
    /// per-key / per-IP code attached.
    RateLimited,

    // Provider / upstream
    ProviderRateLimited,
    ProviderUnavailable,
    ProviderAuthFailed,
    ProviderInvalidRequest,
    /// `PROVIDER_FEATURE_DISALLOWED`: a feature (structured outputs, say)
    /// is blocked by provider or org configuration, not by the request.
    /// Retrying will not help; the operator has to change the config.
    ProviderFeatureDisallowed,
    /// Moderation block on the request content, not on account state:
    /// the user can retry with different content.
    ContentRejected,
    ModelNotAvailable,
    /// The legacy `provider_error` / `upstream_error` types: the upstream
    /// provider failed and the gateway attached no finer code.
    ProviderError,

    // Request shape / validation
    /// `INVALID_REQUEST`, and the legacy `invalid_request` /
    /// `invalid_request_error` / `bad_request` / `missing_fields` types.
    InvalidRequest,
    InvalidRequestBody,
    MissingRequiredField,
    FieldTooLong,
    InvalidAttachment,
    AttachmentTooLarge,
    /// `FILE_MIME_UNSUPPORTED`: `/qai/v1/files` rejected the upload's
    /// MIME type.
    FileMimeUnsupported,
    UnsupportedCapability,
    /// The legacy `invalid_state` / `conflict` types: the resource is not
    /// in a state that allows the operation (rotating an already-rotated
    /// key, extending a deployment that is not ready).
    InvalidState,

    // System
    InternalError,
    ServiceUnavailable,
    StripeApiError,
    IdempotencyConflict,
    /// `NOT_FOUND`, and the legacy `not_found` / `scan_not_found` /
    /// `type_not_found` types.
    NotFound,

    // Per-product paywall codes
    RecipeBoxPaywall,

    /// Unrecognised or absent code; the raw string is on
    /// `ApiError::code`.
    Unknown,
}

impl ErrorCode {
    /// Parse the wire code string into a typed variant. Canonical
    /// uppercase codes map one-to-one; the legacy lowercase `type`
    /// strings the gateway copies into `code` on most 4xx responses
    /// are folded onto the variant with the same meaning. Unknown
    /// strings (including empty) yield `ErrorCode::Unknown`. Match is
    /// case-sensitive in both generations.
    pub fn from_wire(code: &str) -> Self {
        match code {
            "AUTH_HEADER_MISSING" => Self::AuthHeaderMissing,
            "AUTH_HEADER_EMPTY" => Self::AuthHeaderEmpty,
            "KEY_BEARER_MALFORMED" => Self::KeyBearerMalformed,
            "KEY_NOT_FOUND" | "invalid_key" => Self::KeyNotFound,
            "KEY_EXPIRED" => Self::KeyExpired,
            "KEY_REVOKED_BY_ADMIN" => Self::KeyRevokedByAdmin,
            "KEY_REVOKED_BY_OWNER" => Self::KeyRevokedByOwner,
            "KEY_ROTATED" => Self::KeyRotated,
            "KEY_FROZEN_BY_BUDGET" => Self::KeyFrozenByBudget,
            "KEY_PARTNER_REJECTED" => Self::KeyPartnerRejected,
            "SESSION_EXPIRED" => Self::SessionExpired,
            "EPHEMERAL_EXPIRED" => Self::EphemeralExpired,
            "ACCOUNT_DELETED" => Self::AccountDeleted,
            "authentication_error" | "unauthorized" | "auth_error" => Self::AuthenticationError,
            "SCOPE_ENDPOINT_DENIED" => Self::ScopeEndpointDenied,
            "ADMIN_REQUIRED" => Self::AdminRequired,
            "SERVICE_ACCOUNT_REQUIRED" => Self::ServiceAccountRequired,
            "APP_SCOPE_MISMATCH" => Self::AppScopeMismatch,
            "PERMISSION_DENIED" | "forbidden" | "permission_error" => Self::PermissionDenied,
            "INSUFFICIENT_BALANCE"
            | "insufficient_balance"
            | "insufficient_funds"
            | "balance_zero" => Self::InsufficientBalance,
            "TRIAL_EXPIRED" => Self::TrialExpired,
            "SUBSCRIPTION_LAPSED" => Self::SubscriptionLapsed,
            "SPEND_CAP_EXCEEDED" => Self::SpendCapExceeded,
            "BUDGET_FROZEN" => Self::BudgetFrozen,
            "PAYMENT_NOT_CONFIGURED" => Self::PaymentNotConfigured,
            "BILLING_PORTAL_NO_HISTORY" | "no_billing_history" => Self::BillingPortalNoHistory,
            "RATE_LIMITED_PER_KEY" => Self::RateLimitedPerKey,
            "RATE_LIMITED_PER_IP" => Self::RateLimitedPerIP,
            "QUOTA_EXCEEDED" => Self::QuotaExceeded,
            "rate_limited" | "rate_limit_exceeded" | "rate_limit" => Self::RateLimited,
            "PROVIDER_RATE_LIMITED" => Self::ProviderRateLimited,
            "PROVIDER_UNAVAILABLE" => Self::ProviderUnavailable,
            "PROVIDER_AUTH_FAILED" => Self::ProviderAuthFailed,
            "PROVIDER_INVALID_REQUEST" => Self::ProviderInvalidRequest,
            "PROVIDER_FEATURE_DISALLOWED" => Self::ProviderFeatureDisallowed,
            "CONTENT_REJECTED" => Self::ContentRejected,
            "MODEL_NOT_AVAILABLE" => Self::ModelNotAvailable,
            "provider_error" | "upstream_error" => Self::ProviderError,
            "INVALID_REQUEST"
            | "invalid_request"
            | "invalid_request_error"
            | "bad_request"
            | "missing_fields" => Self::InvalidRequest,
            "INVALID_REQUEST_BODY" => Self::InvalidRequestBody,
            "MISSING_REQUIRED_FIELD" => Self::MissingRequiredField,
            "FIELD_TOO_LONG" | "field_too_long" => Self::FieldTooLong,
            "INVALID_ATTACHMENT" | "invalid_attachment" => Self::InvalidAttachment,
            "ATTACHMENT_TOO_LARGE" | "attachment_too_large" => Self::AttachmentTooLarge,
            "FILE_MIME_UNSUPPORTED" => Self::FileMimeUnsupported,
            "UNSUPPORTED_CAPABILITY" | "capability_error" => Self::UnsupportedCapability,
            "invalid_state" | "conflict" => Self::InvalidState,
            "INTERNAL_ERROR" | "internal_error" => Self::InternalError,
            "SERVICE_UNAVAILABLE" | "service_unavailable" | "unavailable" => {
                Self::ServiceUnavailable
            }
            "STRIPE_API_ERROR" | "stripe_error" => Self::StripeApiError,
            "IDEMPOTENCY_CONFLICT" => Self::IdempotencyConflict,
            "NOT_FOUND" | "not_found" | "scan_not_found" | "type_not_found" => Self::NotFound,
            "RECIPE_BOX_PAYWALL" | "recipe_box_paywall" => Self::RecipeBoxPaywall,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_codes_map_to_their_own_variants() {
        assert_eq!(ErrorCode::from_wire("KEY_ROTATED"), ErrorCode::KeyRotated);
        assert_eq!(
            ErrorCode::from_wire("ACCOUNT_DELETED"),
            ErrorCode::AccountDeleted
        );
        assert_eq!(
            ErrorCode::from_wire("APP_SCOPE_MISMATCH"),
            ErrorCode::AppScopeMismatch
        );
        assert_eq!(
            ErrorCode::from_wire("FILE_MIME_UNSUPPORTED"),
            ErrorCode::FileMimeUnsupported
        );
        assert_eq!(ErrorCode::from_wire("NOT_FOUND"), ErrorCode::NotFound);
        assert_eq!(
            ErrorCode::from_wire("PERMISSION_DENIED"),
            ErrorCode::PermissionDenied
        );
        assert_eq!(
            ErrorCode::from_wire("INVALID_REQUEST"),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            ErrorCode::from_wire("PROVIDER_FEATURE_DISALLOWED"),
            ErrorCode::ProviderFeatureDisallowed
        );
    }

    #[test]
    fn legacy_lowercase_types_fold_onto_the_matching_variant() {
        // writeError copies the lowercase `type` into `code`; these are
        // the families the audited routes emit.
        assert_eq!(
            ErrorCode::from_wire("invalid_request"),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            ErrorCode::from_wire("authentication_error"),
            ErrorCode::AuthenticationError
        );
        assert_eq!(ErrorCode::from_wire("invalid_key"), ErrorCode::KeyNotFound);
        assert_eq!(
            ErrorCode::from_wire("invalid_state"),
            ErrorCode::InvalidState
        );
        assert_eq!(
            ErrorCode::from_wire("forbidden"),
            ErrorCode::PermissionDenied
        );
        assert_eq!(
            ErrorCode::from_wire("provider_error"),
            ErrorCode::ProviderError
        );
        assert_eq!(ErrorCode::from_wire("not_found"), ErrorCode::NotFound);
        assert_eq!(
            ErrorCode::from_wire("internal_error"),
            ErrorCode::InternalError
        );
    }

    #[test]
    fn unknown_and_empty_stay_unknown() {
        assert_eq!(ErrorCode::from_wire(""), ErrorCode::Unknown);
        assert_eq!(ErrorCode::from_wire("SOMETHING_NEW"), ErrorCode::Unknown);
        // The raw string survives on the ApiError for callers to match.
        let err = ApiError {
            status_code: 418,
            code: "teapot".into(),
            message: String::new(),
            request_id: String::new(),
        };
        assert_eq!(err.typed_code(), ErrorCode::Unknown);
        assert_eq!(err.code, "teapot");
    }
}
