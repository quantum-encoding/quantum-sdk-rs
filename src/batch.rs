//! Batch processing — submit multiple prompts in a single request.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() -> quantum_sdk::Result<()> {
//! let client = quantum_sdk::Client::new("qai_key_xxx");
//!
//! let resp = client.batch_submit(&[quantum_sdk::BatchJob {
//!     model: "claude-sonnet-4-6".into(),
//!     prompt: "Summarize quantum computing".into(),
//!     ..Default::default()
//! }]).await?;
//!
//! for id in &resp.job_ids {
//!     println!("Submitted: {id}");
//! }
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// A single job in a batch submission.
#[derive(Debug, Clone, Serialize, Default)]
pub struct BatchJob {
    /// Model to use for this job.
    pub model: String,

    /// The prompt text.
    pub prompt: String,

    /// Optional title for this job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Optional maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
}

/// Response from batch submission.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchSubmitResponse {
    /// The IDs of the created jobs.
    pub job_ids: Vec<String>,

    /// Status of the batch submission.
    #[serde(default)]
    pub status: String,
}

/// Response from JSONL batch submission.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchJsonlResponse {
    /// The IDs of the created jobs.
    pub job_ids: Vec<String>,
}

/// A single job in the batch jobs list.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchJobInfo {
    /// Job identifier.
    pub job_id: String,

    /// Current status (e.g. "pending", "running", "completed", "failed").
    pub status: String,

    /// Model used for this job.
    #[serde(default)]
    pub model: Option<String>,

    /// Job title.
    #[serde(default)]
    pub title: Option<String>,

    /// When the job was created.
    #[serde(default)]
    pub created_at: Option<String>,

    /// When the job completed.
    #[serde(default)]
    pub completed_at: Option<String>,

    /// Result data (present when completed).
    #[serde(default)]
    pub result: Option<serde_json::Value>,

    /// Error message (present when failed).
    #[serde(default)]
    pub error: Option<String>,

    /// Cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,
}

/// Response from listing batch jobs.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchJobsResponse {
    /// The list of batch jobs.
    pub jobs: Vec<BatchJobInfo>,
}

impl Client {
    /// Submit a batch of jobs for processing.
    ///
    /// Each job runs independently and can be polled via the Jobs API.
    pub async fn batch_submit(&self, jobs: &[BatchJob]) -> Result<BatchSubmitResponse> {
        let body = serde_json::json!({ "jobs": jobs });
        let (resp, _meta) = self
            .post_json::<serde_json::Value, BatchSubmitResponse>("/qai/v1/batch", &body)
            .await?;
        Ok(resp)
    }

    /// Submit a batch of jobs using JSONL format.
    ///
    /// Each line in the JSONL string is a JSON object with model, prompt, etc.
    pub async fn batch_submit_jsonl(&self, jsonl: &str) -> Result<BatchJsonlResponse> {
        let body = serde_json::json!({ "jsonl": jsonl });
        let (resp, _meta) = self
            .post_json::<serde_json::Value, BatchJsonlResponse>("/qai/v1/batch/jsonl", &body)
            .await?;
        Ok(resp)
    }

    /// List all batch jobs for the account.
    pub async fn batch_jobs(&self) -> Result<BatchJobsResponse> {
        let (resp, _meta) = self
            .get_json::<BatchJobsResponse>("/qai/v1/batch/jobs")
            .await?;
        Ok(resp)
    }

    /// Get the status and result of a single batch job.
    pub async fn batch_job(&self, id: &str) -> Result<BatchJobInfo> {
        let path = format!("/qai/v1/batch/jobs/{id}");
        let (resp, _meta) = self.get_json::<BatchJobInfo>(&path).await?;
        Ok(resp)
    }
}
