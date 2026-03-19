use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

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

impl Client {
    /// Creates a new API key with optional scope and spend restrictions.
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
}
