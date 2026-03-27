use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Response from balance check.
#[derive(Debug, Clone, Deserialize)]
pub struct BalanceResponse {
    #[serde(default)]
    pub user_id: Option<String>,
    pub balance_ticks: i64,
    pub balance_usd: f64,
    #[serde(default)]
    pub ticks_per_usd: i64,
}

/// A single usage entry from the ledger.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageEntry {
    pub id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub delta_ticks: Option<i64>,
    #[serde(default)]
    pub balance_after: Option<i64>,
    #[serde(default)]
    pub input_tokens: Option<i64>,
    #[serde(default)]
    pub output_tokens: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Paginated usage history response.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageResponse {
    pub entries: Vec<UsageEntry>,
    pub has_more: bool,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// Monthly usage summary.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageSummaryMonth {
    pub month: String,
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
    pub total_margin_usd: f64,
    #[serde(default)]
    pub by_provider: Vec<serde_json::Value>,
}

/// Response from usage summary.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageSummaryResponse {
    pub months: Vec<UsageSummaryMonth>,
}

/// Usage query parameters.
#[derive(Debug, Clone, Serialize, Default)]
pub struct UsageQuery {
    /// Max entries per page (default 20, max 100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,

    /// Cursor for pagination (from previous response's next_cursor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_after: Option<String>,
}

/// Pricing entry from the pricing table.
#[derive(Debug, Clone, Deserialize)]
pub struct PricingEntry {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub input_per_million: f64,
    #[serde(default)]
    pub output_per_million: f64,
    #[serde(default)]
    pub cached_per_million: f64,
}

/// Pricing response (map of model_id → entry).
#[derive(Debug, Clone, Deserialize)]
pub struct PricingResponse {
    pub pricing: std::collections::HashMap<String, PricingEntry>,
}

impl Client {
    /// Gets the account credit balance.
    pub async fn account_balance(&self) -> Result<BalanceResponse> {
        let (resp, _meta) = self
            .get_json::<BalanceResponse>("/qai/v1/account/balance")
            .await?;
        Ok(resp)
    }

    /// Gets paginated usage history.
    pub async fn account_usage(&self, query: &UsageQuery) -> Result<UsageResponse> {
        let mut path = "/qai/v1/account/usage".to_string();
        let mut params = Vec::new();
        if let Some(limit) = query.limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(ref cursor) = query.start_after {
            params.push(format!("start_after={cursor}"));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }
        let (resp, _meta) = self.get_json::<UsageResponse>(&path).await?;
        Ok(resp)
    }

    /// Gets monthly usage summary.
    pub async fn account_usage_summary(&self, months: Option<i32>) -> Result<UsageSummaryResponse> {
        let path = if let Some(m) = months {
            format!("/qai/v1/account/usage/summary?months={m}")
        } else {
            "/qai/v1/account/usage/summary".to_string()
        };
        let (resp, _meta) = self.get_json::<UsageSummaryResponse>(&path).await?;
        Ok(resp)
    }

    /// Gets the full pricing table (model → pricing entry map).
    pub async fn account_pricing(&self) -> Result<PricingResponse> {
        let (resp, _meta) = self
            .get_json::<PricingResponse>("/qai/v1/pricing")
            .await?;
        Ok(resp)
    }
}
