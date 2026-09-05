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
    #[serde(default, deserialize_with = "null_as_default")]
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
    /// Base64-encoded video bytes (never a URL on this route).
    pub base64: String,

    /// Video format (e.g. "mp4").
    pub format: String,

    /// Video file size.
    pub size_bytes: i64,

    /// Video index within the batch.
    pub index: i32,
}

/// Alias of [`JobAcceptedResponse`]: the HeyGen routes answer with the
/// shared 202 job envelope.
pub type JobResponse = JobAcceptedResponse;

// ---------------------------------------------------------------------------
// HeyGen Studio
// ---------------------------------------------------------------------------

/// Request body for a HeyGen studio talking-head video: one avatar reading
/// one script in one voice. All three fields are required.
///
/// Submission is gated by a balance preflight estimated from the script
/// length (per-second render rate at a nominal reading speed, with a
/// minimum), so an under-funded caller gets 402 at submit rather than a
/// failed job.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoStudioRequest {
    /// HeyGen avatar id.
    pub avatar_id: String,

    /// Script the avatar speaks.
    pub script: String,

    /// HeyGen voice id.
    pub voice_id: String,
}

/// Backwards-compatible alias.
pub type StudioVideoRequest = VideoStudioRequest;

// ---------------------------------------------------------------------------
// HeyGen Translate
// ---------------------------------------------------------------------------

/// Request body for video translation. The source must be reachable by
/// URL; there is no inline-bytes variant on this route.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoTranslateRequest {
    /// URL of the video to translate (required).
    pub video_url: String,

    /// Target language (required).
    pub output_language: String,

    /// Source language (auto-detected if omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_language: Option<String>,

    /// Title for the translated video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Backwards-compatible alias.
pub type TranslateRequest = VideoTranslateRequest;

// ---------------------------------------------------------------------------
// HeyGen Photo Avatar
// ---------------------------------------------------------------------------

/// Request body for creating a talking-photo video: a photo animated to
/// speak a script.
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

/// Request body for creating a digital twin from training footage
/// (`POST /qai/v1/video/digital-twin`). This trains an avatar; it does not
/// render a video — see [`TwinVideoRequest`] for that.
///
/// A flat fee is held before the body is decoded and released on every
/// error path, so a rejected request costs nothing but a ledger round-trip.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DigitalTwinCreateRequest {
    /// Display name for the twin.
    pub name: String,

    /// URL of the training footage (required).
    pub video_url: String,

    /// Add the look to an existing avatar group instead of creating one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_group_id: Option<String>,
}

/// Response from creating a digital twin (synchronous, not a job).
#[derive(Debug, Clone, Deserialize)]
pub struct DigitalTwinCreateResponse {
    /// Echo of the twin's name.
    #[serde(default)]
    pub name: String,

    /// Consent-recording link the subject must complete before the twin
    /// can render.
    #[serde(default)]
    pub consent_url: String,

    /// Billing model label.
    #[serde(default)]
    pub model: String,

    /// Avatar group id holding the twin (absent if HeyGen returned none).
    #[serde(default)]
    pub group_id: Option<String>,

    /// Group training status.
    #[serde(default)]
    pub status: Option<String>,

    /// Consent status of the group.
    #[serde(default)]
    pub consent_status: Option<String>,

    /// Look id usable as `avatar_id` in [`TwinVideoRequest`].
    #[serde(default)]
    pub avatar_id: Option<String>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for rendering a video of a trained avatar look
/// (`POST /qai/v1/video/twin-video`, async job type "video/twin").
///
/// Exactly one of `script` or `audio_base64` is required; `voice_id` is
/// required with `script`. Billed per generated second at settle; a
/// balance preflight rejects under-funded callers with 402 at submit.
#[derive(Debug, Clone, Serialize, Default)]
pub struct TwinVideoRequest {
    /// Avatar look id (a trained twin or a preset).
    pub avatar_id: String,

    /// Script for a HeyGen TTS voice to read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,

    /// HeyGen voice id (required with `script`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,

    /// Base64-encoded narration audio (alternative to `script`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_base64: Option<String>,

    /// MIME type of `audio_base64` (e.g. "audio/mpeg").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_media_type: Option<String>,

    /// Aspect ratio (e.g. "16:9").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    /// Output resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,

    /// Video title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Render engine; omitted = best supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
}

// ---------------------------------------------------------------------------
// HeyGen Avatars
// ---------------------------------------------------------------------------

/// A HeyGen avatar look.
#[derive(Debug, Clone, Deserialize)]
pub struct Avatar {
    /// Avatar identifier (what the video routes accept as `avatar_id`).
    pub avatar_id: String,

    /// Avatar name. Wire field: `avatar_name`.
    #[serde(rename = "avatar_name", default)]
    pub name: String,

    /// Avatar gender.
    #[serde(default)]
    pub gender: String,

    /// Preview image URL. Wire field: `preview_image_url`.
    #[serde(rename = "preview_image_url", default)]
    pub preview_url: String,

    /// Look type: "studio_avatar" | "digital_twin" | "photo_avatar".
    /// Wire field: `type`.
    #[serde(rename = "type", default)]
    pub avatar_type: String,
}

/// Response from listing HeyGen avatars.
#[derive(Debug, Clone, Deserialize)]
pub struct AvatarsResponse {
    /// Available avatars.
    #[serde(default, deserialize_with = "null_as_default")]
    pub avatars: Vec<Avatar>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// HeyGen Templates
// ---------------------------------------------------------------------------

/// A HeyGen video template (API-ready, draft-v4 templates only).
#[derive(Debug, Clone, Deserialize)]
pub struct VideoTemplate {
    /// Template identifier.
    pub template_id: String,

    /// Template name.
    #[serde(default)]
    pub name: String,

    /// Thumbnail image URL. Wire field: `thumbnail_image_url`.
    #[serde(rename = "thumbnail_image_url", default)]
    pub thumbnail_url: String,
}

/// Response from listing HeyGen video templates.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoTemplatesResponse {
    /// Available templates.
    #[serde(default, deserialize_with = "null_as_default")]
    pub templates: Vec<VideoTemplate>,

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

    /// Voice name. Wire field: `display_name`.
    #[serde(rename = "display_name", default)]
    pub name: String,

    /// Language.
    #[serde(default)]
    pub language: String,

    /// Gender.
    #[serde(default)]
    pub gender: String,

    /// Preview audio URL. Wire field: `preview_audio`.
    #[serde(rename = "preview_audio", default)]
    pub preview_url: String,
}

/// Response from listing HeyGen voices.
#[derive(Debug, Clone, Deserialize)]
pub struct HeyGenVoicesResponse {
    /// Available voices (public catalog followed by the account's private
    /// voices).
    #[serde(default, deserialize_with = "null_as_default")]
    pub voices: Vec<HeyGenVoice>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
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

    /// Creates a HeyGen studio talking-head video (async job type
    /// "video/studio"). Poll the returned `job_id` for the result.
    pub async fn video_studio(&self, req: &VideoStudioRequest) -> Result<JobAcceptedResponse> {
        let (resp, _meta) = self
            .post_json::<VideoStudioRequest, JobAcceptedResponse>("/qai/v1/video/studio", req)
            .await?;
        Ok(resp)
    }

    /// Translates a video into another language (HeyGen; async job type
    /// "video/translate"). A flat hold is taken at submit and trued up
    /// when the job settles.
    pub async fn video_translate(
        &self,
        req: &VideoTranslateRequest,
    ) -> Result<JobAcceptedResponse> {
        let (resp, _meta) = self
            .post_json::<VideoTranslateRequest, JobAcceptedResponse>("/qai/v1/video/translate", req)
            .await?;
        Ok(resp)
    }

    /// Creates a talking-photo video (HeyGen; async job).
    pub async fn video_photo_avatar(
        &self,
        req: &PhotoAvatarRequest,
    ) -> Result<JobAcceptedResponse> {
        let (resp, _meta) = self
            .post_json::<PhotoAvatarRequest, JobAcceptedResponse>("/qai/v1/video/photo-avatar", req)
            .await?;
        Ok(resp)
    }

    /// Creates a digital twin from training footage (HeyGen). Synchronous;
    /// the subject must complete `consent_url` before the twin renders.
    /// Render with [`Client::video_twin`].
    pub async fn video_digital_twin(
        &self,
        req: &DigitalTwinCreateRequest,
    ) -> Result<DigitalTwinCreateResponse> {
        let (mut resp, meta) = self
            .post_json::<DigitalTwinCreateRequest, DigitalTwinCreateResponse>(
                "/qai/v1/video/digital-twin",
                req,
            )
            .await?;
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Renders a video of a trained avatar look delivering a script or
    /// supplied narration (HeyGen; async job type "video/twin").
    pub async fn video_twin(&self, req: &TwinVideoRequest) -> Result<JobAcceptedResponse> {
        let (resp, _meta) = self
            .post_json::<TwinVideoRequest, JobAcceptedResponse>("/qai/v1/video/twin-video", req)
            .await?;
        Ok(resp)
    }

    /// Lists available HeyGen avatar looks (public and private, every page
    /// aggregated).
    pub async fn video_avatars(&self) -> Result<AvatarsResponse> {
        let (resp, _meta) = self
            .get_json::<AvatarsResponse>("/qai/v1/video/avatars")
            .await?;
        Ok(resp)
    }

    /// Lists available HeyGen video templates (API-ready templates only).
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
    /// `result.video_url`. Submit rejects at once with 400 for an empty
    /// `variables` map and 402 when the balance is below the render
    /// preflight; only HeyGen-side validation of the variable values is
    /// deferred and surfaces as a failed job.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(value: &serde_json::Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().expect("object").keys().cloned().collect();
        keys.sort();
        keys
    }

    #[test]
    fn studio_request_carries_the_three_required_keys() {
        let json = serde_json::to_value(VideoStudioRequest {
            avatar_id: "av".into(),
            script: "Hello".into(),
            voice_id: "vc".into(),
        })
        .unwrap();
        assert_eq!(keys(&json), ["avatar_id", "script", "voice_id"]);
    }

    #[test]
    fn translate_request_uses_video_url_and_output_language() {
        let json = serde_json::to_value(VideoTranslateRequest {
            video_url: "https://x/v.mp4".into(),
            output_language: "es".into(),
            source_language: None,
            title: Some("t".into()),
        })
        .unwrap();
        assert_eq!(keys(&json), ["output_language", "title", "video_url"]);
    }

    #[test]
    fn twin_create_and_twin_video_are_different_routes() {
        let json = serde_json::to_value(DigitalTwinCreateRequest {
            name: "Rich".into(),
            video_url: "https://x/train.mp4".into(),
            avatar_group_id: None,
        })
        .unwrap();
        assert_eq!(keys(&json), ["name", "video_url"]);

        let resp: DigitalTwinCreateResponse = serde_json::from_str(
            r#"{"name":"Rich","consent_url":"https://heygen/consent/1","model":"heygen-digital-twin",
                "request_id":"req_t","group_id":"grp_1","status":"pending",
                "consent_status":"not_started","avatar_id":"look_1"}"#,
        )
        .unwrap();
        assert_eq!(resp.group_id.as_deref(), Some("grp_1"));
        assert_eq!(resp.avatar_id.as_deref(), Some("look_1"));

        let json = serde_json::to_value(TwinVideoRequest {
            avatar_id: "look_1".into(),
            script: Some("Hi".into()),
            voice_id: Some("v1".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(keys(&json), ["avatar_id", "script", "voice_id"]);
    }

    #[test]
    fn heygen_catalog_types_read_the_wire_names() {
        let avatars: AvatarsResponse = serde_json::from_str(
            r#"{"avatars":[{"avatar_id":"a1","avatar_name":"Anna","gender":"female",
                "preview_image_url":"https://x/a.png","type":"studio_avatar"}],"request_id":"r"}"#,
        )
        .unwrap();
        assert_eq!(avatars.avatars[0].name, "Anna");
        assert_eq!(avatars.avatars[0].preview_url, "https://x/a.png");
        assert_eq!(avatars.avatars[0].avatar_type, "studio_avatar");

        let templates: VideoTemplatesResponse = serde_json::from_str(
            r#"{"templates":[{"template_id":"t1","name":"Promo",
                "thumbnail_image_url":"https://x/t.png"}],"request_id":"r"}"#,
        )
        .unwrap();
        assert_eq!(templates.templates[0].thumbnail_url, "https://x/t.png");

        let voices: HeyGenVoicesResponse = serde_json::from_str(
            r#"{"voices":[{"voice_id":"v1","display_name":"Ava","language":"en","gender":"female",
                "preview_audio":"https://x/v.mp3"}],"request_id":"r"}"#,
        )
        .unwrap();
        assert_eq!(voices.voices[0].name, "Ava");
        assert_eq!(voices.voices[0].preview_url, "https://x/v.mp3");

        // Empty catalogs are serialised as null by the gateway.
        let none: AvatarsResponse =
            serde_json::from_str(r#"{"avatars":null,"request_id":"r"}"#).unwrap();
        assert!(none.avatars.is_empty());
    }
}
