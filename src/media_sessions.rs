//! Media sessions — server-side state for multi-turn Q&A over one uploaded
//! media file.
//!
//! A session pins three things on the gateway: a Gemini Files API resource
//! (uploaded via [`Client::file_upload`](crate::Client::file_upload)), a Vertex
//! context cache built over that file at session boot, and the conversation
//! history. While the cache is alive a chat turn sends only the next user
//! message and is billed at the cached-read rate; once `expires_at` has
//! passed the turn re-sends the file inline at the full input rate and the
//! gateway rebuilds the cache afterwards.
//!
//! Sessions are stored server-side, so the same session id resumes from any
//! device.

use serde::{Deserialize, Serialize};

use crate::chat::ChatUsage;
use crate::client::Client;
use crate::error::Result;
use crate::serde_util::null_as_default;

/// Request body for `POST /qai/v1/media-sessions`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MediaSessionCreateRequest {
    /// Gemini Files API resource to pin, e.g. `files/abc123` or the
    /// fully-qualified upload URI. Required.
    pub file_uri: String,

    /// MIME type of the pinned file (e.g. `video/mp4`). Required.
    pub mime_type: String,

    /// Gemini model the session's cache is scoped to. Required, and must be a
    /// `gemini-*` id — context caching is Gemini-only.
    pub model: String,

    /// System prompt baked into the cached prefix, so follow-up turns get the
    /// cache discount on it too.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<String>,

    /// Human-readable label for the session's cache. Defaults to the file URI
    /// tail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Requested cache TTL in seconds. Clamped server-side to `[60, 86400]`;
    /// defaults to 3600.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl_seconds: Option<i64>,
}

/// One persisted turn of a media session's conversation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaSessionTurn {
    /// `"user"` or `"assistant"`.
    #[serde(default)]
    pub role: String,

    /// The message text.
    #[serde(default)]
    pub content: String,

    /// RFC3339 timestamp of the turn.
    #[serde(default)]
    pub at: Option<String>,
}

/// A media session record as returned by create / get / list.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MediaSession {
    /// Session identifier — the `{id}` in the session sub-routes.
    #[serde(default)]
    pub id: String,

    /// The pinned Gemini Files API resource.
    #[serde(default)]
    pub file_uri: String,

    /// MIME type of the pinned file.
    #[serde(default)]
    pub mime_type: String,

    /// The session's display name, else the tail of the file URI, else
    /// "untitled media session".
    #[serde(default)]
    pub file_display_name: Option<String>,

    /// Vertex cache resource name (`cachedContents/...`) backing the session.
    #[serde(default)]
    pub cache_name: String,

    /// Input-token count of the cache at default media resolution. Zero means
    /// the gateway could not determine it.
    #[serde(default)]
    pub cache_token_count: i64,

    /// Gemini model the session's cache is scoped to.
    #[serde(default)]
    pub model: String,

    /// System prompt baked into the cached prefix, if any.
    #[serde(default)]
    pub system_instruction: Option<String>,

    /// Conversation history, oldest first.
    #[serde(default, deserialize_with = "null_as_default")]
    pub history: Vec<MediaSessionTurn>,

    /// RFC3339 creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// RFC3339 timestamp of the last chat turn.
    #[serde(default)]
    pub last_used_at: Option<String>,

    /// RFC3339 timestamp at which the underlying cache expires.
    #[serde(default)]
    pub expires_at: Option<String>,

    /// Number of messages recorded on the session.
    #[serde(default)]
    pub message_count: i64,
}

/// Response from `GET /qai/v1/media-sessions`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MediaSessionListResponse {
    /// The caller's sessions, most recently used first.
    #[serde(default, deserialize_with = "null_as_default")]
    pub sessions: Vec<MediaSession>,
}

/// Request body for `POST /qai/v1/media-sessions/{id}/chat`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MediaSessionChatRequest {
    /// The next user message. Required.
    pub message: String,

    /// Output token cap for this turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Sampling temperature for this turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// JSON Schema the model is forced to fill for this turn. Applied per-turn
    /// without invalidating the session cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

/// Response from `POST /qai/v1/media-sessions/{id}/chat`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MediaSessionChatResponse {
    /// The session the turn was appended to.
    #[serde(default)]
    pub session_id: String,

    /// The assistant's reply.
    #[serde(default)]
    pub answer: String,

    /// Token usage and cost for this turn.
    #[serde(default)]
    pub usage: Option<ChatUsage>,

    /// The session's full history including this turn.
    #[serde(default, deserialize_with = "null_as_default")]
    pub history: Vec<MediaSessionTurn>,
}

/// Response from `DELETE /qai/v1/media-sessions/{id}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MediaSessionDeleteResponse {
    /// True once the session record is deleted; releasing the context cache
    /// is best-effort and does not change the answer. Deleting an
    /// already-absent session also reports `true` — the call is idempotent.
    #[serde(default)]
    pub deleted: bool,

    /// Present when the session was already gone (`"already absent"`).
    #[serde(default)]
    pub note: Option<String>,
}

impl Client {
    /// Creates a media session: pins a file, builds a context cache over it,
    /// and returns the session record.
    ///
    /// `POST /qai/v1/media-sessions`
    pub async fn media_session_create(
        &self,
        req: &MediaSessionCreateRequest,
    ) -> Result<MediaSession> {
        let (resp, _meta) = self
            .post_json::<MediaSessionCreateRequest, MediaSession>("/qai/v1/media-sessions", req)
            .await?;
        Ok(resp)
    }

    /// Lists the caller's media sessions, most recently used first, at most
    /// fifty.
    ///
    /// `GET /qai/v1/media-sessions`
    pub async fn media_session_list(&self) -> Result<MediaSessionListResponse> {
        let (resp, _meta) = self
            .get_json::<MediaSessionListResponse>("/qai/v1/media-sessions")
            .await?;
        Ok(resp)
    }

    /// Reads one media session's state, including its conversation history.
    ///
    /// `GET /qai/v1/media-sessions/{id}`
    pub async fn media_session_get(&self, id: &str) -> Result<MediaSession> {
        let (resp, _meta) = self
            .get_json::<MediaSession>(&format!("/qai/v1/media-sessions/{id}"))
            .await?;
        Ok(resp)
    }

    /// Sends the next user turn to a media session and returns the answer.
    ///
    /// `POST /qai/v1/media-sessions/{id}/chat`
    pub async fn media_session_chat(
        &self,
        id: &str,
        req: &MediaSessionChatRequest,
    ) -> Result<MediaSessionChatResponse> {
        let (resp, _meta) = self
            .post_json::<MediaSessionChatRequest, MediaSessionChatResponse>(
                &format!("/qai/v1/media-sessions/{id}/chat"),
                req,
            )
            .await?;
        Ok(resp)
    }

    /// Deletes a media session; its context cache is released on a
    /// best-effort basis.
    ///
    /// `DELETE /qai/v1/media-sessions/{id}`
    pub async fn media_session_delete(&self, id: &str) -> Result<MediaSessionDeleteResponse> {
        let (resp, _meta) = self
            .delete_json::<MediaSessionDeleteResponse>(&format!("/qai/v1/media-sessions/{id}"))
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_request_omits_unset_options() {
        let req = MediaSessionCreateRequest {
            file_uri: "files/abc123".into(),
            mime_type: "video/mp4".into(),
            model: "gemini-3.1-flash-lite".into(),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["file_uri"], "files/abc123");
        assert!(json.get("system_instruction").is_none());
        assert!(json.get("display_name").is_none());
        assert!(json.get("cache_ttl_seconds").is_none());
    }

    #[test]
    fn session_decodes_null_history() {
        let session: MediaSession = serde_json::from_str(
            r#"{"id":"s1","file_uri":"files/a","mime_type":"video/mp4",
                "cache_name":"cachedContents/x","model":"gemini-3.1-flash-lite",
                "history":null,"message_count":0}"#,
        )
        .expect("decode");
        assert_eq!(session.id, "s1");
        assert!(session.history.is_empty());
        assert_eq!(session.cache_token_count, 0);
    }

    #[test]
    fn chat_response_decodes_history_and_usage() {
        let resp: MediaSessionChatResponse = serde_json::from_str(
            r#"{"session_id":"s1","answer":"it is a duck",
                "usage":{"input_tokens":10,"output_tokens":4,"cost_ticks":7},
                "history":[{"role":"user","content":"what is it?","at":"2026-01-01T00:00:00Z"}]}"#,
        )
        .expect("decode");
        assert_eq!(resp.answer, "it is a duck");
        assert_eq!(resp.history.len(), 1);
        assert_eq!(resp.usage.expect("usage").cost_ticks, 7);
    }
}
