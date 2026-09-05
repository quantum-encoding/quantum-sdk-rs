use serde::{Deserialize, Serialize};

use crate::chat::ChatRequest;
use crate::client::Client;
use crate::error::{ApiError, Error, Result};
use crate::serde_util::null_as_default;

/// Request to create an async job.
#[derive(Debug, Clone, Serialize)]
pub struct JobCreateRequest {
    /// Job type (e.g. "video/generate", "audio/music").
    #[serde(rename = "type")]
    pub job_type: String,

    /// Job parameters (model-specific).
    pub params: serde_json::Value,
}

/// The 202 envelope every async submission answers with: the Jobs API,
/// the HeyGen video routes (studio, translate, photo-avatar, twin-video,
/// template render) and the 3D pipeline. Poll with [`Client::get_job`] /
/// [`Client::poll_job`] or subscribe with [`Client::stream_job`].
///
/// No cost appears here; `cost_ticks` is reported by [`Client::get_job`]
/// once the job has settled.
#[derive(Debug, Clone, Deserialize)]
pub struct JobAcceptedResponse {
    /// Unique job identifier for polling.
    pub job_id: String,

    /// Initial job status (always "pending").
    #[serde(default)]
    pub status: String,

    /// Job type (e.g. "video/studio", "3d/generate").
    #[serde(rename = "type", default)]
    pub job_type: Option<String>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: Option<String>,

    /// Creation timestamp (RFC 3339). Sent by `POST /qai/v1/jobs`; the
    /// dedicated media routes omit it.
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Alias of [`JobAcceptedResponse`]: `POST /qai/v1/jobs` answers with the
/// same envelope as every other async route.
pub type JobCreateResponse = JobAcceptedResponse;

/// A job as reported by `GET /qai/v1/jobs/{id}` and, per entry, by
/// `GET /qai/v1/jobs`.
#[derive(Debug, Clone, Deserialize)]
pub struct JobStatusResponse {
    /// Unique job identifier.
    pub job_id: String,

    /// "pending" | "running" | "completed" | "failed". Only the last two
    /// are terminal.
    pub status: String,

    /// Job type (e.g. "video/generate", "audio/tts").
    #[serde(rename = "type", default)]
    pub job_type: Option<String>,

    /// Job output when completed. Results stored in GCS are inlined here.
    #[serde(default)]
    pub result: Option<serde_json::Value>,

    /// Error message if the job failed.
    #[serde(default)]
    pub error: Option<String>,

    /// Total cost in ticks (0 until the job has settled).
    #[serde(default)]
    pub cost_ticks: i64,

    /// Originating request identifier.
    #[serde(default)]
    pub request_id: Option<String>,

    /// Job creation timestamp (RFC 3339).
    #[serde(default)]
    pub created_at: Option<String>,

    /// When processing began.
    #[serde(default)]
    pub started_at: Option<String>,

    /// When the job finished.
    #[serde(default)]
    pub completed_at: Option<String>,
}

/// Alias of [`JobStatusResponse`]: list entries carry the same fields as a
/// single status read.
pub type JobListEntry = JobStatusResponse;

/// Response from listing jobs.
#[derive(Debug, Clone, Deserialize)]
pub struct JobListResponse {
    /// The caller's newest jobs (at most 50).
    #[serde(default, deserialize_with = "null_as_default")]
    pub jobs: Vec<JobStatusResponse>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Alias of [`JobListResponse`].
pub type ListJobsResponse = JobListResponse;

/// A single SSE event from a job stream.
///
/// `error` events come in two flavours: a job failure carries `job_id`,
/// `status: "failed"` and the job's `error`; the stream's own 10-minute
/// deadline emits `{"type":"error","error":"stream timeout (10 minutes)"}`
/// with no `job_id` or `status`, and the job keeps running — reopen the
/// stream or fall back to [`Client::poll_job`].
#[derive(Debug, Clone, Deserialize)]
pub struct JobStreamEvent {
    /// Event type: "progress", "complete", "error".
    #[serde(rename = "type", default)]
    pub event_type: String,

    /// Job identifier (absent on a stream-timeout error).
    #[serde(default)]
    pub job_id: Option<String>,

    /// Job status.
    #[serde(default)]
    pub status: Option<String>,

    /// Job result (on completion).
    #[serde(default)]
    pub result: Option<serde_json::Value>,

    /// Error message (on failure or stream timeout).
    #[serde(default)]
    pub error: Option<String>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Completion timestamp.
    #[serde(default)]
    pub completed_at: Option<String>,
}

impl JobStreamEvent {
    /// True when this `error` event is the stream's own deadline rather
    /// than a job failure; the job is still running.
    pub fn is_stream_timeout(&self) -> bool {
        self.event_type == "error" && self.job_id.is_none() && self.status.is_none()
    }
}

impl Client {
    /// Creates an async job (202). Returns the job envelope for polling.
    pub async fn create_job(&self, req: &JobCreateRequest) -> Result<JobAcceptedResponse> {
        let (resp, _meta) = self
            .post_json::<JobCreateRequest, JobAcceptedResponse>("/qai/v1/jobs", req)
            .await?;
        Ok(resp)
    }

    /// Checks the status of an async job. Batch jobs live in a different
    /// store: read those with [`Client::batch_job`].
    pub async fn get_job(&self, job_id: &str) -> Result<JobStatusResponse> {
        let path = format!("/qai/v1/jobs/{job_id}");
        let (resp, _meta) = self.get_json::<JobStatusResponse>(&path).await?;
        Ok(resp)
    }

    /// Lists the caller's newest 50 jobs. Older jobs are not reachable
    /// through this route; keep the ids from submission if you need them.
    pub async fn list_jobs(&self) -> Result<JobListResponse> {
        let (resp, _meta) = self.get_json::<JobListResponse>("/qai/v1/jobs").await?;
        Ok(resp)
    }

    /// Opens an SSE stream for a job, returning the raw response.
    ///
    /// Events are `data: {json}` lines decoding to [`JobStreamEvent`]:
    /// "progress" on each status change, then one "complete" or "error",
    /// after which the stream closes. The stream also closes after 10
    /// minutes with an `error` event whose `job_id` is absent (see
    /// [`JobStreamEvent::is_stream_timeout`]); the job itself continues.
    pub async fn stream_job(&self, job_id: &str) -> Result<reqwest::Response> {
        let path = format!("/qai/v1/jobs/{job_id}/stream");
        let (resp, _meta) = self.get_stream_raw(&path).await?;
        Ok(resp)
    }

    /// Polls a job until it reports "completed" or "failed".
    ///
    /// Each attempt sleeps for `poll_interval` before checking, so the first
    /// status read happens one interval after the call. A "failed" job is
    /// returned as `Ok` with `status == "failed"`. When `max_attempts` runs
    /// out the result is `Err` with an [`ApiError`] whose `code` is
    /// `"poll_timeout"` and whose `status_code` is 0 (raised locally, no
    /// HTTP status); the job keeps running and can be polled again.
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
        Err(Error::Api(ApiError {
            status_code: 0,
            code: "poll_timeout".into(),
            message: format!(
                "job {job_id} still running after {max_attempts} polls of {poll_interval:?}"
            ),
            request_id: String::new(),
        }))
    }

    /// Convenience method for 3D model generation via the async jobs system.
    ///
    /// Submits a job with type `"3d/generate"` and the given parameters.
    /// Returns the job envelope -- use `poll_job` to wait for completion.
    pub async fn generate_3d(
        &self,
        model: &str,
        prompt: Option<&str>,
        image_url: Option<&str>,
    ) -> Result<JobAcceptedResponse> {
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
    /// Use [`Client::stream_job`] or [`Client::poll_job`] to get the result.
    pub async fn chat_job(&self, req: &ChatRequest) -> Result<JobAcceptedResponse> {
        let params = serde_json::to_value(req)?;
        let job_req = JobCreateRequest {
            job_type: "chat".to_string(),
            params,
        };
        self.create_job(&job_req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_envelope_decodes_both_producers() {
        // POST /qai/v1/jobs (jobCreateResponse) carries created_at.
        let created: JobAcceptedResponse = serde_json::from_str(
            r#"{"job_id":"qai_job_1","status":"pending","type":"3d/generate",
                "created_at":"2026-09-05T10:00:00Z","request_id":"req_1"}"#,
        )
        .unwrap();
        assert_eq!(created.created_at.as_deref(), Some("2026-09-05T10:00:00Z"));

        // The HeyGen routes send job_id/status/type/request_id only.
        let media: JobAcceptedResponse = serde_json::from_str(
            r#"{"job_id":"qai_job_2","status":"pending","type":"video/studio","request_id":"req_2"}"#,
        )
        .unwrap();
        assert_eq!(media.job_type.as_deref(), Some("video/studio"));
        assert!(media.created_at.is_none());
    }

    #[test]
    fn list_response_uses_status_entries_and_tolerates_null() {
        let list: JobListResponse = serde_json::from_str(
            r#"{"jobs":[{"job_id":"j1","type":"chat","status":"running",
                "created_at":"2026-09-05T10:00:00Z","request_id":"r1"}],"request_id":"req_l"}"#,
        )
        .unwrap();
        assert_eq!(list.jobs[0].status, "running");
        assert_eq!(list.jobs[0].cost_ticks, 0);

        let empty: JobListResponse =
            serde_json::from_str(r#"{"jobs":null,"request_id":"req_e"}"#).unwrap();
        assert!(empty.jobs.is_empty());
    }

    #[test]
    fn stream_timeout_is_distinguishable_from_failure() {
        let timeout: JobStreamEvent =
            serde_json::from_str(r#"{"type":"error","error":"stream timeout (10 minutes)"}"#)
                .unwrap();
        assert!(timeout.is_stream_timeout());

        let failed: JobStreamEvent = serde_json::from_str(
            r#"{"type":"error","job_id":"j1","status":"failed","error":"boom"}"#,
        )
        .unwrap();
        assert!(!failed.is_stream_timeout());
    }
}
