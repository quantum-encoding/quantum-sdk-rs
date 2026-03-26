use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Request body for video generation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoRequest {
    /// Video generation model (e.g. "heygen", "grok-imagine-video", "sora-2", "veo-2").
    pub model: String,

    /// Describes the video to generate.
    pub prompt: String,

    /// Target video duration in seconds (default 8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,

    /// Video aspect ratio (e.g. "16:9", "9:16").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
}

/// Response from video generation.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoResponse {
    /// Generated videos.
    pub videos: Vec<GeneratedVideo>,

    /// Model that generated the videos.
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// A single generated video.
#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedVideo {
    /// Base64-encoded video data (or a URL).
    pub base64: String,

    /// Video format (e.g. "mp4").
    pub format: String,

    /// Video file size.
    pub size_bytes: i64,

    /// Video index within the batch.
    pub index: i32,
}

// ---------------------------------------------------------------------------
// Job response (shared by HeyGen endpoints)
// ---------------------------------------------------------------------------

/// Response from async video job submission.
#[derive(Debug, Clone, Deserialize)]
pub struct JobResponse {
    /// Job identifier for polling status.
    pub job_id: String,

    /// Current status.
    #[serde(default)]
    pub status: String,

    /// Total cost in ticks (may be 0 until job completes).
    #[serde(default)]
    pub cost_ticks: i64,

    /// Additional response fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// HeyGen Studio
// ---------------------------------------------------------------------------

/// A clip in a studio video.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StudioClip {
    /// Avatar ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_id: Option<String>,

    /// Voice ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,

    /// Script text for this clip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,

    /// Background settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<serde_json::Value>,
}

/// Request body for HeyGen studio video creation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoStudioRequest {
    /// Video title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Video clips.
    pub clips: Vec<StudioClip>,

    /// Video dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,

    /// Aspect ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
}

// ---------------------------------------------------------------------------
// HeyGen Translate
// ---------------------------------------------------------------------------

/// Backwards-compatible alias.
pub type StudioVideoRequest = VideoStudioRequest;

/// Request body for video translation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoTranslateRequest {
    /// URL of the video to translate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,

    /// Base64-encoded video (alternative to URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_base64: Option<String>,

    /// Target language code.
    pub target_language: String,

    /// Source language code (auto-detected if omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_language: Option<String>,
}

/// Backwards-compatible alias.
pub type TranslateRequest = VideoTranslateRequest;

// ---------------------------------------------------------------------------
// HeyGen Photo Avatar
// ---------------------------------------------------------------------------

/// Request body for creating a photo avatar video.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PhotoAvatarRequest {
    /// Base64-encoded photo.
    pub photo_base64: String,

    /// Script text for the avatar to speak.
    pub script: String,

    /// Voice ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,

    /// Aspect ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
}

// ---------------------------------------------------------------------------
// HeyGen Digital Twin
// ---------------------------------------------------------------------------

/// Request body for digital twin video generation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DigitalTwinRequest {
    /// Digital twin / avatar ID.
    pub avatar_id: String,

    /// Script text.
    pub script: String,

    /// Voice ID (uses twin's default voice if omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,

    /// Aspect ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
}

// ---------------------------------------------------------------------------
// HeyGen Avatars
// ---------------------------------------------------------------------------

/// A HeyGen avatar.
#[derive(Debug, Clone, Deserialize)]
pub struct Avatar {
    /// Avatar identifier.
    pub avatar_id: String,

    /// Avatar name.
    #[serde(default)]
    pub name: Option<String>,

    /// Avatar gender.
    #[serde(default)]
    pub gender: Option<String>,

    /// Preview image URL.
    #[serde(default)]
    pub preview_url: Option<String>,

    /// Additional fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Response from listing HeyGen avatars.
#[derive(Debug, Clone, Deserialize)]
pub struct AvatarsResponse {
    pub avatars: Vec<Avatar>,
}

// ---------------------------------------------------------------------------
// HeyGen Templates
// ---------------------------------------------------------------------------

/// A HeyGen video template.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoTemplate {
    /// Template identifier.
    pub template_id: String,

    /// Template name.
    #[serde(default)]
    pub name: Option<String>,

    /// Preview image URL.
    #[serde(default)]
    pub preview_url: Option<String>,

    /// Additional fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Response from listing HeyGen video templates.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoTemplatesResponse {
    pub templates: Vec<VideoTemplate>,
}

// ---------------------------------------------------------------------------
// HeyGen Voices
// ---------------------------------------------------------------------------

/// A HeyGen voice.
#[derive(Debug, Clone, Deserialize)]
pub struct HeyGenVoice {
    /// Voice identifier.
    pub voice_id: String,

    /// Voice name.
    #[serde(default)]
    pub name: Option<String>,

    /// Language.
    #[serde(default)]
    pub language: Option<String>,

    /// Gender.
    #[serde(default)]
    pub gender: Option<String>,

    /// Additional fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Response from listing HeyGen voices.
#[derive(Debug, Clone, Deserialize)]
pub struct HeyGenVoicesResponse {
    pub voices: Vec<HeyGenVoice>,
}

// ---------------------------------------------------------------------------
// Client impl
// ---------------------------------------------------------------------------

impl Client {
    /// Generates a video from a text prompt.
    ///
    /// Video generation is slow (30s-5min). For production use, consider submitting
    /// via the Jobs API instead.
    pub async fn generate_video(&self, req: &VideoRequest) -> Result<VideoResponse> {
        let (mut resp, meta) = self
            .post_json::<VideoRequest, VideoResponse>("/qai/v1/video/generate", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Creates a HeyGen studio video from clips.
    pub async fn video_studio(&self, req: &VideoStudioRequest) -> Result<JobResponse> {
        let (resp, _meta) = self
            .post_json::<VideoStudioRequest, JobResponse>("/qai/v1/video/studio", req)
            .await?;
        Ok(resp)
    }

    /// Translates a video into another language (HeyGen).
    pub async fn video_translate(&self, req: &VideoTranslateRequest) -> Result<JobResponse> {
        let (resp, _meta) = self
            .post_json::<VideoTranslateRequest, JobResponse>("/qai/v1/video/translate", req)
            .await?;
        Ok(resp)
    }

    /// Creates a video from a photo avatar (HeyGen).
    pub async fn video_photo_avatar(&self, req: &PhotoAvatarRequest) -> Result<JobResponse> {
        let (resp, _meta) = self
            .post_json::<PhotoAvatarRequest, JobResponse>("/qai/v1/video/photo-avatar", req)
            .await?;
        Ok(resp)
    }

    /// Creates a video from a digital twin avatar (HeyGen).
    pub async fn video_digital_twin(&self, req: &DigitalTwinRequest) -> Result<JobResponse> {
        let (resp, _meta) = self
            .post_json::<DigitalTwinRequest, JobResponse>("/qai/v1/video/digital-twin", req)
            .await?;
        Ok(resp)
    }

    /// Lists available HeyGen avatars.
    pub async fn video_avatars(&self) -> Result<AvatarsResponse> {
        let (resp, _meta) = self
            .get_json::<AvatarsResponse>("/qai/v1/video/avatars")
            .await?;
        Ok(resp)
    }

    /// Lists available HeyGen video templates.
    pub async fn video_templates(&self) -> Result<VideoTemplatesResponse> {
        let (resp, _meta) = self
            .get_json::<VideoTemplatesResponse>("/qai/v1/video/templates")
            .await?;
        Ok(resp)
    }

    /// Lists available HeyGen voices.
    pub async fn video_heygen_voices(&self) -> Result<HeyGenVoicesResponse> {
        let (resp, _meta) = self
            .get_json::<HeyGenVoicesResponse>("/qai/v1/video/heygen-voices")
            .await?;
        Ok(resp)
    }
}
