use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Request body for text-to-speech.
#[derive(Debug, Clone, Serialize, Default)]
pub struct TtsRequest {
    /// TTS model (e.g. "tts-1", "eleven_multilingual_v2", "grok-3-tts").
    pub model: String,

    /// Text to synthesise into speech.
    pub text: String,

    /// Voice to use (e.g. "alloy", "echo", "nova", "Rachel").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,

    /// Audio format (e.g. "mp3", "wav", "opus"). Default: "mp3".
    #[serde(rename = "format", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// Speech rate (provider-dependent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

/// Response from text-to-speech.
#[derive(Debug, Clone, Deserialize)]
pub struct TtsResponse {
    /// Base64-encoded audio data.
    pub audio_base64: String,

    /// Audio format (e.g. "mp3").
    pub format: String,

    /// Audio file size.
    pub size_bytes: i64,

    /// Model that generated the audio.
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for speech-to-text.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SttRequest {
    /// STT model (e.g. "whisper-1", "scribe_v2").
    pub model: String,

    /// Base64-encoded audio data.
    pub audio_base64: String,

    /// Original filename (helps with format detection). Default: "audio.mp3".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// BCP-47 language code hint (e.g. "en", "de").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// Response from speech-to-text.
#[derive(Debug, Clone, Deserialize)]
pub struct SttResponse {
    /// Transcribed text.
    pub text: String,

    /// Model that performed transcription.
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for music generation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MusicRequest {
    /// Music generation model (e.g. "lyria").
    pub model: String,

    /// Describes the music to generate.
    pub prompt: String,

    /// Target duration in seconds (default 30).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,
}

/// Response from music generation.
#[derive(Debug, Clone, Deserialize)]
pub struct MusicResponse {
    /// Generated music clips.
    pub audio_clips: Vec<MusicClip>,

    /// Model that generated the music.
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// A single generated music clip.
#[derive(Debug, Clone, Deserialize)]
pub struct MusicClip {
    /// Base64-encoded audio data.
    pub base64: String,

    /// Audio format (e.g. "mp3", "wav").
    pub format: String,

    /// Audio file size.
    pub size_bytes: i64,

    /// Clip index within the batch.
    pub index: i32,
}

impl Client {
    /// Generates speech from text.
    pub async fn speak(&self, req: &TtsRequest) -> Result<TtsResponse> {
        let (mut resp, meta) = self
            .post_json::<TtsRequest, TtsResponse>("/qai/v1/audio/tts", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Converts speech to text.
    pub async fn transcribe(&self, req: &SttRequest) -> Result<SttResponse> {
        let (mut resp, meta) = self
            .post_json::<SttRequest, SttResponse>("/qai/v1/audio/stt", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Generates music from a text prompt.
    pub async fn generate_music(&self, req: &MusicRequest) -> Result<MusicResponse> {
        let (mut resp, meta) = self
            .post_json::<MusicRequest, MusicResponse>("/qai/v1/audio/music", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }
}
