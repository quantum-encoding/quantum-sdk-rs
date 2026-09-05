use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::{ApiError, Error, Result};
use crate::serde_util::null_as_default;

/// Request body for text-to-speech.
#[derive(Debug, Clone, Serialize, Default)]
pub struct TextToSpeechRequest {
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

/// Backwards-compatible alias.
pub type TtsRequest = TextToSpeechRequest;

/// Response from text-to-speech.
#[derive(Debug, Clone, Deserialize)]
pub struct TextToSpeechResponse {
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

    /// Post-deduction credit balance in ticks.
    #[serde(default)]
    pub balance_after: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Backwards-compatible alias.
pub type TtsResponse = TextToSpeechResponse;

/// Request body for speech-to-text.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SpeechToTextRequest {
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

/// Backwards-compatible alias.
pub type SttRequest = SpeechToTextRequest;

/// Response from speech-to-text.
#[derive(Debug, Clone, Deserialize)]
pub struct SpeechToTextResponse {
    /// Transcribed text.
    pub text: String,

    /// Model that performed transcription.
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Post-deduction credit balance in ticks.
    #[serde(default)]
    pub balance_after: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Backwards-compatible alias.
pub type SttResponse = SpeechToTextResponse;

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
    #[serde(default, deserialize_with = "null_as_default")]
    pub audio_clips: Vec<MusicClip>,

    /// Model that generated the music.
    #[serde(default)]
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Post-deduction credit balance in ticks.
    #[serde(default)]
    pub balance_after: i64,

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
// Dialogue (ElevenLabs multi-speaker)
// ---------------------------------------------------------------------------

/// A single dialogue turn, used to build a [`DialogueRequest`] with
/// [`DialogueRequest::from_turns`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DialogueTurn {
    /// Speaker name or identifier.
    pub speaker: String,

    /// Text for this speaker to say.
    pub text: String,

    /// Voice ID for this speaker. A speaker needs a voice on at least one
    /// of its turns; every turn of the same speaker must agree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
}

/// Voice mapping for ElevenLabs dialogue.
#[derive(Debug, Clone, Serialize)]
pub struct DialogueVoice {
    pub voice_id: String,
    pub name: String,
}

/// Request body for dialogue generation: `text` (the full script) plus
/// `voices` (speaker-to-voice mapping). Billed per character of `text`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DialogueRequest {
    /// Full dialogue script (e.g. "Speaker1: Hello!\nSpeaker2: Hi there!").
    pub text: String,

    /// Voice mappings: each speaker name mapped to a voice_id (at least one).
    pub voices: Vec<DialogueVoice>,

    /// Dialogue model (default "eleven_v3").
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
    /// Builds a request from individual turns: the script becomes
    /// "Speaker: text" lines and `voices` gets one entry per speaker, in
    /// order of first appearance.
    ///
    /// Every speaker must carry a voice on at least one turn, and all of a
    /// speaker's turns must name the same voice; otherwise this returns an
    /// `invalid_request` error (status 0, raised locally) rather than
    /// sending a script the gateway would bill with an unmapped speaker.
    pub fn from_turns(turns: Vec<DialogueTurn>, model: Option<String>) -> Result<Self> {
        let text = turns
            .iter()
            .map(|t| format!("{}: {}", t.speaker, t.text))
            .collect::<Vec<_>>()
            .join("\n");

        let mut voices: Vec<DialogueVoice> = Vec::new();
        for turn in &turns {
            let Some(voice_id) = &turn.voice else {
                continue;
            };
            match voices.iter().find(|v| v.name == turn.speaker) {
                Some(existing) if existing.voice_id != *voice_id => {
                    return Err(local_invalid_request(format!(
                        "speaker {:?} maps to both voice {:?} and voice {:?}",
                        turn.speaker, existing.voice_id, voice_id
                    )));
                }
                Some(_) => {}
                None => voices.push(DialogueVoice {
                    voice_id: voice_id.clone(),
                    name: turn.speaker.clone(),
                }),
            }
        }

        if let Some(unmapped) = turns
            .iter()
            .find(|t| !voices.iter().any(|v| v.name == t.speaker))
        {
            return Err(local_invalid_request(format!(
                "speaker {:?} has no voice on any turn",
                unmapped.speaker
            )));
        }

        Ok(Self {
            text,
            voices,
            model,
            ..Default::default()
        })
    }
}

/// An error raised by the SDK before any request is sent. Status 0 marks it
/// as local; the code matches what the gateway would have answered.
fn local_invalid_request(message: String) -> Error {
    Error::Api(ApiError {
        status_code: 0,
        code: "invalid_request".into(),
        message,
        request_id: String::new(),
    })
}

/// Response from dialogue generation.
#[derive(Debug, Clone, Deserialize)]
pub struct DialogueResponse {
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
// Speech-to-speech, isolate, remix, dub, align
// ---------------------------------------------------------------------------

/// Request body for speech-to-speech conversion (ElevenLabs).
///
/// The gateway fixes the model to `eleven_multilingual_v2` and the provider
/// default output format; neither can be chosen per request.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SpeechToSpeechRequest {
    /// Target voice identifier (required).
    pub voice_id: String,

    /// Base64-encoded source audio (required).
    pub audio_base64: String,
}

/// Response from speech-to-speech conversion.
#[derive(Debug, Clone, Deserialize)]
pub struct SpeechToSpeechResponse {
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

/// Request body for voice isolation. The output format is the provider
/// default and cannot be chosen.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IsolateVoiceRequest {
    /// Base64-encoded audio to isolate voice from.
    pub audio_base64: String,

    /// Original filename (helps detect the container format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Backwards-compatible alias.
pub type IsolateRequest = IsolateVoiceRequest;

/// Response from voice isolation.
#[derive(Debug, Clone, Deserialize)]
pub struct IsolateVoiceResponse {
    /// Base64-encoded audio data.
    pub audio_base64: String,

    /// Audio format (e.g. "mp3").
    pub format: String,

    /// File size in bytes.
    #[serde(default)]
    pub size_bytes: i64,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for voice remixing (ElevenLabs voice remix). Only
/// `audio_base64` is required; the remaining knobs steer the remix and are
/// forwarded as-is. Billed flat per request.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RemixVoiceRequest {
    /// Base64-encoded source audio (required).
    pub audio_base64: String,

    /// Original filename (helps detect the container format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Target gender for the remixed voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,

    /// Target accent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,

    /// Target speaking style.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    /// Target pacing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pacing: Option<String>,

    /// Audio quality setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_quality: Option<String>,

    /// How strongly the attributes steer the remix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_strength: Option<String>,

    /// Script to speak in the remixed voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

/// Backwards-compatible alias.
pub type RemixRequest = RemixVoiceRequest;

/// Response from voice remixing.
#[derive(Debug, Clone, Deserialize)]
pub struct RemixVoiceResponse {
    /// Base64-encoded audio data (absent when the provider returned none).
    #[serde(default)]
    pub audio_base64: Option<String>,

    /// Audio format.
    pub format: String,

    /// File size in bytes.
    #[serde(default)]
    pub size_bytes: i64,

    /// Identifier of the remixed voice, when the provider created one.
    #[serde(default)]
    pub voice_id: Option<String>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for audio dubbing.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DubRequest {
    /// Base64-encoded source audio or video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_base64: Option<String>,

    /// Original filename (helps detect format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// URL to source media (alternative to audio_base64).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,

    /// Target language (BCP-47 code, e.g. "es", "de").
    pub target_lang: String,

    /// Source language (auto-detected if omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_lang: Option<String>,

    /// Number of speakers (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_speakers: Option<i32>,

    /// Enable highest quality processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highest_resolution: Option<bool>,
}

/// Response from dubbing.
#[derive(Debug, Clone, Deserialize)]
pub struct DubResponse {
    /// Provider dubbing job identifier.
    pub dubbing_id: String,

    /// Base64-encoded dubbed audio.
    pub audio_base64: String,

    /// Audio format.
    pub format: String,

    /// Target language echoed back.
    #[serde(default)]
    pub target_lang: String,

    /// Provider status of the dub.
    #[serde(default)]
    pub status: String,

    /// Provider-side processing time.
    #[serde(default)]
    pub processing_time_seconds: f64,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for audio alignment / forced alignment.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AlignRequest {
    /// Base64-encoded audio data.
    pub audio_base64: String,

    /// Original filename (helps detect the container format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Transcript text to align against the audio.
    pub text: String,

    /// Language code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// A single word with timing information from forced alignment.
#[derive(Debug, Clone, Deserialize)]
pub struct AlignedWord {
    /// Word text.
    pub text: String,

    /// Start time in seconds.
    pub start_time: f64,

    /// End time in seconds.
    pub end_time: f64,

    /// Alignment confidence score.
    #[serde(default)]
    pub confidence: f64,
}

/// Response from audio alignment. Only word-level timings exist on the
/// wire; there is no segment level.
#[derive(Debug, Clone, Deserialize)]
pub struct AlignResponse {
    /// Word-level alignment.
    #[serde(default, deserialize_with = "null_as_default")]
    pub alignment: Vec<AlignedWord>,

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
// Voice design
// ---------------------------------------------------------------------------

/// Request body for voice design (generating a voice from a description).
/// The preview format is the provider default and cannot be chosen.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VoiceDesignRequest {
    /// Text description of the desired voice.
    #[serde(rename = "voice_description")]
    pub description: String,

    /// Sample text to speak with the designed voice.
    #[serde(rename = "sample_text")]
    pub text: String,
}

/// Response from voice design: several candidate voices, each with a
/// preview clip. Save the chosen `generated_voice_id` to keep it.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceDesignResponse {
    /// Candidate voices.
    #[serde(default, deserialize_with = "null_as_default")]
    pub previews: Vec<VoicePreview>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// A single voice preview from voice design.
#[derive(Debug, Clone, Deserialize)]
pub struct VoicePreview {
    /// Identifier of the candidate voice.
    pub generated_voice_id: String,

    /// Base64-encoded preview audio.
    pub audio_base64: String,

    /// Audio format of the preview.
    pub format: String,
}

// ---------------------------------------------------------------------------
// Starfish TTS (HeyGen)
// ---------------------------------------------------------------------------

/// Request body for Starfish TTS. The output format is the provider default
/// and cannot be chosen.
#[derive(Debug, Clone, Serialize, Default)]
pub struct StarfishTTSRequest {
    /// Text to synthesise (required).
    pub text: String,

    /// HeyGen voice identifier (required).
    pub voice_id: String,

    /// Input type (e.g. "text", "ssml").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,

    /// Speech speed multiplier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,

    /// BCP-47 language code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Locale code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// Response from Starfish TTS. The audio arrives as `audio_base64` or as a
/// `url`, whichever the provider returned.
#[derive(Debug, Clone, Deserialize)]
pub struct StarfishTTSResponse {
    /// Base64-encoded audio data.
    #[serde(default)]
    pub audio_base64: Option<String>,

    /// URL of the rendered audio.
    #[serde(default)]
    pub url: Option<String>,

    /// Audio format.
    pub format: String,

    /// File size in bytes.
    #[serde(default)]
    pub size_bytes: i64,

    /// Duration in seconds.
    #[serde(default)]
    pub duration: f64,

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
// Eleven Music (advanced music generation with sections and finetunes)
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
/// Either `prompt` or `sections` is required.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ElevenMusicRequest {
    /// Music model (default "music_v1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Describes the music to generate.
    pub prompt: String,

    /// Structured sections (verse, chorus, …) with lyrics and style.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<MusicSection>>,

    /// Target duration in seconds (default 30). Cost scales with duration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,

    /// Accepted by the gateway but not forwarded to the provider yet; steer
    /// language through the prompt or lyrics instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Include vocals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocals: Option<bool>,

    /// Global style.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    /// Styles to avoid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_exclude: Option<String>,

    /// Finetune to generate with (see [`Client::list_finetunes`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finetune_id: Option<String>,
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
#[derive(Debug, Clone, Deserialize)]
pub struct ElevenMusicResponse {
    /// Generated music clips.
    #[serde(default, deserialize_with = "null_as_default")]
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

/// A music finetune as the gateway reports it. Creation returns `id` and
/// `status` only; `model_id` appears once training has produced a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinetuneInfo {
    /// Finetune identifier.
    pub id: String,

    /// Training status.
    #[serde(default)]
    pub status: String,

    /// Model id to pass as `finetune_id`, once available.
    #[serde(default)]
    pub model_id: Option<String>,
}

/// Response from listing finetunes.
#[derive(Debug, Clone, Deserialize)]
pub struct ListFinetunesResponse {
    /// Finetunes on the account.
    #[serde(default, deserialize_with = "null_as_default")]
    pub finetunes: Vec<FinetuneInfo>,
}

/// Request body for creating a music finetune.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MusicFinetuneCreateRequest {
    /// Display name (required).
    pub name: String,

    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Base64-encoded training audio files (at least one).
    pub samples: Vec<String>,
}

// ---------------------------------------------------------------------------
// HeyGen sounds search (background music + sound effects)
// ---------------------------------------------------------------------------

/// Query parameters for searching the sounds catalog.
#[derive(Debug, Clone, Default)]
pub struct AudioSoundsQuery {
    /// Natural-language description of the sound wanted (required).
    pub query: String,

    /// Catalog to search: "music" | "sound_effects" (API default: "music").
    /// Wire param: `type`.
    pub sound_type: Option<String>,

    /// Max results, 1–50 (API default 10).
    pub limit: Option<i32>,

    /// Minimum similarity score, 0–1 (API default 0.7).
    pub min_score: Option<f64>,

    /// Opaque cursor from a previous response's `next_token`.
    pub token: Option<String>,
}

/// A track from the sounds catalog.
#[derive(Debug, Clone, Deserialize)]
pub struct AudioSound {
    /// Track identifier.
    pub id: String,

    /// Track name.
    #[serde(default)]
    pub name: String,

    /// Track description.
    #[serde(default)]
    pub description: String,

    /// Pre-signed WAV URL with a limited lifetime — download promptly,
    /// do not cache.
    #[serde(default)]
    pub audio_url: String,

    /// Duration in seconds.
    #[serde(default)]
    pub duration: f64,

    /// Similarity score 0–1 (best first).
    #[serde(default)]
    pub score: f64,

    /// "music" | "sound_effects". Wire field: `type`.
    #[serde(rename = "type", default)]
    pub sound_type: String,
}

/// Response from searching the sounds catalog (unbilled).
#[derive(Debug, Clone, Deserialize)]
pub struct AudioSoundsResponse {
    /// Matching tracks, best score first (empty page → `[]`).
    #[serde(default, deserialize_with = "null_as_default")]
    pub sounds: Vec<AudioSound>,

    /// More pages exist.
    #[serde(default)]
    pub has_more: bool,

    /// Pass as `token` for the next page (may be empty).
    #[serde(default)]
    pub next_token: String,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Response from `POST /qai/v1/audio/stt/realtime-token`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RealtimeSttTokenResponse {
    /// Realtime credential. Pass it as the `token` query parameter on the
    /// WebSocket connect.
    #[serde(default)]
    pub token: String,

    /// Token lifetime in seconds (900 — 15 minutes).
    #[serde(default)]
    pub expires_in: i64,

    /// WebSocket endpoint the token authenticates against.
    #[serde(default)]
    pub ws_endpoint: String,

    /// Ticks charged for the session estimate.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Gateway request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// Client impl
// ---------------------------------------------------------------------------

/// Fills `cost_ticks` and `request_id` from the response headers when the
/// body left them empty.
macro_rules! backfill_meta {
    ($resp:expr, $meta:expr) => {{
        if $resp.cost_ticks == 0 {
            $resp.cost_ticks = $meta.cost_ticks;
        }
        if $resp.request_id.is_empty() {
            $resp.request_id = $meta.request_id;
        }
    }};
}

impl Client {
    /// Searches HeyGen's background-music and sound-effects catalogs
    /// (semantic ranking, best score first). Unbilled catalog route.
    pub async fn search_audio_sounds(
        &self,
        query: &AudioSoundsQuery,
    ) -> Result<AudioSoundsResponse> {
        let mut params = vec![format!("query={}", urlencoding::encode(&query.query))];
        if let Some(ref t) = query.sound_type {
            params.push(format!("type={}", urlencoding::encode(t)));
        }
        if let Some(limit) = query.limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(min_score) = query.min_score {
            params.push(format!("min_score={min_score}"));
        }
        if let Some(ref token) = query.token {
            params.push(format!("token={}", urlencoding::encode(token)));
        }
        let path = format!("/qai/v1/audio/sounds?{}", params.join("&"));
        let (resp, _meta) = self.get_json::<AudioSoundsResponse>(&path).await?;
        Ok(resp)
    }

    /// Generates speech from text.
    pub async fn speak(&self, req: &TextToSpeechRequest) -> Result<TextToSpeechResponse> {
        let (mut resp, meta) = self
            .post_json::<TextToSpeechRequest, TextToSpeechResponse>("/qai/v1/audio/tts", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Converts speech to text.
    pub async fn transcribe(&self, req: &SpeechToTextRequest) -> Result<SpeechToTextResponse> {
        let (mut resp, meta) = self
            .post_json::<SpeechToTextRequest, SpeechToTextResponse>("/qai/v1/audio/stt", req)
            .await?;
        backfill_meta!(resp, meta);
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
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Generates music from a text prompt.
    pub async fn generate_music(&self, req: &MusicRequest) -> Result<MusicResponse> {
        let (mut resp, meta) = self
            .post_json::<MusicRequest, MusicResponse>("/qai/v1/audio/music", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Generates multi-speaker dialogue audio (billed per script character).
    pub async fn dialogue(&self, req: &DialogueRequest) -> Result<DialogueResponse> {
        let (mut resp, meta) = self
            .post_json::<DialogueRequest, DialogueResponse>("/qai/v1/audio/dialogue", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Converts speech to a different voice.
    pub async fn speech_to_speech(
        &self,
        req: &SpeechToSpeechRequest,
    ) -> Result<SpeechToSpeechResponse> {
        let (mut resp, meta) = self
            .post_json::<SpeechToSpeechRequest, SpeechToSpeechResponse>(
                "/qai/v1/audio/speech-to-speech",
                req,
            )
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Isolates voice from background noise and music.
    pub async fn isolate_voice(&self, req: &IsolateVoiceRequest) -> Result<IsolateVoiceResponse> {
        let (mut resp, meta) = self
            .post_json::<IsolateVoiceRequest, IsolateVoiceResponse>("/qai/v1/audio/isolate", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Remixes a voice recording towards the requested attributes (flat
    /// per-request charge).
    pub async fn remix_voice(&self, req: &RemixVoiceRequest) -> Result<RemixVoiceResponse> {
        let (mut resp, meta) = self
            .post_json::<RemixVoiceRequest, RemixVoiceResponse>("/qai/v1/audio/remix", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Dubs audio or video into a target language.
    pub async fn dub(&self, req: &DubRequest) -> Result<DubResponse> {
        let (mut resp, meta) = self
            .post_json::<DubRequest, DubResponse>("/qai/v1/audio/dub", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Performs forced alignment of text against audio.
    pub async fn align(&self, req: &AlignRequest) -> Result<AlignResponse> {
        let (mut resp, meta) = self
            .post_json::<AlignRequest, AlignResponse>("/qai/v1/audio/align", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Designs voices from a text description; each preview carries a
    /// `generated_voice_id` and a sample clip.
    pub async fn voice_design(&self, req: &VoiceDesignRequest) -> Result<VoiceDesignResponse> {
        let (mut resp, meta) = self
            .post_json::<VoiceDesignRequest, VoiceDesignResponse>("/qai/v1/audio/voice-design", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Generates speech using Starfish TTS (HeyGen).
    pub async fn starfish_tts(&self, req: &StarfishTTSRequest) -> Result<StarfishTTSResponse> {
        let (mut resp, meta) = self
            .post_json::<StarfishTTSRequest, StarfishTTSResponse>("/qai/v1/audio/starfish-tts", req)
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Generates music via ElevenLabs Eleven Music (sections, finetunes).
    /// Editing an earlier generation is not supported on this route.
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
        backfill_meta!(resp, meta);
        Ok(resp)
    }

    /// Lists the music finetunes on the ElevenLabs account.
    pub async fn list_finetunes(&self) -> Result<ListFinetunesResponse> {
        let (resp, _) = self
            .get_json::<ListFinetunesResponse>("/qai/v1/audio/finetunes")
            .await?;
        Ok(resp)
    }

    /// Creates a music finetune from base64 audio samples. Answers 201 with
    /// the finetune's `id` and initial `status`; training is asynchronous,
    /// so `model_id` is empty until [`Client::list_finetunes`] reports it.
    pub async fn create_finetune(&self, req: &MusicFinetuneCreateRequest) -> Result<FinetuneInfo> {
        let (resp, _) = self
            .post_json::<MusicFinetuneCreateRequest, FinetuneInfo>("/qai/v1/audio/finetunes", req)
            .await?;
        Ok(resp)
    }

    /// Deletes a music finetune by ID (`{"status":"deleted"}`; 404 when
    /// unknown, 403 when owned by another account).
    pub async fn delete_finetune(&self, id: &str) -> Result<serde_json::Value> {
        let path = format!("/qai/v1/audio/finetunes/{id}");
        let (resp, _) = self.delete_json::<serde_json::Value>(&path).await?;
        Ok(resp)
    }

    /// Mints a token for a realtime speech-to-text WebSocket.
    ///
    /// The client then connects straight to
    /// [`RealtimeSttTokenResponse::ws_endpoint`] with `?token=<token>` — the
    /// gateway brokers only the credential, so there is no proxy hop. The
    /// gateway bills a flat per-session estimate when the token is minted and
    /// reports a 15-minute TTL; ElevenLabs treats the token as single-use,
    /// which the gateway neither enforces nor observes.
    ///
    /// `POST /qai/v1/audio/stt/realtime-token`
    pub async fn audio_stt_realtime_token(&self) -> Result<RealtimeSttTokenResponse> {
        let (mut resp, meta) = self
            .post_json_empty::<RealtimeSttTokenResponse>("/qai/v1/audio/stt/realtime-token")
            .await?;
        backfill_meta!(resp, meta);
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(value: &serde_json::Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().expect("object").keys().cloned().collect();
        keys.sort();
        keys
    }

    #[test]
    fn token_response_carries_the_direct_endpoint() {
        let resp: RealtimeSttTokenResponse = serde_json::from_str(
            r#"{"token":"tok_abc","expires_in":900,
                "ws_endpoint":"wss://api.elevenlabs.io/v1/speech-to-text/realtime",
                "cost_ticks":6000000,"request_id":"req_1"}"#,
        )
        .expect("decode");
        assert_eq!(resp.expires_in, 900);
        assert!(resp.ws_endpoint.starts_with("wss://"));
        assert_eq!(resp.cost_ticks, 6_000_000);
    }

    #[test]
    fn remix_request_sends_the_gateway_knobs_only() {
        let req = RemixVoiceRequest {
            audio_base64: "AQID".into(),
            filename: Some("in.mp3".into()),
            gender: Some("female".into()),
            accent: Some("british".into()),
            style: Some("calm".into()),
            pacing: Some("slow".into()),
            audio_quality: Some("high".into()),
            prompt_strength: Some("strong".into()),
            script: Some("Hello".into()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(
            keys(&json),
            [
                "accent",
                "audio_base64",
                "audio_quality",
                "filename",
                "gender",
                "pacing",
                "prompt_strength",
                "script",
                "style",
            ]
        );
    }

    #[test]
    fn remix_response_decodes_without_audio() {
        // remixHTTPResponse omits audio_base64 and voice_id when empty.
        let resp: RemixVoiceResponse = serde_json::from_str(
            r#"{"format":"mp3","size_bytes":0,"cost_ticks":3000000000,"request_id":"req_r",
                "provenance":{"model":"eleven_voice_remix"}}"#,
        )
        .unwrap();
        assert!(resp.audio_base64.is_none());
        assert!(resp.voice_id.is_none());
        assert_eq!(resp.cost_ticks, 3_000_000_000);
    }

    #[test]
    fn speech_to_speech_and_starfish_send_voice_id_only() {
        let json = serde_json::to_value(SpeechToSpeechRequest {
            voice_id: "v1".into(),
            audio_base64: "AQID".into(),
        })
        .unwrap();
        assert_eq!(keys(&json), ["audio_base64", "voice_id"]);

        let json = serde_json::to_value(StarfishTTSRequest {
            text: "hi".into(),
            voice_id: "hv1".into(),
            speed: Some(1.1),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(keys(&json), ["speed", "text", "voice_id"]);
    }

    #[test]
    fn isolate_and_voice_design_send_no_format() {
        let json = serde_json::to_value(IsolateVoiceRequest {
            audio_base64: "AQID".into(),
            filename: Some("noisy.wav".into()),
        })
        .unwrap();
        assert_eq!(keys(&json), ["audio_base64", "filename"]);

        let json = serde_json::to_value(VoiceDesignRequest {
            description: "warm baritone".into(),
            text: "Sample line".into(),
        })
        .unwrap();
        assert_eq!(keys(&json), ["sample_text", "voice_description"]);
    }

    #[test]
    fn voice_design_response_exposes_previews() {
        let resp: VoiceDesignResponse = serde_json::from_str(
            r#"{"previews":[{"generated_voice_id":"gv1","audio_base64":"AQID","format":"mp3"}],
                "cost_ticks":10,"request_id":"req_v"}"#,
        )
        .unwrap();
        assert_eq!(resp.previews.len(), 1);
        assert_eq!(resp.previews[0].generated_voice_id, "gv1");

        let empty: VoiceDesignResponse =
            serde_json::from_str(r#"{"previews":null,"cost_ticks":0,"request_id":""}"#).unwrap();
        assert!(empty.previews.is_empty());
    }

    #[test]
    fn typed_audio_responses_decode_handler_shapes() {
        let d: DialogueResponse = serde_json::from_str(
            r#"{"audio_base64":"AQID","format":"mp3","size_bytes":3,"model":"eleven_v3",
                "cost_ticks":1,"request_id":"req_d","provenance":{}}"#,
        )
        .unwrap();
        assert_eq!(d.model, "eleven_v3");

        let s: SpeechToSpeechResponse = serde_json::from_str(
            r#"{"audio_base64":"AQID","format":"mp3","size_bytes":3,
                "model":"eleven_multilingual_v2","cost_ticks":1,"request_id":"req_s"}"#,
        )
        .unwrap();
        assert_eq!(s.size_bytes, 3);

        let i: IsolateVoiceResponse = serde_json::from_str(
            r#"{"audio_base64":"AQID","format":"mp3","size_bytes":3,"cost_ticks":1,
                "request_id":"req_i"}"#,
        )
        .unwrap();
        assert_eq!(i.request_id, "req_i");

        let dub: DubResponse = serde_json::from_str(
            r#"{"dubbing_id":"dub1","audio_base64":"AQID","format":"mp3","target_lang":"es",
                "status":"dubbed","processing_time_seconds":12.5,"cost_ticks":1,
                "request_id":"req_x"}"#,
        )
        .unwrap();
        assert_eq!(dub.dubbing_id, "dub1");

        let st: StarfishTTSResponse = serde_json::from_str(
            r#"{"format":"mp3","size_bytes":10,"duration":1.5,"model":"heygen-starfish",
                "cost_ticks":1,"request_id":"req_st","url":"https://x/a.mp3"}"#,
        )
        .unwrap();
        assert_eq!(st.url.as_deref(), Some("https://x/a.mp3"));
        assert!(st.audio_base64.is_none());
    }

    #[test]
    fn align_response_is_word_level_only() {
        let resp: AlignResponse = serde_json::from_str(
            r#"{"alignment":[{"text":"hi","start_time":0.0,"end_time":0.4,"confidence":0.9}],
                "model":"scribe","cost_ticks":1,"request_id":"req_a"}"#,
        )
        .unwrap();
        assert_eq!(resp.alignment.len(), 1);
        assert_eq!(resp.alignment[0].end_time, 0.4);
    }

    #[test]
    fn music_advanced_request_has_no_edit_fields_and_optional_model() {
        let json = serde_json::to_value(ElevenMusicRequest {
            prompt: "lofi".into(),
            duration_seconds: Some(60),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(keys(&json), ["duration_seconds", "prompt"]);

        let resp: ElevenMusicResponse = serde_json::from_str(
            r#"{"clips":null,"model":"music_v1","cost_ticks":0,"request_id":"r","provenance":{}}"#,
        )
        .unwrap();
        assert!(resp.clips.is_empty());
    }

    #[test]
    fn finetune_shapes_match_elevenlabs_status() {
        let list: ListFinetunesResponse = serde_json::from_str(
            r#"{"finetunes":[{"id":"ft_1","status":"training"},
                {"id":"ft_2","status":"ready","model_id":"music_v1_ft_2"}]}"#,
        )
        .unwrap();
        assert_eq!(list.finetunes.len(), 2);
        assert_eq!(list.finetunes[1].model_id.as_deref(), Some("music_v1_ft_2"));

        let created: FinetuneInfo =
            serde_json::from_str(r#"{"id":"ft_3","status":"queued"}"#).unwrap();
        assert_eq!(created.id, "ft_3");
        assert!(created.model_id.is_none());

        let json = serde_json::to_value(MusicFinetuneCreateRequest {
            name: "mine".into(),
            description: None,
            samples: vec!["AQID".into()],
        })
        .unwrap();
        assert_eq!(keys(&json), ["name", "samples"]);
    }

    #[test]
    fn from_turns_maps_every_speaker() {
        let req = DialogueRequest::from_turns(
            vec![
                DialogueTurn {
                    speaker: "A".into(),
                    text: "Hi".into(),
                    voice: None,
                },
                DialogueTurn {
                    speaker: "B".into(),
                    text: "Hello".into(),
                    voice: Some("vb".into()),
                },
                DialogueTurn {
                    speaker: "A".into(),
                    text: "Bye".into(),
                    voice: Some("va".into()),
                },
            ],
            None,
        )
        .unwrap();
        assert_eq!(req.text, "A: Hi\nB: Hello\nA: Bye");
        assert_eq!(req.voices.len(), 2);
        assert_eq!(req.voices[0].name, "B");
        assert_eq!(req.voices[1].voice_id, "va");
    }

    #[test]
    fn from_turns_rejects_unmapped_and_conflicting_speakers() {
        let unmapped = DialogueRequest::from_turns(
            vec![DialogueTurn {
                speaker: "A".into(),
                text: "Hi".into(),
                voice: None,
            }],
            None,
        );
        match unmapped {
            Err(Error::Api(e)) => {
                assert_eq!(e.code, "invalid_request");
                assert_eq!(e.status_code, 0);
            }
            other => panic!("expected local invalid_request, got {other:?}"),
        }

        let conflict = DialogueRequest::from_turns(
            vec![
                DialogueTurn {
                    speaker: "A".into(),
                    text: "Hi".into(),
                    voice: Some("v1".into()),
                },
                DialogueTurn {
                    speaker: "A".into(),
                    text: "Again".into(),
                    voice: Some("v2".into()),
                },
            ],
            None,
        );
        assert!(matches!(conflict, Err(Error::Api(e)) if e.message.contains("v2")));
    }
}
