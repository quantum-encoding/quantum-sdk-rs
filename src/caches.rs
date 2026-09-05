//! Gemini context caching.
//!
//! A cache pins an uploaded file (and an optional system prompt) at the
//! provider so follow-up turns are billed at the cached-read rate instead of
//! re-sending the whole file each time. Create a cache over a `file_uri` from
//! [`Client::file_upload`](crate::Client::file_upload), then pass the returned
//! `cache_name` as `cached_content` on subsequent chat requests.
//!
//! Caching is Gemini-only, and the provider requires a minimum of ~4096 tokens
//! of content — files below that are rejected with `cache_too_small`, and the
//! caller should fall back to attaching the file directly.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Request body for `POST /qai/v1/caches`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CacheCreateRequest {
    /// Provider file resource to cache (e.g. `files/abc123`). Required.
    pub file_uri: String,

    /// MIME type of the cached file. Required.
    pub mime_type: String,

    /// Gemini model id the cache is scoped to. Required — a cache can only be
    /// read back by the model it was created for.
    pub model: String,

    /// System prompt baked into the cached prefix so follow-up turns get the
    /// discount on it too.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<String>,

    /// Human-readable label. Defaults to the file URI tail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Requested lifetime in seconds. Clamped server-side to `[60, 86400]`;
    /// defaults to 3600. Cached tokens bill per stored token-hour, so a long
    /// TTL over a large file is real money.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<i64>,
}

/// Response from `POST /qai/v1/caches`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CacheCreateResponse {
    /// Provider resource name (`cachedContents/...`). Pass verbatim as
    /// `cached_content` on chat requests.
    #[serde(default)]
    pub cache_name: String,

    /// The model the cache is scoped to, echoed back.
    #[serde(default)]
    pub model: String,

    /// RFC3339 expiry. Chat calls referencing the cache after this 404.
    #[serde(default)]
    pub expires_at: String,

    /// The label set at creation, or the auto-derived one.
    #[serde(default)]
    pub display_name: String,

    /// Number of tokens the cache occupies, when the provider reported it.
    #[serde(default)]
    pub token_count: i64,
}

/// Response from `DELETE /qai/v1/caches/{name}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CacheDeleteResponse {
    /// True once the cache is released. A cache that already expired also
    /// reports `true` — the call is idempotent.
    #[serde(default)]
    pub deleted: bool,

    /// Present when the cache was already gone
    /// (`"already expired or unknown"`).
    #[serde(default)]
    pub note: Option<String>,
}

impl Client {
    /// Creates a context cache over an uploaded file.
    ///
    /// `POST /qai/v1/caches`
    pub async fn cache_create(&self, req: &CacheCreateRequest) -> Result<CacheCreateResponse> {
        let (resp, _meta) = self
            .post_json::<CacheCreateRequest, CacheCreateResponse>("/qai/v1/caches", req)
            .await?;
        Ok(resp)
    }

    /// Releases a context cache early rather than waiting for its TTL.
    ///
    /// `cache_name` may be the full `cachedContents/<id>` resource name or just
    /// the `<id>` suffix; the gateway normalises both.
    ///
    /// `DELETE /qai/v1/caches/{name}`
    pub async fn cache_delete(&self, cache_name: &str) -> Result<CacheDeleteResponse> {
        let (resp, _meta) = self
            .delete_json::<CacheDeleteResponse>(&format!("/qai/v1/caches/{cache_name}"))
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_request_omits_unset_options() {
        let req = CacheCreateRequest {
            file_uri: "files/abc123".into(),
            mime_type: "video/mp4".into(),
            model: "gemini-3.1-flash-lite".into(),
            ttl_seconds: Some(7200),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["ttl_seconds"], 7200);
        assert!(json.get("system_instruction").is_none());
        assert!(json.get("display_name").is_none());
    }

    #[test]
    fn delete_response_reports_already_expired() {
        let resp: CacheDeleteResponse =
            serde_json::from_str(r#"{"deleted":true,"note":"already expired or unknown"}"#)
                .expect("decode");
        assert!(resp.deleted);
        assert_eq!(resp.note.as_deref(), Some("already expired or unknown"));
    }
}
