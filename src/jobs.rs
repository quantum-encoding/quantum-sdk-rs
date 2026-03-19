use serde::{Deserialize, Serialize};

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
}

/// Response from job status check.
#[derive(Debug, Clone, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub status: String,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub cost_ticks: i64,
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
            result: None,
            error: Some(format!("Job polling timed out after {max_attempts} attempts")),
            cost_ticks: 0,
        })
    }
}
