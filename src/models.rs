use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::account::{PricingEntry, PricingResponse};
use crate::client::Client;
use crate::error::Result;
use crate::serde_util::null_as_default;

/// Describes an available model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelInfo {
    /// Model identifier used in API requests.
    pub id: String,

    /// Upstream provider (e.g. "anthropic", "xai", "openai", "dashscope").
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

/// One model's pricing, as `/qai/v1/pricing` sends it. The model id is
/// the map key in [`Client::get_pricing`]; the entry repeats it in
/// `model`.
pub type PricingInfo = PricingEntry;

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

impl Client {
    /// Returns all available models with provider and pricing information.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let (resp, _meta) = self.get_json::<ModelsResponse>("/qai/v1/models").await?;
        Ok(resp.models)
    }

    /// Returns the pricing table for every model, keyed by model id, with
    /// the gateway's margin already applied. The same route as
    /// [`account_pricing`](Client::account_pricing), unwrapped to the map.
    pub async fn get_pricing(&self) -> Result<HashMap<String, PricingInfo>> {
        let (resp, _meta) = self.get_json::<PricingResponse>("/qai/v1/pricing").await?;
        Ok(resp.pricing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pricing_is_a_map_keyed_by_model_id() {
        // routes_meta.go writes {"pricing": {<id>: entry}, "count", "margin"}.
        let body = r#"{"pricing":{"claude-sonnet-4-6":{"provider":"anthropic","model":"claude-sonnet-4-6",
            "display_name":"Claude Sonnet 4.6","category":"Text","context_window":"200K",
            "input_per_million":3.3,"output_per_million":16.5,"cached_per_million":0.33},
            "grok-imagine-image":{"provider":"xai","model":"grok-imagine-image","display_name":"Grok Imagine",
            "category":"Image","per_unit_price":0.077,"price_unit":"per image"}},"count":2,"margin":0.1}"#;
        let resp: PricingResponse = serde_json::from_str(body).unwrap();
        let sonnet = &resp.pricing["claude-sonnet-4-6"];
        assert_eq!(sonnet.provider, "anthropic");
        assert_eq!(sonnet.input_per_million, 3.3);
        assert_eq!(sonnet.cached_per_million, 0.33);
        let image = &resp.pricing["grok-imagine-image"];
        assert_eq!(image.per_unit_price, Some(0.077));
        assert_eq!(image.price_unit.as_deref(), Some("per image"));
    }
}
