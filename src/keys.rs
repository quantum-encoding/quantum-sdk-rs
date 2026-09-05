use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::region::Region;

/// Request body for creating an API key.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CreateKeyRequest {
    /// Human-readable name for the key.
    pub name: String,

    /// Restrict to specific endpoints (e.g. ["chat", "images"]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<String>>,

    /// Maximum spend in USD before the key is disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spend_cap_usd: Option<f64>,

    /// Rate limit in requests per minute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<i32>,

    /// Routing region for every request made with this key: `"americas"`,
    /// `"europe"`, or `"asia"` (see [`Region`]). The gateway scopes the
    /// key's inference routing to that region; unset = unscoped legacy
    /// routing. Honored on standard key creation only — partner and
    /// ephemeral keys ignore it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

impl CreateKeyRequest {
    /// Sets the key's routing region (typed — rejects unknown values at
    /// compile time rather than letting the gateway silently route them
    /// unscoped).
    pub fn region(mut self, region: Region) -> Self {
        self.region = Some(region.as_str().to_string());
        self
    }
}

/// Details about an API key (returned on creation and listing).
#[derive(Debug, Clone, Deserialize)]
pub struct KeyDetails {
    /// Unique key identifier.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// First characters of the key for identification.
    pub key_prefix: String,

    /// Scope restrictions.
    #[serde(default)]
    pub scope: Option<serde_json::Value>,

    /// Amount spent by this key in ticks.
    #[serde(default)]
    pub spent_ticks: i64,

    /// Whether the key has been revoked.
    #[serde(default)]
    pub revoked: bool,

    /// Creation timestamp (RFC 3339).
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last usage timestamp (RFC 3339). Only present in list responses.
    #[serde(default)]
    pub last_used: Option<String>,
}

impl KeyDetails {
    /// The key's effective routing region from its scope (`scope.region`) —
    /// `"americas"`, `"europe"`, or `"asia"` when the key is region-scoped,
    /// `None` for unscoped legacy keys. Parse with [`Region::parse`] for the
    /// typed form.
    pub fn scope_region(&self) -> Option<&str> {
        self.scope
            .as_ref()
            .and_then(|s| s.get("region"))
            .and_then(|r| r.as_str())
            .filter(|r| !r.is_empty())
    }
}

/// Response from creating an API key.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateKeyResponse {
    /// The full API key (only shown once on creation).
    pub key: String,

    /// Key metadata.
    pub details: KeyDetails,
}

/// Response from listing API keys.
#[derive(Debug, Clone, Deserialize)]
pub struct ListKeysResponse {
    /// All keys for the account.
    pub keys: Vec<KeyDetails>,
}

/// Generic status response for operations that return a simple status.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusResponse {
    /// Status message (e.g. "ok", "deleted", "revoked").
    pub status: String,

    /// Optional human-readable message.
    #[serde(default)]
    pub message: Option<String>,
}

/// A per-device default key, as listed by [`Client::list_device_keys`].
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceKey {
    /// Key identifier.
    pub key_id: String,
    /// The device the key was minted for.
    #[serde(default)]
    pub device_id: Option<String>,
    /// First characters of the key.
    #[serde(default)]
    pub key_prefix: Option<String>,
    /// When it was created.
    #[serde(default)]
    pub created_at: Option<String>,
    /// When it was last used, if ever.
    #[serde(default)]
    pub last_used: Option<String>,
}

/// Response from listing device keys.
#[derive(Debug, Clone, Deserialize)]
pub struct ListDeviceKeysResponse {
    pub devices: Vec<DeviceKey>,
}

/// Request body for rotating a key.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RotateKeyRequest {
    /// How long the old key keeps working after rotation, in seconds.
    pub grace_seconds: i64,
}

/// Response from rotating a key.
#[derive(Debug, Clone, Deserialize)]
pub struct RotateKeyResponse {
    /// The new key, shown once.
    pub key: String,
    /// The new key's details.
    pub details: KeyDetails,
    /// The id of the key that was rotated out.
    #[serde(default)]
    pub old_key_id: Option<String>,
}

/// One day of a key's usage.
#[derive(Debug, Clone, Deserialize)]
pub struct KeyUsageDay {
    pub day: String,
    pub requests: i64,
    pub cost_usd: f64,
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
}

/// A key's usage on one model.
#[derive(Debug, Clone, Deserialize)]
pub struct KeyUsageModel {
    pub model: String,
    pub requests: i64,
    pub cost_usd: f64,
}

/// Response from [`Client::key_usage`].
#[derive(Debug, Clone, Deserialize)]
pub struct KeyUsageResponse {
    pub days: Vec<KeyUsageDay>,
    pub models: Vec<KeyUsageModel>,
    pub total_cost_usd: f64,
}

/// Request body for an ephemeral key: a short-lived token a server hands
/// to a browser or device so it can call the gateway directly.
#[derive(Debug, Clone, Serialize, Default)]
pub struct EphemeralKeyRequest {
    /// Lifetime in seconds (default 3600, at most 86400).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,
    /// Downstream user id, for attribution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_ref: Option<String>,
    /// Spend cap for the session, in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spend_cap: Option<f64>,
    /// Endpoints the token may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<String>>,
    /// Requests per minute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<i32>,
}

/// Response from [`Client::create_ephemeral_key`].
#[derive(Debug, Clone, Deserialize)]
pub struct EphemeralKeyResponse {
    /// The token, shown once.
    pub token: String,
    /// When it expires (RFC 3339).
    pub expires_at: String,
    /// The gateway the token is for.
    #[serde(default)]
    pub base_url: Option<String>,
}

/// Request body for a partner key: a key minted on behalf of a partner
/// app's end user, attributed to that user.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PartnerKeyRequest {
    /// The partner (e.g. `"cosmicduck"`).
    pub partner_id: String,
    /// The partner's user id.
    pub partner_ref: String,
    /// Display name (default `partner:{partner_ref}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Spend cap for the key, in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spend_cap_usd: Option<f64>,
    /// Endpoints the key may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<String>>,
    /// Requests per minute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<i32>,
}

/// Response from [`Client::create_partner_key`].
#[derive(Debug, Clone, Deserialize)]
pub struct PartnerKeyResponse {
    /// The key, shown once.
    pub key: String,
    /// The key's details.
    pub details: KeyDetails,
    /// The gateway the key is for.
    #[serde(default)]
    pub base_url: Option<String>,
}

impl Client {
    /// Creates a new API key with optional scope and spend restrictions.
    ///
    /// The gateway does not dedupe key minting, so the request is never
    /// replayed on a 502/503/504: if one masks a completed mint, the key
    /// exists but its secret was never delivered — list keys and revoke
    /// it rather than minting again blindly.
    pub async fn create_key(&self, req: &CreateKeyRequest) -> Result<CreateKeyResponse> {
        let (resp, _meta) = self
            .post_json::<CreateKeyRequest, CreateKeyResponse>("/qai/v1/keys", req)
            .await?;
        Ok(resp)
    }

    /// Lists all API keys for the account.
    pub async fn list_keys(&self) -> Result<ListKeysResponse> {
        let (resp, _meta) = self.get_json::<ListKeysResponse>("/qai/v1/keys").await?;
        Ok(resp)
    }

    /// Revokes an API key by its ID.
    pub async fn revoke_key(&self, id: &str) -> Result<StatusResponse> {
        let path = format!("/qai/v1/keys/{id}");
        let (resp, _meta) = self.delete_json::<StatusResponse>(&path).await?;
        Ok(resp)
    }

    /// Lists the account's per-device default keys.
    pub async fn list_device_keys(&self) -> Result<ListDeviceKeysResponse> {
        let (resp, _meta) = self
            .get_json::<ListDeviceKeysResponse>("/qai/v1/keys/devices")
            .await?;
        Ok(resp)
    }

    /// Rotates a key: mints a replacement and keeps the old one working for
    /// `grace_seconds` so deployed clients can pick the new one up.
    ///
    /// Never replayed on a 5xx: a second rotate of an already-rotated id
    /// is a 409 `invalid_state`, and the only copy of the new secret was
    /// in the lost response. On a 502/503/504 here, treat the rotation
    /// as possibly done, list keys, and rotate the *new* id if you must.
    pub async fn rotate_key(&self, id: &str, req: &RotateKeyRequest) -> Result<RotateKeyResponse> {
        let path = format!("/qai/v1/keys/{id}/rotate");
        let (resp, _meta) = self
            .post_json::<RotateKeyRequest, RotateKeyResponse>(&path, req)
            .await?;
        Ok(resp)
    }

    /// A key's usage by day and by model.
    pub async fn key_usage(&self, id: &str) -> Result<KeyUsageResponse> {
        let path = format!("/qai/v1/keys/{id}/usage");
        let (resp, _meta) = self.get_json::<KeyUsageResponse>(&path).await?;
        Ok(resp)
    }

    /// Mints a short-lived token for a browser or device.
    ///
    /// Only accounts on the `internal` developer tier (and admins) may
    /// call this; any other account gets a 403 `forbidden` before the
    /// body is read. Single attempt: a replay could mint a second token
    /// whose secret nobody received.
    pub async fn create_ephemeral_key(
        &self,
        req: &EphemeralKeyRequest,
    ) -> Result<EphemeralKeyResponse> {
        let (resp, _meta) = self
            .post_json::<EphemeralKeyRequest, EphemeralKeyResponse>("/qai/v1/keys/ephemeral", req)
            .await?;
        Ok(resp)
    }

    /// Mints a key on behalf of a partner app's end user.
    ///
    /// Only accounts on the `internal` developer tier (and admins) may
    /// call this; any other account gets a 403 `forbidden` before the
    /// body is read. Single attempt, like every key-minting call.
    pub async fn create_partner_key(&self, req: &PartnerKeyRequest) -> Result<PartnerKeyResponse> {
        let (resp, _meta) = self
            .post_json::<PartnerKeyRequest, PartnerKeyResponse>("/qai/v1/keys/partner", req)
            .await?;
        Ok(resp)
    }
}
