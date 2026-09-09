use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

use crate::serde_util::null_as_default as deserialize_null_as_default;

/// Request body for image generation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ImageRequest {
    /// Image generation model (e.g. "grok-imagine-image", "gpt-image-1", "dall-e-3").
    pub model: String,

    /// Describes the image to generate.
    pub prompt: String,

    /// Number of images to generate (default 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,

    /// Output dimensions (e.g. "1024x1024", "1536x1024").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// Aspect ratio (e.g. "16:9", "1:1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    /// Quality level (e.g. "standard", "hd").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,

    /// Image format (e.g. "png", "jpeg", "webp").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// Style preset (e.g. "vivid", "natural"). DALL-E 3 specific.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    /// Background mode (e.g. "auto", "transparent", "opaque"). GPT-Image specific.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,

    /// Image URL or data URI for image-to-3D conversion (Meshy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    // ── Meshy 3D generation options ──
    /// Mesh topology: "triangle" or "quad".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology: Option<String>,

    /// Target polygon count (100-300,000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_polycount: Option<i32>,

    /// Symmetry mode: "auto", "on", or "off".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symmetry_mode: Option<String>,

    /// Pose mode: "", "a-pose", or "t-pose".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pose_mode: Option<String>,

    /// Generate PBR texture maps (base_color, metallic, roughness, normal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_pbr: Option<bool>,

    /// Catalog-schema-driven extra parameters (e.g. resolution, output_compression,
    /// number_of_images, negative_prompt). Flattened to top-level JSON so any param
    /// the backend's /qai/v1/images accepts is forwarded without a typed field.
    /// An empty map serializes to nothing.
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Response from image generation.
/// `#[non_exhaustive]`: this is only ever deserialised from a gateway reply,
/// never built by a caller, so sealing it costs nothing and makes every future
/// field a patch release instead of a breaking one.
///
/// The request types below are deliberately NOT sealed. `#[non_exhaustive]`
/// forbids struct-literal construction from another crate — including with
/// `..Default::default()`, which is the idiom every caller uses — so sealing a
/// type people build would trade one break now for worse ergonomics forever.
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize)]
pub struct ImageResponse {
    /// Generated images.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub images: Vec<GeneratedImage>,

    /// Model that generated the images.
    #[serde(default)]
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Post-deduction credit balance in ticks. Zero for free or cached
    /// calls, or when the response omits it.
    #[serde(default)]
    pub balance_after: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,

    /// The prompt the provider actually generated from, when it rewrote the
    /// one it was given.
    ///
    /// gpt-image routinely rewrites; the picture is made from this text, not
    /// from what was sent. A caller holding only its own prompt cannot
    /// reproduce its own image, and cannot tell why the output drifted.
    /// Absent when the provider returned no rewrite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,

    /// The token counts the charge was computed from, for models priced on
    /// tokens rather than per image.
    ///
    /// Flat-priced models report none and the field is absent: the rate is
    /// per image and checkable without them. For a token-priced model this is
    /// the whole audit — two real gpt-image-2 generations on the same day came
    /// back at $0.0527 and $0.01628, a 3x spread with nothing else on the wire
    /// to explain it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<ImageUsage>,
}

/// The token counts behind a token-priced image charge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageUsage {
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub completion_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
}

/// A single generated image.
#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedImage {
    /// Base64-encoded image data.
    pub base64: String,

    /// Image format (e.g. "png", "jpeg").
    pub format: String,

    /// Image index within the batch.
    pub index: i32,
}

/// Request body for image editing.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ImageEditRequest {
    /// Editing model (e.g. "gpt-image-1", "grok-imagine-image").
    pub model: String,

    /// Describes the desired edit.
    pub prompt: String,

    /// Base64-encoded input images.
    pub input_images: Vec<String>,

    /// Number of edited images to generate (default 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,

    /// Output dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// Aspect ratio, e.g. "1:1" or "16:9" (Gemini).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    /// Resolution tier: "1K", "2K", "4K" (Gemini Pro / Nano Banana 2).
    ///
    /// Distinct from `size`, which is OpenAI's pixel enum. The two name
    /// different things and both reach the gateway.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size: Option<String>,

    /// Render effort: "auto", "low", "medium", "high" (OpenAI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,

    /// Output container: "png", "jpeg", "webp".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// Background mode: "auto", "transparent", "opaque" (OpenAI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,

    /// "high" preserves faces across an edit (OpenAI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_fidelity: Option<String>,

    /// Search grounding (Gemini Pro).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding: Option<bool>,
}

/// Response from image editing (same shape as generation).
pub type ImageEditResponse = ImageResponse;

impl Client {
    /// Generates images from a text prompt.
    pub async fn generate_image(&self, req: &ImageRequest) -> Result<ImageResponse> {
        let (mut resp, meta) = self
            .post_json::<ImageRequest, ImageResponse>("/qai/v1/images/generate", req)
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

    /// Edits images using an AI model.
    pub async fn edit_image(&self, req: &ImageEditRequest) -> Result<ImageEditResponse> {
        let (mut resp, meta) = self
            .post_json::<ImageEditRequest, ImageEditResponse>("/qai/v1/images/edit", req)
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
}

#[cfg(test)]
mod receipt_tests {
    use super::*;

    /// A receipt has to survive the wire. Both fields were added because the
    /// gateway had them and dropped them; a client that cannot deserialise
    /// them puts them straight back in the bin.
    #[test]
    fn carries_the_rewrite_and_the_usage() {
        let body = r#"{
            "images": [{"base64":"iVBOR","format":"png","index":0}],
            "model": "gpt-image-2",
            "cost_ticks": 527000000,
            "balance_after": 91,
            "request_id": "qai_req_f372518d-447",
            "revised_prompt": "A golden rubber duck wearing a black silk top hat, studio lit",
            "usage": {"prompt_tokens": 120, "completion_tokens": 1580, "total_tokens": 1700}
        }"#;
        let r: ImageResponse = serde_json::from_str(body).expect("deserialise");
        assert_eq!(r.cost_ticks, 527_000_000);
        assert_eq!(
            r.revised_prompt.as_deref(),
            Some("A golden rubber duck wearing a black silk top hat, studio lit")
        );
        let u = r.usage.expect("usage present");
        assert_eq!((u.prompt_tokens, u.completion_tokens, u.total_tokens), (120, 1580, 1700));
    }

    /// A flat-priced generation reports neither, and both stay None rather
    /// than becoming zeros that look measured. An older gateway that has not
    /// shipped these fields must keep deserialising too.
    #[test]
    fn absent_fields_stay_absent() {
        let body = r#"{
            "images": [],
            "model": "grok-imagine-image-2.0",
            "cost_ticks": 500000000,
            "balance_after": 88,
            "request_id": "qai_req_9c03b229-d09"
        }"#;
        let r: ImageResponse = serde_json::from_str(body).expect("deserialise");
        assert_eq!(r.cost_ticks, 500_000_000);
        assert!(r.revised_prompt.is_none(), "a rewrite was invented");
        assert!(r.usage.is_none(), "a usage block was invented");
    }
}
