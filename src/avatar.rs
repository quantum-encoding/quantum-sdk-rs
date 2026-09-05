//! HeyGen Avatar Realtime (Broadcast) sessions.
//!
//! A realtime session makes an avatar speak live and publishes a plain HLS
//! stream (720p). Sessions are prepaid: the entire `max_duration_seconds`
//! block is charged at create time and is not refunded on early cancel
//! (cancelling only stops the upstream meter).
//!
//! Recommended flow:
//! 1. [`Client::create_avatar_realtime_session`] → `stream_id`
//! 2. Poll [`Client::get_avatar_realtime_session`] (~2s) until `status == "streaming"`,
//!    then play `hls_url`
//! 3. For `text_stream` sessions, append text with
//!    [`Client::send_avatar_realtime_text`] and close with `is_final: true`
//!    (idle timeout is ~30s without new text)
//! 4. [`Client::cancel_avatar_realtime_session`] as soon as you're done
//!
//! Not to be confused with the WebSocket voice realtime API in
//! [`crate::realtime`].

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Audio input union for `audio`-type realtime sessions,
/// discriminated by `input_type` (wire field `type`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AvatarAudioInput {
    /// Input kind: "url" | "asset_id" | "base64".
    #[serde(rename = "type")]
    pub input_type: String,

    /// Publicly accessible HTTPS URL (when `input_type == "url"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// HeyGen asset id from an asset upload (when `input_type == "asset_id"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,

    /// MIME type, e.g. "audio/mpeg" (when `input_type == "base64"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,

    /// Base64-encoded audio bytes (when `input_type == "base64"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Request body for creating a live avatar session (prepaid).
#[derive(Debug, Clone, Serialize, Default)]
pub struct AvatarRealtimeRequest {
    /// Session kind: "tts" | "audio" | "text_stream".
    #[serde(rename = "type")]
    pub session_type: String,

    /// HeyGen photo-avatar / motion-avatar look id (required for all kinds).
    pub avatar_id: String,

    /// Voice id — required for "tts" and "text_stream", must be omitted for
    /// "audio". The omission rule is enforced by the gateway's HeyGen
    /// client before the upstream call and surfaces as a 502
    /// `provider_error`, not a 400.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,

    /// The fixed script ("tts") or the initial non-empty seed ("text_stream").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Audio input — required for "audio", must be omitted for "tts"/"text_stream".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AvatarAudioInput>,

    /// Prepaid block in seconds (1–3600), charged in full at create time.
    pub max_duration_seconds: i32,
}

/// Response from creating a live avatar session.
#[derive(Debug, Clone, Deserialize)]
pub struct AvatarRealtimeCreateResponse {
    /// Session id — use in the status/text/cancel calls.
    pub stream_id: String,

    /// Always "pending" at create.
    #[serde(default)]
    pub status: String,

    /// Echo of `max_duration_seconds`.
    #[serde(default)]
    pub prepaid_seconds: i32,

    /// Ticks charged for the prepaid block.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Post-deduction credit balance in ticks (from the X-QAI-Balance-After
    /// header).
    #[serde(default)]
    pub balance_after: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Response from a session status check.
#[derive(Debug, Clone, Deserialize)]
pub struct AvatarRealtimeStatusResponse {
    /// Session id.
    pub stream_id: String,

    /// "pending" | "streaming" | "completed" | "error".
    pub status: String,

    /// HLS `.m3u8` playback URL (720p); present once streaming.
    #[serde(default)]
    pub hls_url: Option<String>,

    /// Failure detail when `status == "error"`.
    #[serde(default)]
    pub error_message: Option<String>,

    /// On completed text_stream sessions: "final_marker" | "idle_timeout".
    #[serde(default)]
    pub end_reason: Option<String>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for appending a text delta to a `text_stream` session.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AvatarRealtimeTextRequest {
    /// Text fragment to append (a token or coalesced batch). Required unless
    /// `is_final` is true, in which case it may be empty.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub delta: String,

    /// True closes the text input. Appending afterwards is refused by
    /// HeyGen; the gateway passes the upstream 4xx through as a
    /// `provider_error` (410 per HeyGen's documentation). Wire field:
    /// `final`.
    #[serde(rename = "final")]
    pub is_final: bool,
}

impl AvatarRealtimeTextRequest {
    /// A delta-append request.
    pub fn delta(delta: impl Into<String>) -> Self {
        Self {
            delta: delta.into(),
            is_final: false,
        }
    }

    /// A close-the-stream request (empty final marker).
    pub fn final_marker() -> Self {
        Self {
            delta: String::new(),
            is_final: true,
        }
    }
}

/// Response from appending a text delta.
#[derive(Debug, Clone, Deserialize)]
pub struct AvatarRealtimeTextResponse {
    /// Always true on success.
    #[serde(default)]
    pub ok: bool,

    /// Total text bytes buffered for the session so far.
    #[serde(default)]
    pub buffered_bytes: i64,

    /// Echo of the request's `final` flag. Wire field: `final`.
    #[serde(rename = "final", default)]
    pub is_final: bool,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Response from cancelling a session early.
#[derive(Debug, Clone, Deserialize)]
pub struct AvatarRealtimeCancelResponse {
    /// Session id.
    pub stream_id: String,

    /// True = this call initiated cancellation; false = the session was
    /// already terminal (cancel is idempotent).
    #[serde(default)]
    pub cancelled: bool,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// Client impl
// ---------------------------------------------------------------------------

impl Client {
    /// Creates a live avatar realtime session (HeyGen Broadcast).
    ///
    /// The entire `max_duration_seconds` block (1–3600 s) is charged at
    /// create time; cancelling early does not refund it.
    pub async fn create_avatar_realtime_session(
        &self,
        req: &AvatarRealtimeRequest,
    ) -> Result<AvatarRealtimeCreateResponse> {
        let (mut resp, meta) = self
            .post_json::<AvatarRealtimeRequest, AvatarRealtimeCreateResponse>(
                "/qai/v1/avatar/realtime",
                req,
            )
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.balance_after == 0 {
            resp.balance_after = meta.balance_after;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Gets the live status of an avatar realtime session.
    ///
    /// Poll (~2s) until `status == "streaming"`, then play `hls_url`.
    /// "completed" and "error" are terminal.
    pub async fn get_avatar_realtime_session(
        &self,
        stream_id: &str,
    ) -> Result<AvatarRealtimeStatusResponse> {
        let path = format!("/qai/v1/avatar/realtime/{stream_id}");
        let (resp, _meta) = self.get_json::<AvatarRealtimeStatusResponse>(&path).await?;
        Ok(resp)
    }

    /// Appends a text delta to a `text_stream` session (or closes it with
    /// [`AvatarRealtimeTextRequest::final_marker`]).
    pub async fn send_avatar_realtime_text(
        &self,
        stream_id: &str,
        req: &AvatarRealtimeTextRequest,
    ) -> Result<AvatarRealtimeTextResponse> {
        let path = format!("/qai/v1/avatar/realtime/{stream_id}/text");
        let (resp, _meta) = self
            .post_json::<AvatarRealtimeTextRequest, AvatarRealtimeTextResponse>(&path, req)
            .await?;
        Ok(resp)
    }

    /// Terminates an avatar realtime session early (idempotent; no refund —
    /// this only stops HeyGen's upstream meter).
    pub async fn cancel_avatar_realtime_session(
        &self,
        stream_id: &str,
    ) -> Result<AvatarRealtimeCancelResponse> {
        let path = format!("/qai/v1/avatar/realtime/{stream_id}/cancel");
        let (resp, _meta) = self
            .post_json_empty::<AvatarRealtimeCancelResponse>(&path)
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_request_serialization() {
        // Delta append: "final" always present, delta carried.
        let json = serde_json::to_value(AvatarRealtimeTextRequest::delta(" more")).unwrap();
        assert_eq!(json["delta"], " more");
        assert_eq!(json["final"], false);

        // Final marker: empty delta omitted entirely.
        let json = serde_json::to_value(AvatarRealtimeTextRequest::final_marker()).unwrap();
        assert!(json.get("delta").is_none());
        assert_eq!(json["final"], true);
    }

    #[test]
    fn create_request_omits_empty_optionals() {
        let req = AvatarRealtimeRequest {
            session_type: "tts".into(),
            avatar_id: "av_1".into(),
            voice_id: Some("v_1".into()),
            text: Some("Hello".into()),
            audio: None,
            max_duration_seconds: 60,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["type"], "tts");
        assert_eq!(json["avatar_id"], "av_1");
        assert_eq!(json["voice_id"], "v_1");
        assert_eq!(json["text"], "Hello");
        assert_eq!(json["max_duration_seconds"], 60);
        assert!(json.get("audio").is_none());
    }

    #[test]
    fn audio_input_serialization() {
        let req = AvatarRealtimeRequest {
            session_type: "audio".into(),
            avatar_id: "av_1".into(),
            voice_id: None,
            text: None,
            audio: Some(AvatarAudioInput {
                input_type: "base64".into(),
                media_type: Some("audio/mpeg".into()),
                data: Some("AQID".into()),
                ..Default::default()
            }),
            max_duration_seconds: 120,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("voice_id").is_none());
        assert!(json.get("text").is_none());
        assert_eq!(json["audio"]["type"], "base64");
        assert_eq!(json["audio"]["media_type"], "audio/mpeg");
        assert_eq!(json["audio"]["data"], "AQID");
        assert!(json["audio"].get("url").is_none());
    }
}
