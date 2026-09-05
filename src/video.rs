use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::jobs::JobAcceptedResponse;
use crate::serde_util::null_as_default;

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

    /// Catalog-schema-driven extra parameters (resolution, sample_count,
    /// negative_prompt, person_generation, generate_audio, …). Flattened to
    /// top-level JSON so any param the backend's /qai/v1/videos accepts is
    /// forwarded without a typed field. Empty map serializes to nothing.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
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

    /// Credit balance in ticks after this request was charged.
    #[serde(default)]
    pub balance_after: i64,

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

/// Backwards-compatible alias.
pub type StudioVideoRequest = VideoStudioRequest;

// ---------------------------------------------------------------------------
// HeyGen Translate
// ---------------------------------------------------------------------------

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
// HeyGen typed responses (with request_id)
// ---------------------------------------------------------------------------

/// Response from listing HeyGen avatars (includes request_id).
#[derive(Debug, Clone, Deserialize)]
pub struct HeyGenAvatarsResponse {
    /// Available avatars (raw JSON items).
    #[serde(default, deserialize_with = "null_as_default")]
    pub avatars: Vec<serde_json::Value>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Response from listing HeyGen templates (includes request_id).
#[derive(Debug, Clone, Deserialize)]
pub struct HeyGenTemplatesResponse {
    /// Available templates (raw JSON items).
    #[serde(default)]
    pub templates: Vec<serde_json::Value>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// HeyGen Template v3 (variable schema + render)
// ---------------------------------------------------------------------------

/// A variable slot referenced by a template scene.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoTemplateSceneVariable {
    /// Variable name (key into the template's `variables` map).
    pub name: String,

    /// Variable kind (e.g. "text", "image", "character", "voice").
    #[serde(default)]
    pub variable_type: String,
}

/// A scene in a template, in template order.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoTemplateScene {
    /// Scene identifier (usable in a generate request's `scene_ids`).
    pub scene_id: String,

    /// Scene script with placeholders unreplaced (e.g. "Introducing {{headline}}...").
    #[serde(default)]
    pub script: String,

    /// Variables referenced by this scene.
    #[serde(default, deserialize_with = "null_as_default")]
    pub variables: Vec<VideoTemplateSceneVariable>,
}

/// Detailed template info: variable schema + scenes.
///
/// Each `variables[name]` value is a discriminated union on its `"type"` field
/// ("text" | "image" | "video" | "audio" | "voice" | "character"; unknown
/// future types round-trip verbatim), returned in the exact shape a generate
/// request accepts — replace defaults and submit back.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoTemplateDetail {
    /// Template identifier.
    pub id: String,

    /// Template name.
    #[serde(default)]
    pub name: String,

    /// Aspect ratio (e.g. "16:9").
    #[serde(default)]
    pub aspect_ratio: String,

    /// Variable schema keyed by variable name (union values kept as raw JSON
    /// so unknown future variable types round-trip verbatim).
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,

    /// Scenes in template order.
    #[serde(default, deserialize_with = "null_as_default")]
    pub scenes: Vec<VideoTemplateScene>,
}

/// Response from inspecting a template's variable schema (unbilled).
#[derive(Debug, Clone, Deserialize)]
pub struct VideoTemplateDetailResponse {
    /// The template detail.
    pub template: VideoTemplateDetail,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Output dimension for a template render. Both values must be even,
/// each 128–4096, and keep the template aspect ratio.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoTemplateDimension {
    pub width: i32,
    pub height: i32,
}

/// Subtitle position for burned-in captions.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoSubtitlePosition {
    pub x: f64,
    pub y: f64,
}

/// Subtitle options for a template render (implies captions).
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoTemplateSubtitles {
    /// Subtitle preset (e.g. "classic", "bold", "bright"). Required.
    pub preset_name: String,

    /// Alignment (default 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<i32>,

    /// Disable word highlighting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_highlight: Option<bool>,

    /// Font size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<i32>,

    /// Subtitle position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<VideoSubtitlePosition>,
}

/// Request body for rendering a video from a template (async job).
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoTemplateGenerateRequest {
    /// Variable overrides keyed by name (at least one required). Values use
    /// the same union shapes returned by the template detail route; omitted
    /// variables keep the template defaults.
    pub variables: HashMap<String, serde_json::Value>,

    /// Names the generated video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Restrict the render to these scenes, in order (repeats allowed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_ids: Option<Vec<String>>,

    /// Output dimension (must keep the template aspect ratio).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<VideoTemplateDimension>,

    /// Frames per second: 25 (default), 30, or 60.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<i32>,

    /// Burn captions (default false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<bool>,

    /// Subtitle options (implies captions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitles: Option<VideoTemplateSubtitles>,

    /// Background audio moves with scenes (default true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reorder_music: Option<bool>,

    /// Keep text vertically centered (default false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_text_vertically_centered: Option<bool>,

    /// Include a GIF preview in the webhook payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_gif: Option<bool>,

    /// Enable a public share page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_sharing: Option<bool>,

    /// HeyGen folder id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,

    /// Brand voice id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand_voice_id: Option<String>,
}

// ---------------------------------------------------------------------------
// HeyGen Batch videos
// ---------------------------------------------------------------------------

/// Request body for submitting a batch of videos.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoBatchSubmitRequest {
    /// 1–100 raw HeyGen `POST /v3/videos` request bodies, passed through
    /// verbatim. Each is polymorphic, discriminated by its `"type"` field
    /// ("avatar" | "image" | "cinematic_avatar"), so items are kept as
    /// opaque JSON objects.
    pub videos: Vec<serde_json::Value>,

    /// Display name for the batch in the HeyGen app.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Response from submitting a video batch (202 Accepted).
#[derive(Debug, Clone, Deserialize)]
pub struct VideoBatchSubmitResponse {
    /// Batch id — poll [`Client::video_batch_status`] with it.
    pub batch_id: String,

    /// Always "processing" at submit.
    #[serde(default)]
    pub status: String,

    /// Count of submitted items.
    #[serde(default)]
    pub total_items: i32,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Query parameters for the batch status page.
#[derive(Debug, Clone, Default)]
pub struct VideoBatchStatusQuery {
    /// Page size (1–100; upstream default 100).
    pub limit: Option<i32>,

    /// Opaque cursor from a previous response's `next_token`.
    pub token: Option<String>,
}

/// Per-item error detail in a batch status page.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoBatchItemError {
    #[serde(default)]
    pub code: String,

    #[serde(default)]
    pub message: String,
}

/// One item of a batch status page, ordered by `item_index`.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoBatchItem {
    /// Zero-based position in the submitted `videos` array.
    pub item_index: i32,

    /// "queued" | "processing" | "completed" | "failed".
    pub status: String,

    /// Present once the item's video exists.
    #[serde(default)]
    pub video_id: Option<String>,

    /// Present only when `billing_status == "settled"` and the item completed.
    #[serde(default)]
    pub video_url: Option<String>,

    /// Present only when the item failed.
    #[serde(default)]
    pub error: Option<VideoBatchItemError>,
}

/// Response from a batch status check (one cursor-paginated page of items).
///
/// Billing settles the first time a GET observes a terminal batch status;
/// `video_url` values are withheld until `billing_status == "settled"` —
/// keep polling until then to obtain URLs.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoBatchStatusResponse {
    /// Batch id.
    pub batch_id: String,

    /// Batch display name (may be empty).
    #[serde(default)]
    pub title: String,

    /// Batch-level status: "processing" | "completed" | "failed".
    pub status: String,

    /// Count of submitted items.
    #[serde(default)]
    pub total_items: i32,

    /// Per-item-status counts across the whole batch.
    #[serde(default)]
    pub counts_by_status: HashMap<String, i32>,

    /// Batch creation time in unix seconds (upstream HeyGen timestamp).
    #[serde(default)]
    pub created_at: i64,

    /// One page of items, ordered by `item_index`.
    #[serde(default, deserialize_with = "null_as_default")]
    pub items: Vec<VideoBatchItem>,

    /// More item pages exist.
    #[serde(default)]
    pub has_more: bool,

    /// Pass as `token` for the next page (may be empty).
    #[serde(default)]
    pub next_token: String,

    /// "unsettled" | "settlement_pending" | "settled".
    #[serde(default)]
    pub billing_status: String,

    /// Total ticks charged for the batch; 0 until settled.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
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

    /// Inspects a HeyGen template's variable schema and scenes (unbilled).
    ///
    /// Only draft-v4 templates with variables are supported upstream; an
    /// unknown template id surfaces as a `provider_error`.
    pub async fn video_template_detail(
        &self,
        template_id: &str,
    ) -> Result<VideoTemplateDetailResponse> {
        let path = format!("/qai/v1/video/template/{template_id}");
        let (resp, _meta) = self.get_json::<VideoTemplateDetailResponse>(&path).await?;
        Ok(resp)
    }

    /// Renders a video from a HeyGen template (async job type
    /// "video/template-v3").
    ///
    /// Returns the accepted-job envelope — poll with `get_job` / `poll_job`
    /// (or SSE via `stream_job`) until "completed"/"failed", then read
    /// `result.video_url`. Deep validation happens at execution time, so
    /// violations surface as a failed job rather than a 4xx at submit.
    pub async fn video_template_generate(
        &self,
        template_id: &str,
        req: &VideoTemplateGenerateRequest,
    ) -> Result<JobAcceptedResponse> {
        let path = format!("/qai/v1/video/template/{template_id}");
        let (resp, _meta) = self
            .post_json::<VideoTemplateGenerateRequest, JobAcceptedResponse>(&path, req)
            .await?;
        Ok(resp)
    }

    /// Submits 1–100 raw HeyGen video payloads as one batch (202 Accepted).
    ///
    /// Poll [`Client::video_batch_status`] for progress and delivery.
    pub async fn video_batch_submit(
        &self,
        req: &VideoBatchSubmitRequest,
    ) -> Result<VideoBatchSubmitResponse> {
        let (resp, _meta) = self
            .post_json::<VideoBatchSubmitRequest, VideoBatchSubmitResponse>(
                "/qai/v1/video/batch",
                req,
            )
            .await?;
        Ok(resp)
    }

    /// Gets a batch's status plus one cursor-paginated page of items.
    ///
    /// Poll (~5s) until `status` is terminal, then keep polling until
    /// `billing_status == "settled"` — per-item `video_url` values are
    /// withheld until settlement. Collect URLs across pages via `next_token`.
    pub async fn video_batch_status(
        &self,
        batch_id: &str,
        query: &VideoBatchStatusQuery,
    ) -> Result<VideoBatchStatusResponse> {
        let mut params = Vec::new();
        if let Some(limit) = query.limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(ref token) = query.token {
            params.push(format!("token={}", urlencoding::encode(token)));
        }
        let mut path = format!("/qai/v1/video/batch/{batch_id}");
        if !params.is_empty() {
            path = format!("{path}?{}", params.join("&"));
        }
        let (resp, _meta) = self.get_json::<VideoBatchStatusResponse>(&path).await?;
        Ok(resp)
    }
}
