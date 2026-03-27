use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::jobs::{JobCreateRequest, JobCreateResponse};

// ---------------------------------------------------------------------------
// Scrape
// ---------------------------------------------------------------------------

/// A single scrape target.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScrapeTarget {
    /// Target name.
    pub name: String,

    /// Start URL to scrape.
    pub url: String,

    /// Target type: "scrape" (default) or "openapi".
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,

    /// CSS selector for navigation links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,

    /// CSS selector for content area.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Joplin notebook name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notebook: Option<String>,

    /// Enable recursive link discovery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,

    /// Maximum pages to scrape.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_pages: Option<i32>,

    /// Delay between pages in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<i32>,

    /// RAG provider name for auto-ingest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingest: Option<String>,

    /// OpenAPI spec URL (for type=openapi targets).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_url: Option<String>,
}

/// Request body for submitting a scrape job.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScrapeRequest {
    /// Targets to scrape.
    pub targets: Vec<ScrapeTarget>,
}

/// Response from submitting a scrape job.
#[derive(Debug, Clone, Deserialize)]
pub struct ScrapeResponse {
    /// Job identifier for polling.
    pub job_id: String,

    /// Initial status.
    #[serde(default)]
    pub status: String,

    /// Number of targets submitted.
    #[serde(default)]
    pub targets: i32,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// Screenshot
// ---------------------------------------------------------------------------

/// A single URL to screenshot.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScreenshotURL {
    /// Page URL to capture.
    pub url: String,

    /// Viewport width (default 1280).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,

    /// Viewport height (default 800).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,

    /// Capture full scrollable page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_page: Option<bool>,

    /// Wait before capture in milliseconds (default 1000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<i32>,
}

/// Request body for taking screenshots.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScreenshotRequest {
    /// URLs to screenshot.
    pub urls: Vec<ScreenshotURL>,
}

/// A single screenshot result.
#[derive(Debug, Clone, Deserialize)]
pub struct ScreenshotResult {
    /// Source URL.
    pub url: String,

    /// Base64-encoded image data.
    #[serde(default)]
    pub base64: String,

    /// Image format (e.g. "png").
    #[serde(default)]
    pub format: String,

    /// Viewport width used.
    #[serde(default)]
    pub width: i32,

    /// Viewport height used.
    #[serde(default)]
    pub height: i32,

    /// Error message if capture failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// Response from the screenshot endpoint (synchronous batch).
#[derive(Debug, Clone, Deserialize)]
pub struct ScreenshotResponse {
    /// Screenshot results.
    #[serde(default)]
    pub screenshots: Vec<ScreenshotResult>,

    /// Number of screenshots.
    #[serde(default)]
    pub count: i32,
}

/// Response from async screenshot job submission.
#[derive(Debug, Clone, Deserialize)]
pub struct ScreenshotJobResponse {
    /// Job identifier for polling.
    pub job_id: String,

    /// Initial status.
    #[serde(default)]
    pub status: String,

    /// Number of URLs submitted.
    #[serde(default)]
    pub urls: i32,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Submits a doc-scraping job. Returns a job ID for polling.
    pub async fn scrape(&self, req: &ScrapeRequest) -> Result<ScrapeResponse> {
        let (resp, _meta) = self
            .post_json::<ScrapeRequest, ScrapeResponse>("/qai/v1/scraper/scrape", req)
            .await?;
        Ok(resp)
    }

    /// Takes screenshots of URLs. For <=5 URLs, returns results inline.
    /// For >5, returns a job ID for async processing.
    pub async fn screenshot(&self, req: &ScreenshotRequest) -> Result<ScreenshotResponse> {
        let (resp, _meta) = self
            .post_json::<ScreenshotRequest, ScreenshotResponse>("/qai/v1/scraper/screenshot", req)
            .await?;
        Ok(resp)
    }

    /// Submits a large screenshot batch as an async job.
    pub async fn screenshot_job(&self, req: &ScreenshotRequest) -> Result<JobCreateResponse> {
        let params = serde_json::to_value(req)?;
        self.create_job(&JobCreateRequest {
            job_type: "screenshot".into(),
            params,
        })
        .await
    }
}
