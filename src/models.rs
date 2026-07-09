use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::serde_util::null_as_default;

/// Describes an available model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelInfo {
    /// Model identifier used in API requests.
    pub id: String,

    /// Upstream provider (e.g. "anthropic", "xai", "openai").
    pub provider: String,

    /// Human-readable model name.
    pub display_name: String,

    /// Model category (e.g. "Text", "Image", "Audio", "Video", "Embedding").
    #[serde(default)]
    pub category: Option<String>,

    /// Cost per million input tokens in USD (text models).
    #[serde(default)]
    pub input_per_million: f64,

    /// Cost per million output tokens in USD (text models).
    #[serde(default)]
    pub output_per_million: f64,

    /// Per-unit price for non-token models (image/audio/video).
    #[serde(default)]
    pub per_unit_price: Option<f64>,

    /// Price unit description (e.g. "per image", "per second").
    #[serde(default)]
    pub price_unit: Option<String>,

    /// Context window (e.g. "200K", "2M"); display-only.
    #[serde(default)]
    pub context_window: Option<String>,

    /// Routing hint ("direct", "vertex-maas", …).
    #[serde(default)]
    pub route: Option<String>,

    /// Reachable via GCP/Vertex credentials.
    #[serde(default)]
    pub vertex_available: bool,

    /// Rolling aliases that resolve to this model (e.g. "claude-opus-latest").
    /// Prefer sending the alias so backend model swaps don't break pinned picks.
    #[serde(default, deserialize_with = "null_as_default")]
    pub aliases: Vec<String>,

    /// True when this model is the current target of a rolling alias — the
    /// recommended "latest" default for its family/category.
    #[serde(default)]
    pub is_default: bool,

    /// Semantic per-model parameter schema (temperature/effort/size/…), as raw
    /// JSON so clients can render controls without the SDK modelling every kind.
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
}

/// Pricing details for a model.
#[derive(Debug, Clone, Deserialize)]
pub struct PricingInfo {
    /// Model identifier.
    pub id: String,

    /// Upstream provider.
    pub provider: String,

    /// Human-readable model name.
    pub display_name: String,

    /// Cost per million input tokens in USD.
    pub input_per_million: f64,

    /// Cost per million output tokens in USD.
    pub output_per_million: f64,
}

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct PricingResponse {
    pricing: Vec<PricingInfo>,
}

impl Client {
    /// Returns all available models with provider and pricing information.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let (resp, _meta) = self.get_json::<ModelsResponse>("/qai/v1/models").await?;
        Ok(resp.models)
    }

    /// Returns the complete pricing table for all models.
    pub async fn get_pricing(&self) -> Result<Vec<PricingInfo>> {
        let (resp, _meta) = self.get_json::<PricingResponse>("/qai/v1/pricing").await?;
        Ok(resp.pricing)
    }
}
