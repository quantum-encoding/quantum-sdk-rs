use serde::Deserialize;

use crate::client::Client;
use crate::error::Result;

/// Describes an available model.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    /// Model identifier used in API requests.
    pub id: String,

    /// Upstream provider (e.g. "anthropic", "xai", "openai").
    pub provider: String,

    /// Human-readable model name.
    pub display_name: String,

    /// Cost per million input tokens in USD.
    pub input_per_million: f64,

    /// Cost per million output tokens in USD.
    pub output_per_million: f64,
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
