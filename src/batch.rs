//! Batch processing — submit multiple prompts in a single request, run at
//! batch (lower-priority, discounted) pricing.
//!
//! Batch jobs live in their own store and are read back with
//! [`Client::batch_jobs`] / [`Client::batch_job`], not the Jobs API
//! (`GET /qai/v1/jobs/{id}` answers 404 for a batch id).
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
//!     let job = client.batch_job(id).await?;
//!     println!("{id}: {}", job.status);
//! }
//! # Ok(())
//! # }
//! ```

use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};

use crate::client::{Client, parse_api_error};
use crate::error::Result;
use crate::serde_util::null_as_default;

/// A single job in a batch submission.
#[derive(Debug, Clone, Serialize, Default)]
pub struct BatchJob {
    /// Model to use for this job (must be a priced model, else the whole
    /// submission is rejected with 400).
    pub model: String,

    /// The prompt text.
    pub prompt: String,

    /// Optional title for this job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Optional system prompt (prepended to the prompt server-side).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Optional maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
}

/// Response from batch submission (202 Accepted).
///
/// `job_ids` can be shorter than the input: jobs with an empty `model` or
/// `prompt` are skipped silently, as is any job whose store write failed,
/// and there is no per-index error. Ids are in input order among the
/// accepted jobs, so keep your own mapping when the counts differ.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchSubmitResponse {
    /// The IDs of the created jobs (in input order among accepted jobs).
    #[serde(default, deserialize_with = "null_as_default")]
    pub job_ids: Vec<String>,

    /// Number of jobs created (equals `job_ids.len()`).
    #[serde(default)]
    pub jobs: i64,

    /// Submission label derived from the accepted count.
    #[serde(default)]
    pub batch_id: String,

    /// Pricing note from the gateway.
    #[serde(default)]
    pub pricing: String,

    /// Status of the batch submission (always "queued").
    #[serde(default)]
    pub status: String,
}

/// Alias of [`BatchSubmitResponse`]: the JSONL route reuses the array
/// handler and answers with the same envelope.
pub type BatchJsonlResponse = BatchSubmitResponse;

/// A batch job as stored by the gateway (`internal/batch.Job`).
#[derive(Debug, Clone, Deserialize)]
pub struct BatchJobInfo {
    /// Job identifier.
    pub id: String,

    /// "queued" | "running" | "paused" | "complete" | "failed" | "cancelled".
    pub status: String,

    /// Queue priority (batch jobs share one low priority).
    #[serde(default)]
    pub priority: i32,

    /// Job kind ("user_batch" for SDK submissions). Wire field: `type`.
    #[serde(rename = "type", default)]
    pub job_type: String,

    /// Job title (the submitted one, or one derived from the prompt).
    #[serde(default)]
    pub title: String,

    /// Prompt as stored (with any system prompt prepended).
    #[serde(default)]
    pub prompt: String,

    /// Model used for this job.
    #[serde(default)]
    pub model: String,

    /// Model output (present when complete and stored inline).
    #[serde(default)]
    pub output: Option<String>,

    /// GCS location of the output when it was too large to inline.
    #[serde(default)]
    pub output_gcs: Option<String>,

    /// Error message (present when failed).
    #[serde(default)]
    pub error: Option<String>,

    /// Submitting user id.
    #[serde(default)]
    pub created_by: String,

    /// When the job was created (RFC 3339).
    #[serde(default)]
    pub created_at: String,

    /// When processing began.
    #[serde(default)]
    pub started_at: Option<String>,

    /// When the job finished.
    #[serde(default)]
    pub completed_at: Option<String>,

    /// Tokens consumed (present once complete).
    #[serde(default)]
    pub tokens: i64,
}

/// Response from listing batch jobs.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchJobsResponse {
    /// The caller's batch jobs (see [`Client::batch_jobs`] for the window).
    #[serde(default, deserialize_with = "null_as_default")]
    pub jobs: Vec<BatchJobInfo>,
}

/// Alias for [`BatchJob`] — a single input in a batch submission.
pub type BatchJobInput = BatchJob;

/// Request body for batch submission (wraps a list of jobs).
#[derive(Debug, Clone, Serialize, Default)]
pub struct BatchSubmitRequest {
    /// Jobs to submit (1–100).
    pub jobs: Vec<BatchJob>,
}

impl Client {
    /// Submits a batch of jobs for processing (1–100 per call).
    ///
    /// The gateway requires a balance of at least a small per-job minimum
    /// (402 otherwise) and rejects the whole batch when any job names an
    /// unpriced model (400). Each accepted job runs independently; read
    /// results with [`Client::batch_job`], not the Jobs API. See
    /// [`BatchSubmitResponse`] for why `job_ids` may be shorter than the
    /// input.
    pub async fn batch_submit(&self, jobs: &[BatchJob]) -> Result<BatchSubmitResponse> {
        let body = BatchSubmitRequest {
            jobs: jobs.to_vec(),
        };
        let (resp, _meta) = self
            .post_json::<BatchSubmitRequest, BatchSubmitResponse>("/qai/v1/batch", &body)
            .await?;
        Ok(resp)
    }

    /// Submits a batch of jobs as raw JSONL: the body is the text itself,
    /// one JSON object per line in the [`BatchJob`] shape (up to 1 MiB).
    /// Blank lines and lines starting with `#` are ignored; lines that fail
    /// to parse or lack `model`/`prompt` are dropped, and a body with no
    /// valid line is rejected with 400.
    pub async fn batch_submit_jsonl(&self, jsonl: &str) -> Result<BatchJsonlResponse> {
        let url = format!("{}/qai/v1/batch/jsonl", self.base_url());
        let resp = self
            .http()
            .post(&url)
            .header(CONTENT_TYPE, "application/x-ndjson")
            .body(jsonl.to_owned())
            .send()
            .await?;
        let request_id = resp
            .headers()
            .get("X-QAI-Request-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        if !resp.status().is_success() {
            return Err(parse_api_error(resp, &request_id).await);
        }
        Ok(resp.json::<BatchJsonlResponse>().await?)
    }

    /// Lists the caller's batch jobs.
    ///
    /// The gateway reads the newest 100 batch jobs across all users and
    /// filters to the caller, so a caller's older jobs drop out of this
    /// list once other users' jobs push them past that window.
    pub async fn batch_jobs(&self) -> Result<BatchJobsResponse> {
        let (resp, _meta) = self
            .get_json::<BatchJobsResponse>("/qai/v1/batch/jobs")
            .await?;
        Ok(resp)
    }

    /// Gets the status and output of a single batch job.
    ///
    /// The lookup scans the newest 200 batch jobs across all users; a
    /// caller's job older than that window answers 404 even though it
    /// exists. Read the output promptly once `status == "complete"`.
    pub async fn batch_job(&self, id: &str) -> Result<BatchJobInfo> {
        let path = format!("/qai/v1/batch/jobs/{id}");
        let (resp, _meta) = self.get_json::<BatchJobInfo>(&path).await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_body_wraps_jobs_with_the_handler_keys() {
        let body = BatchSubmitRequest {
            jobs: vec![BatchJob {
                model: "claude-sonnet-4-6".into(),
                prompt: "hi".into(),
                system_prompt: Some("be brief".into()),
                ..Default::default()
            }],
        };
        let json = serde_json::to_value(&body).unwrap();
        let job = &json["jobs"][0];
        let mut keys: Vec<&str> = job
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["model", "prompt", "system_prompt"]);
    }

    #[test]
    fn submit_response_decodes_the_202_envelope() {
        let resp: BatchSubmitResponse = serde_json::from_str(
            r#"{"batch_id":"batch_2","jobs":2,"job_ids":["a","b"],
                "pricing":"50% of real-time rates","status":"queued"}"#,
        )
        .unwrap();
        assert_eq!(resp.job_ids, ["a", "b"]);
        assert_eq!(resp.jobs, 2);

        // Every input skipped: job_ids is a nil slice → null.
        let none: BatchSubmitResponse = serde_json::from_str(
            r#"{"batch_id":"batch_0","jobs":0,"job_ids":null,"pricing":"","status":"queued"}"#,
        )
        .unwrap();
        assert!(none.job_ids.is_empty());
    }

    #[test]
    fn batch_job_decodes_the_store_shape() {
        let job: BatchJobInfo = serde_json::from_str(
            r#"{"id":"abc123","priority":10,"type":"user_batch","title":"Batch 1/1: hi",
                "prompt":"hi","model":"claude-sonnet-4-6","status":"complete",
                "output":"hello","created_by":"user_1",
                "created_at":"2026-09-05T10:00:00Z","started_at":"2026-09-05T10:00:05Z",
                "completed_at":"2026-09-05T10:00:09Z","tokens":42}"#,
        )
        .unwrap();
        assert_eq!(job.id, "abc123");
        assert_eq!(job.status, "complete");
        assert_eq!(job.output.as_deref(), Some("hello"));
        assert!(job.output_gcs.is_none());

        let list: BatchJobsResponse = serde_json::from_str(r#"{"jobs":null}"#).unwrap();
        assert!(list.jobs.is_empty());

        let queued: BatchJobInfo = serde_json::from_str(
            r#"{"id":"q","priority":10,"type":"user_batch","title":"t","prompt":"p",
                "model":"m","status":"queued","created_by":"u","created_at":"2026-09-05T10:00:00Z"}"#,
        )
        .unwrap();
        assert!(queued.completed_at.is_none());
    }
}
