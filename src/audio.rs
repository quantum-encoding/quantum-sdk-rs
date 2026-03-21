use std::collections::HashMap;

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
    #[serde(default)]
    pub audio_clips: Vec<MusicClip>,

    /// Model that generated the music.
    #[serde(default)]
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
    #[serde(default)]
    pub format: String,

    /// Audio file size.
    #[serde(default)]
    pub size_bytes: i64,

    /// Clip index within the batch.
    #[serde(default)]
    pub index: i32,
}

/// Request body for sound effects generation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SoundEffectRequest {
    /// Text prompt describing the sound effect.
    pub prompt: String,

    /// Optional duration in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
}

/// Response from sound effects generation.
#[derive(Debug, Clone, Deserialize)]
pub struct SoundEffectResponse {
    /// Base64-encoded audio data.
    pub audio_base64: String,

    /// Audio format (e.g. "mp3").
    pub format: String,

    /// File size in bytes.
    #[serde(default)]
    pub size_bytes: i64,

    /// Model used.
    #[serde(default)]
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// Advanced Audio Types
// ---------------------------------------------------------------------------

/// Generic audio response used by multiple advanced audio endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct AudioResponse {
    /// Base64-encoded audio data.
    #[serde(default)]
    pub audio_base64: Option<String>,

    /// Audio format (e.g. "mp3", "wav").
    #[serde(default)]
    pub format: Option<String>,

    /// File size in bytes.
    #[serde(default)]
    pub size_bytes: Option<i64>,

    /// Model used.
    #[serde(default)]
    pub model: Option<String>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,

    /// Additional response fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A single dialogue turn (used for building the request — converted to text + voices).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DialogueTurn {
    /// Speaker name or identifier.
    pub speaker: String,

    /// Text for this speaker to say.
    pub text: String,

    /// Voice ID to use for this speaker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
}

/// Voice mapping for ElevenLabs dialogue.
#[derive(Debug, Clone, Serialize)]
pub struct DialogueVoice {
    pub voice_id: String,
    pub name: String,
}

/// Request body sent to the QAI proxy for dialogue generation.
/// The proxy expects `text` (full script) + `voices` (speaker-to-voice mapping).
#[derive(Debug, Clone, Serialize, Default)]
pub struct DialogueRequest {
    /// Full dialogue script (e.g. "Speaker1: Hello!\nSpeaker2: Hi there!").
    pub text: String,

    /// Voice mappings — each speaker name mapped to a voice_id.
    pub voices: Vec<DialogueVoice>,

    /// Dialogue model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Output audio format.
    #[serde(rename = "output_format", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// Seed for reproducible generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
}

impl DialogueRequest {
    /// Build a DialogueRequest from individual turns.
    /// Converts turns into the text + voices format the API expects.
    pub fn from_turns(turns: Vec<DialogueTurn>, model: Option<String>) -> Self {
        // Build the script text: "Speaker: text\n..."
        let text = turns.iter()
            .map(|t| format!("{}: {}", t.speaker, t.text))
            .collect::<Vec<_>>()
            .join("\n");

        // Deduplicate voices — one entry per unique speaker
        let mut seen = std::collections::HashSet::new();
        let voices: Vec<DialogueVoice> = turns.iter()
            .filter(|t| t.voice.is_some() && seen.insert(t.speaker.clone()))
            .map(|t| DialogueVoice {
                voice_id: t.voice.clone().unwrap_or_default(),
                name: t.speaker.clone(),
            })
            .collect();

        Self {
            text,
            voices,
            model,
            ..Default::default()
        }
    }
}

/// Request body for speech-to-speech conversion.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SpeechToSpeechRequest {
    /// Model for conversion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Base64-encoded source audio.
    pub audio_base64: String,

    /// Target voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,

    /// Output audio format.
    #[serde(rename = "format", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

/// Request body for voice isolation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IsolateRequest {
    /// Base64-encoded audio to isolate voice from.
    pub audio_base64: String,

    /// Output audio format.
    #[serde(rename = "format", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

/// Request body for voice remixing.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RemixRequest {
    /// Base64-encoded source audio.
    pub audio_base64: String,

    /// Target voice for the remix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,

    /// Model for remixing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Output audio format.
    #[serde(rename = "format", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

/// Request body for audio dubbing.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DubRequest {
    /// Base64-encoded source audio or video.
    pub audio_base64: String,

    /// Original filename (helps detect format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Target language (BCP-47 code, e.g. "es", "de").
    pub target_language: String,

    /// Source language (auto-detected if omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_language: Option<String>,
}

/// Request body for audio alignment / forced alignment.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AlignRequest {
    /// Base64-encoded audio data.
    pub audio_base64: String,

    /// Transcript text to align against the audio.
    pub text: String,

    /// Language code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// A single alignment segment.
#[derive(Debug, Clone, Deserialize)]
pub struct AlignmentSegment {
    /// Aligned text.
    pub text: String,

    /// Start time in seconds.
    pub start: f64,

    /// End time in seconds.
    pub end: f64,
}

/// Response from audio alignment.
#[derive(Debug, Clone, Deserialize)]
pub struct AlignResponse {
    /// Aligned segments.
    pub segments: Vec<AlignmentSegment>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for voice design (generating a voice from a description).
#[derive(Debug, Clone, Serialize, Default)]
pub struct VoiceDesignRequest {
    /// Text description of the desired voice.
    pub description: String,

    /// Sample text to speak with the designed voice.
    pub text: String,

    /// Output audio format.
    #[serde(rename = "format", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

/// Request body for Starfish TTS.
#[derive(Debug, Clone, Serialize, Default)]
pub struct StarfishTTSRequest {
    /// Text to synthesise.
    pub text: String,

    /// Voice identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,

    /// Output audio format.
    #[serde(rename = "format", skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// Speech speed multiplier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

// ---------------------------------------------------------------------------
// Eleven Music (advanced music generation with sections, finetunes, etc.)
// ---------------------------------------------------------------------------

/// A section within an Eleven Music generation request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MusicSection {
    pub section_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_exclude: Option<String>,
}

/// Request body for advanced music generation (ElevenLabs Eleven Music).
#[derive(Debug, Clone, Serialize, Default)]
pub struct ElevenMusicRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<MusicSection>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocals: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_exclude: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finetune_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_reference_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_instruction: Option<String>,
}

/// A single music clip from advanced generation.
#[derive(Debug, Clone, Deserialize)]
pub struct ElevenMusicClip {
    /// Base64-encoded audio data.
    #[serde(default)]
    pub base64: String,
    /// Audio format (e.g. "mp3").
    #[serde(default)]
    pub format: String,
    /// File size in bytes.
    #[serde(default)]
    pub size: i64,
}

/// Response from advanced music generation.
/// Backend returns: { clips: [...], model, cost_ticks, request_id }
#[derive(Debug, Clone, Deserialize)]
pub struct ElevenMusicResponse {
    /// Generated music clips.
    #[serde(default)]
    pub clips: Vec<ElevenMusicClip>,
    /// Model used.
    #[serde(default)]
    pub model: String,
    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,
    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Info about a music finetune.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinetuneInfo {
    pub finetune_id: String,
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Response from listing finetunes.
#[derive(Debug, Clone, Deserialize)]
pub struct ListFinetunesResponse {
    pub finetunes: Vec<FinetuneInfo>,
}

// ---------------------------------------------------------------------------
// Client impl
// ---------------------------------------------------------------------------

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

    /// Generates sound effects from a text prompt (ElevenLabs).
    pub async fn sound_effects(&self, req: &SoundEffectRequest) -> Result<SoundEffectResponse> {
        let (mut resp, meta) = self
            .post_json::<SoundEffectRequest, SoundEffectResponse>(
                "/qai/v1/audio/sound-effects",
                req,
            )
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

    /// Generates multi-speaker dialogue audio.
    pub async fn dialogue(&self, req: &DialogueRequest) -> Result<AudioResponse> {
        let (mut resp, meta) = self
            .post_json::<DialogueRequest, AudioResponse>("/qai/v1/audio/dialogue", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Converts speech to a different voice.
    pub async fn speech_to_speech(
        &self,
        req: &SpeechToSpeechRequest,
    ) -> Result<AudioResponse> {
        let (mut resp, meta) = self
            .post_json::<SpeechToSpeechRequest, AudioResponse>(
                "/qai/v1/audio/speech-to-speech",
                req,
            )
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Isolates voice from background noise and music.
    pub async fn isolate_voice(&self, req: &IsolateRequest) -> Result<AudioResponse> {
        let (mut resp, meta) = self
            .post_json::<IsolateRequest, AudioResponse>("/qai/v1/audio/isolate", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Remixes audio with a different voice.
    pub async fn remix_voice(&self, req: &RemixRequest) -> Result<AudioResponse> {
        let (mut resp, meta) = self
            .post_json::<RemixRequest, AudioResponse>("/qai/v1/audio/remix", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Dubs audio or video into a target language.
    pub async fn dub(&self, req: &DubRequest) -> Result<AudioResponse> {
        let (mut resp, meta) = self
            .post_json::<DubRequest, AudioResponse>("/qai/v1/audio/dub", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Performs forced alignment of text against audio.
    pub async fn align(&self, req: &AlignRequest) -> Result<AlignResponse> {
        let (mut resp, meta) = self
            .post_json::<AlignRequest, AlignResponse>("/qai/v1/audio/align", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Designs a new voice from a text description and generates sample audio.
    pub async fn voice_design(&self, req: &VoiceDesignRequest) -> Result<AudioResponse> {
        let (mut resp, meta) = self
            .post_json::<VoiceDesignRequest, AudioResponse>("/qai/v1/audio/voice-design", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Generates speech using Starfish TTS (HeyGen).
    pub async fn starfish_tts(&self, req: &StarfishTTSRequest) -> Result<AudioResponse> {
        let (mut resp, meta) = self
            .post_json::<StarfishTTSRequest, AudioResponse>("/qai/v1/audio/starfish-tts", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Generates music via ElevenLabs Eleven Music (advanced: sections, finetunes, edits).
    pub async fn generate_music_advanced(
        &self,
        req: &ElevenMusicRequest,
    ) -> Result<ElevenMusicResponse> {
        let (mut resp, meta) = self
            .post_json::<ElevenMusicRequest, ElevenMusicResponse>(
                "/qai/v1/audio/music/advanced",
                req,
            )
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Lists all music finetunes for the authenticated user.
    pub async fn list_finetunes(&self) -> Result<ListFinetunesResponse> {
        let (resp, _) = self
            .get_json::<ListFinetunesResponse>("/qai/v1/audio/finetunes")
            .await?;
        Ok(resp)
    }

    /// Creates a new music finetune from audio sample files.
    pub async fn create_finetune(
        &self,
        name: &str,
        files: Vec<crate::voices::CloneVoiceFile>,
    ) -> Result<FinetuneInfo> {
        let mut form = reqwest::multipart::Form::new().text("name", name.to_string());

        for file in files {
            let part = reqwest::multipart::Part::bytes(file.data)
                .file_name(file.filename)
                .mime_str(&file.mime_type)
                .map_err(|e| crate::error::Error::Http(e.into()))?;
            form = form.part("files", part);
        }

        let (resp, _) = self
            .post_multipart::<FinetuneInfo>("/qai/v1/audio/finetunes", form)
            .await?;
        Ok(resp)
    }

    /// Deletes a music finetune by ID.
    pub async fn delete_finetune(&self, id: &str) -> Result<serde_json::Value> {
        let path = format!("/qai/v1/audio/finetunes/{id}");
        let (resp, _) = self.delete_json::<serde_json::Value>(&path).await?;
        Ok(resp)
    }
}
