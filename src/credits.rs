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

    /// Whether this is the popular/recommended pack.
    #[serde(default)]
    pub popular: Option<bool>,
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

/// A developer tier, as `/qai/v1/credits/tiers` describes it.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditTier {
    /// Tier identifier (e.g. `"standard"`, `"lifetime"`, `"internal"`).
    pub tier: String,

    /// Display label.
    #[serde(default)]
    pub label: String,

    /// Margin the gateway adds on top of provider cost, in percent.
    #[serde(default)]
    pub margin_percent: f64,

    /// What the tier offers.
    #[serde(default)]
    pub description: String,

    /// How an account qualifies for it.
    #[serde(default)]
    pub requirements: String,
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

    /// Expected monthly spend in USD (optional); `expected_monthly_usd`
    /// on the wire.
    #[serde(
        rename = "expected_monthly_usd",
        skip_serializing_if = "Option::is_none"
    )]
    pub expected_monthly_usd: Option<f64>,

    /// Website URL (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}

/// A one-time lifetime unlock product.
#[derive(Debug, Clone, Deserialize)]
pub struct LifetimePlan {
    pub id: String,
    pub label: String,
    pub amount_usd: f64,
    /// Seats included; 0 means unlimited.
    #[serde(default)]
    pub seats: i64,
    #[serde(default)]
    pub description: Option<String>,
}

/// Response from [`Client::lifetime_plans`].
#[derive(Debug, Clone, Deserialize)]
pub struct LifetimePlansResponse {
    pub plans: Vec<LifetimePlan>,
}

/// Request body for buying a lifetime plan.
#[derive(Debug, Clone, Serialize, Default)]
pub struct LifetimePurchaseRequest {
    pub plan_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,
}

/// Response from [`Client::lifetime_purchase`]: where to pay.
#[derive(Debug, Clone, Deserialize)]
pub struct LifetimePurchaseResponse {
    pub checkout_url: String,
    pub session_id: String,
    pub plan: LifetimePlan,
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
    pub async fn credit_purchase(
        &self,
        req: &CreditPurchaseRequest,
    ) -> Result<CreditPurchaseResponse> {
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

    /// List the lifetime unlock plans.
    pub async fn lifetime_plans(&self) -> Result<LifetimePlansResponse> {
        let (resp, _meta) = self
            .get_json::<LifetimePlansResponse>("/qai/v1/credits/lifetime")
            .await?;
        Ok(resp)
    }

    /// Buy a lifetime plan. Returns a checkout URL for payment.
    pub async fn lifetime_purchase(
        &self,
        req: &LifetimePurchaseRequest,
    ) -> Result<LifetimePurchaseResponse> {
        let (resp, _meta) = self
            .post_json::<LifetimePurchaseRequest, LifetimePurchaseResponse>(
                "/qai/v1/credits/lifetime",
                req,
            )
            .await?;
        Ok(resp)
    }

    /// Apply for the developer program.
    pub async fn dev_program_apply(
        &self,
        req: &DevProgramApplyRequest,
    ) -> Result<DevProgramApplyResponse> {
        let (resp, _meta) = self
            .post_json::<DevProgramApplyRequest, DevProgramApplyResponse>(
                "/qai/v1/credits/dev-program",
                req,
            )
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_read_the_gateways_tier_info_shape() {
        // billing.TierInfo: {tier,label,margin_percent,description,requirements}.
        let body = r#"{"tiers":[{"tier":"standard","label":"Standard","margin_percent":10,
            "description":"Pay as you go","requirements":"None"}]}"#;
        let resp: CreditTiersResponse = serde_json::from_str(body).unwrap();
        let t = &resp.tiers[0];
        assert_eq!(t.tier, "standard");
        assert_eq!(t.label, "Standard");
        assert_eq!(t.margin_percent, 10.0);
        assert_eq!(t.requirements, "None");
    }

    #[test]
    fn dev_program_spend_uses_the_gateways_field_name() {
        let req = DevProgramApplyRequest {
            use_case: "agents".into(),
            company: None,
            expected_monthly_usd: Some(250.0),
            website: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(
            json,
            r#"{"use_case":"agents","expected_monthly_usd":250.0}"#
        );
    }
}
