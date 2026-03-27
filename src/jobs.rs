use serde::{Deserialize, Serialize};

use crate::chat::ChatRequest;
use crate::client::Client;
use crate::error::Result;

/// Request to create an async job.
#[derive(Debug, Clone, Serialize)]
pub struct JobCreateRequest {
    /// Job type (e.g. "video/generate", "audio/music").
    #[serde(rename = "type")]
    pub job_type: String,

    /// Job parameters (model-specific).
    pub params: serde_json::Value,
}

/// Response from job creation.
#[derive(Debug, Clone, Deserialize)]
pub struct JobCreateResponse {
    pub job_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "type", default)]
    pub job_type: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Response from job status check.
#[derive(Debug, Clone, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub status: String,
    #[serde(rename = "type", default)]
    pub job_type: Option<String>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub cost_ticks: i64,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
}

/// Summary of a job in the list response.
#[derive(Debug, Clone, Deserialize)]
pub struct JobSummary {
    pub job_id: String,
    pub status: String,
    #[serde(rename = "type", default)]
    pub job_type: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub cost_ticks: i64,
}

/// Response from listing jobs.
#[derive(Debug, Clone, Deserialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<JobSummary>,
}

/// A single SSE event from a job stream.
#[derive(Debug, Clone, Deserialize)]
pub struct JobStreamEvent {
    /// Event type (e.g. "progress", "complete", "error").
    #[serde(rename = "type", default)]
    pub event_type: String,

    /// Job identifier.
    #[serde(default)]
    pub job_id: Option<String>,

    /// Job status.
    #[serde(default)]
    pub status: Option<String>,

    /// Job result (on completion).
    #[serde(default)]
    pub result: Option<serde_json::Value>,

    /// Error message (on failure).
    #[serde(default)]
    pub error: Option<String>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Completion timestamp.
    #[serde(default)]
    pub completed_at: Option<String>,
}

/// Response from async job submission (HeyGen, 3D, etc.).
/// The client should poll the job status using `get_job`.
#[derive(Debug, Clone, Deserialize)]
pub struct JobAcceptedResponse {
    /// Unique job identifier for polling.
    pub job_id: String,

    /// Initial job status (e.g. "pending").
    #[serde(default)]
    pub status: String,

    /// Job type.
    #[serde(rename = "type", default)]
    pub job_type: Option<String>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: Option<String>,
}

/// A single job entry in the detailed job list response.
#[derive(Debug, Clone, Deserialize)]
pub struct JobListEntry {
    /// Unique job identifier.
    pub job_id: String,

    /// Job type (e.g. "video/generate", "audio/tts").
    #[serde(rename = "type", default)]
    pub job_type: Option<String>,

    /// Job status ("pending", "processing", "completed", "failed").
    pub status: String,

    /// Job output when completed.
    #[serde(default)]
    pub result: Option<serde_json::Value>,

    /// Error message if the job failed.
    #[serde(default)]
    pub error: Option<String>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Job creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// When processing began.
    #[serde(default)]
    pub started_at: Option<String>,

    /// When the job finished.
    #[serde(default)]
    pub completed_at: Option<String>,

    /// Originating request identifier.
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Response from listing jobs (detailed variant).
#[derive(Debug, Clone, Deserialize)]
pub struct JobListResponse {
    /// The list of jobs.
    pub jobs: Vec<JobListEntry>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: Option<String>,
}

impl Client {
    /// Creates an async job. Returns the job ID for polling.
    pub async fn create_job(&self, req: &JobCreateRequest) -> Result<JobCreateResponse> {
        let (resp, _meta) = self
            .post_json::<JobCreateRequest, JobCreateResponse>("/qai/v1/jobs", req)
            .await?;
        Ok(resp)
    }

    /// Checks the status of an async job.
    pub async fn get_job(&self, job_id: &str) -> Result<JobStatusResponse> {
        let path = format!("/qai/v1/jobs/{job_id}");
        let (resp, _meta) = self.get_json::<JobStatusResponse>(&path).await?;
        Ok(resp)
    }

    /// Lists all jobs for the account.
    pub async fn list_jobs(&self) -> Result<ListJobsResponse> {
        let (resp, _meta) = self
            .get_json::<ListJobsResponse>("/qai/v1/jobs")
            .await?;
        Ok(resp)
    }

    /// Opens an SSE stream for a job, returning the raw response.
    /// Events: progress, complete, error. Auto-closes on terminal state.
    pub async fn stream_job(
        &self,
        job_id: &str,
    ) -> Result<reqwest::Response> {
        let path = format!("/qai/v1/jobs/{job_id}/stream");
        let (resp, _meta) = self.get_stream_raw(&path).await?;
        Ok(resp)
    }

    /// Polls a job until completion or timeout.
    /// Returns the final status response.
    pub async fn poll_job(
        &self,
        job_id: &str,
        poll_interval: std::time::Duration,
        max_attempts: usize,
    ) -> Result<JobStatusResponse> {
        for _ in 0..max_attempts {
            tokio::time::sleep(poll_interval).await;
            let status = self.get_job(job_id).await?;
            match status.status.as_str() {
                "completed" | "failed" => return Ok(status),
                _ => continue,
            }
        }
        Ok(JobStatusResponse {
            job_id: job_id.to_string(),
            status: "timeout".to_string(),
            job_type: None,
            result: None,
            error: Some(format!("Job polling timed out after {max_attempts} attempts")),
            cost_ticks: 0,
            request_id: None,
            created_at: None,
            started_at: None,
            completed_at: None,
        })
    }

    /// Convenience method for 3D model generation via the async jobs system.
    ///
    /// Submits a job with type `"3d/generate"` and the given parameters.
    /// Returns the job creation response -- use `poll_job` to wait for completion.
    pub async fn generate_3d(
        &self,
        model: &str,
        prompt: Option<&str>,
        image_url: Option<&str>,
    ) -> Result<JobCreateResponse> {
        let mut params = serde_json::json!({ "model": model });
        if let Some(p) = prompt {
            params["prompt"] = serde_json::Value::String(p.to_string());
        }
        if let Some(u) = image_url {
            params["image_url"] = serde_json::Value::String(u.to_string());
        }
        let req = JobCreateRequest {
            job_type: "3d/generate".to_string(),
            params,
        };
        self.create_job(&req).await
    }

    /// Submits a chat completion as an async job.
    ///
    /// Useful for long-running models (e.g. Opus) where synchronous `/qai/v1/chat`
    /// may time out. Params are the same shape as [`ChatRequest`].
    /// Use [`stream_job()`] or [`poll_job()`] to get the result.
    pub async fn chat_job(&self, req: &ChatRequest) -> Result<JobCreateResponse> {
        let params = serde_json::to_value(req)?;
        let job_req = JobCreateRequest {
            job_type: "chat".to_string(),
            params,
        };
        self.create_job(&job_req).await
    }
}
