use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::client::Client;
use crate::error::Result;

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// Request body for vision analysis endpoints.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VisionRequest {
    /// Base64-encoded image (with or without data: prefix).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,

    /// Image URL (fetched by the model provider).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    /// Model to use. Default: gemini-2.5-flash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Analysis profile: "combined" (default), "scene", "objects", "ocr", "quality".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Domain context for relevance checking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<VisionContext>,
}

/// Domain context for relevance analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisionContext {
    /// Installation type (e.g. "solar", "heat_pump", "ev_charger").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installation_type: Option<String>,

    /// Phase (e.g. "pre_install", "installation", "post_install").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,

    /// Expected items for relevance checking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_items: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

/// Full vision analysis response.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct VisionResponse {
    /// Scene description.
    #[serde(default)]
    pub caption: Option<String>,

    /// Suggested tags (lowercase_snake_case).
    #[serde(default)]
    pub tags: Vec<String>,

    /// Detected objects with bounding boxes.
    #[serde(default)]
    pub objects: Vec<DetectedObject>,

    /// Image quality assessment.
    #[serde(default)]
    pub quality: Option<QualityAssessment>,

    /// Relevance check against context.
    #[serde(default)]
    pub relevance: Option<RelevanceCheck>,

    /// Extracted text and overlay metadata.
    #[serde(default)]
    pub ocr: Option<OcrResult>,

    /// Model used.
    #[serde(default)]
    pub model: String,

    /// Cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// A detected object with bounding box.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetectedObject {
    /// Object label.
    pub label: String,

    /// Detection confidence (0.0 - 1.0).
    #[serde(default)]
    pub confidence: f64,

    /// Bounding box: [y_min, x_min, y_max, x_max] normalised to 0-1000.
    #[serde(default)]
    pub bounding_box: [i32; 4],
}

/// Image quality assessment.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct QualityAssessment {
    /// Overall rating: "good", "acceptable", "poor".
    #[serde(default)]
    pub overall: String,

    /// Quality score (0.0 - 1.0).
    #[serde(default)]
    pub score: f64,

    /// Blur level: "none", "slight", "significant".
    #[serde(default)]
    pub blur: String,

    /// Lighting: "well_lit", "dim", "dark".
    #[serde(default)]
    pub darkness: String,

    /// Resolution: "high", "adequate", "low".
    #[serde(default)]
    pub resolution: String,

    /// Exposure: "correct", "over", "under".
    #[serde(default)]
    pub exposure: String,

    /// Specific issues found.
    #[serde(default)]
    pub issues: Vec<String>,
}

/// Relevance check against expected content.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RelevanceCheck {
    /// Whether the image is relevant to the context.
    #[serde(default)]
    pub relevant: bool,

    /// Relevance score (0.0 - 1.0).
    #[serde(default)]
    pub score: f64,

    /// Items expected based on context.
    #[serde(default)]
    pub expected_items: Vec<String>,

    /// Items actually found in the image.
    #[serde(default)]
    pub found_items: Vec<String>,

    /// Expected but not found.
    #[serde(default)]
    pub missing_items: Vec<String>,

    /// Found but not expected.
    #[serde(default)]
    pub unexpected_items: Vec<String>,

    /// Additional notes.
    #[serde(default)]
    pub notes: Option<String>,
}

/// OCR / text extraction result.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct OcrResult {
    /// All extracted text concatenated.
    #[serde(default)]
    pub text: Option<String>,

    /// Extracted metadata (GPS, timestamp, address, etc.).
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// Individual text overlays with positions.
    #[serde(default)]
    pub overlays: Vec<TextOverlay>,
}

/// A detected text region in the image.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TextOverlay {
    /// Extracted text content.
    #[serde(default)]
    pub text: String,

    /// Bounding box: [y_min, x_min, y_max, x_max] normalised to 0-1000.
    #[serde(default)]
    pub bounding_box: Option<[i32; 4]>,

    /// Overlay type: "gps", "timestamp", "address", "label", "other".
    #[serde(rename = "type", default)]
    pub overlay_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Full combined vision analysis (scene + objects + quality + OCR + relevance).
    pub async fn vision_analyze(&self, req: &VisionRequest) -> Result<VisionResponse> {
        let (resp, _meta) = self
            .post_json::<VisionRequest, VisionResponse>("/qai/v1/vision/analyze", req)
            .await?;
        Ok(resp)
    }

    /// Object detection with bounding boxes.
    pub async fn vision_detect(&self, req: &VisionRequest) -> Result<VisionResponse> {
        let (resp, _meta) = self
            .post_json::<VisionRequest, VisionResponse>("/qai/v1/vision/detect", req)
            .await?;
        Ok(resp)
    }

    /// Scene description and tags.
    pub async fn vision_describe(&self, req: &VisionRequest) -> Result<VisionResponse> {
        let (resp, _meta) = self
            .post_json::<VisionRequest, VisionResponse>("/qai/v1/vision/describe", req)
            .await?;
        Ok(resp)
    }

    /// Text extraction and overlay metadata (OCR).
    pub async fn vision_ocr(&self, req: &VisionRequest) -> Result<VisionResponse> {
        let (resp, _meta) = self
            .post_json::<VisionRequest, VisionResponse>("/qai/v1/vision/ocr", req)
            .await?;
        Ok(resp)
    }

    /// Image quality assessment.
    pub async fn vision_quality(&self, req: &VisionRequest) -> Result<VisionResponse> {
        let (resp, _meta) = self
            .post_json::<VisionRequest, VisionResponse>("/qai/v1/vision/quality", req)
            .await?;
        Ok(resp)
    }
}
