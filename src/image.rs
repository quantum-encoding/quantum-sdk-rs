use serde::{Deserialize, Deserializer, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Deserialize null as Default::default() (e.g. null → empty Vec).
fn deserialize_null_as_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

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
}

/// Response from image generation.
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

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
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
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }
}
