//! Credits — purchase credit packs, check balance, view tiers, and apply for dev program.
//!
//! Some endpoints (packs, tiers) do not require authentication and can be
//! called without an API key.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// A credit pack available for purchase.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditPack {
    /// Unique pack identifier.
    pub id: String,

    /// Display label (e.g. "$5 Starter").
    #[serde(default)]
    pub label: String,

    /// Price in USD.
    #[serde(default)]
    pub amount_usd: f64,

    /// Number of credit ticks included.
    #[serde(default)]
    pub ticks: i64,

    /// Description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Response from listing credit packs.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditPacksResponse {
    /// Available credit packs.
    pub packs: Vec<CreditPack>,
}

/// Request to purchase a credit pack.
#[derive(Debug, Clone, Serialize)]
pub struct CreditPurchaseRequest {
    /// The pack ID to purchase.
    pub pack_id: String,

    /// URL to redirect to after successful payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_url: Option<String>,

    /// URL to redirect to if payment is cancelled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,
}

/// Response from purchasing a credit pack.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditPurchaseResponse {
    /// URL to redirect the user to for payment.
    pub checkout_url: String,
}

/// Response from checking credit balance.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditBalanceResponse {
    /// Balance in ticks.
    pub balance_ticks: i64,

    /// Balance in USD.
    pub balance_usd: f64,
}

/// A pricing tier.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditTier {
    /// Tier name.
    #[serde(default)]
    pub name: Option<String>,

    /// Minimum balance for this tier.
    #[serde(default)]
    pub min_balance: i64,

    /// Discount percentage.
    #[serde(default)]
    pub discount_percent: f64,

    /// Additional tier data.
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Response from listing credit tiers.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditTiersResponse {
    /// Available tiers.
    pub tiers: Vec<CreditTier>,
}

/// Request to apply for the developer program.
#[derive(Debug, Clone, Serialize)]
pub struct DevProgramApplyRequest {
    /// Description of the intended use case.
    pub use_case: String,

    /// Company name (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,

    /// Expected monthly spend in USD (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_usd: Option<f64>,

    /// Website URL (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}

/// Response from dev program application.
#[derive(Debug, Clone, Deserialize)]
pub struct DevProgramApplyResponse {
    /// Status of the application (e.g. "submitted", "approved").
    pub status: String,
}

impl Client {
    /// List available credit packs. No authentication required.
    pub async fn credit_packs(&self) -> Result<CreditPacksResponse> {
        let (resp, _meta) = self
            .get_json::<CreditPacksResponse>("/qai/v1/credits/packs")
            .await?;
        Ok(resp)
    }

    /// Purchase a credit pack. Returns a checkout URL for payment.
    pub async fn credit_purchase(&self, req: &CreditPurchaseRequest) -> Result<CreditPurchaseResponse> {
        let (resp, _meta) = self
            .post_json::<CreditPurchaseRequest, CreditPurchaseResponse>(
                "/qai/v1/credits/purchase",
                req,
            )
            .await?;
        Ok(resp)
    }

    /// Get the current credit balance.
    pub async fn credit_balance(&self) -> Result<CreditBalanceResponse> {
        let (resp, _meta) = self
            .get_json::<CreditBalanceResponse>("/qai/v1/credits/balance")
            .await?;
        Ok(resp)
    }

    /// List available credit tiers. No authentication required.
    pub async fn credit_tiers(&self) -> Result<CreditTiersResponse> {
        let (resp, _meta) = self
            .get_json::<CreditTiersResponse>("/qai/v1/credits/tiers")
            .await?;
        Ok(resp)
    }

    /// Apply for the developer program.
    pub async fn dev_program_apply(&self, req: &DevProgramApplyRequest) -> Result<DevProgramApplyResponse> {
        let (resp, _meta) = self
            .post_json::<DevProgramApplyRequest, DevProgramApplyResponse>(
                "/qai/v1/credits/dev-program",
                req,
            )
            .await?;
        Ok(resp)
    }
}
