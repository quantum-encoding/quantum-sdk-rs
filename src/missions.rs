use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::client::Client;
use crate::error::Result;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for creating a mission.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MissionCreateRequest {
    /// High-level task description.
    pub goal: String,

    /// Strategy: "wave" (default), "dag", "mapreduce", "refinement", "branch".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,

    /// Conductor model (default: claude-sonnet-4-6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_model: Option<String>,

    /// Worker team configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<HashMap<String, MissionWorkerConfig>>,

    /// Maximum orchestration steps (default: 25).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<i32>,

    /// Custom system prompt for the conductor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Existing session ID for context continuity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Worker configuration within a mission.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MissionWorkerConfig {
    /// Model to use for this worker.
    pub model: String,

    /// Cost tier: "cheap", "mid", "expensive".
    #[serde(default)]
    pub tier: String,

    /// Worker description / capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request body for chatting with a mission's architect.
#[derive(Debug, Clone, Serialize)]
pub struct MissionChatRequest {
    /// Message to send to the architect.
    pub message: String,

    /// Enable streaming (not yet supported).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Request body for updating a mission plan.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MissionPlanUpdate {
    /// Updated task list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<HashMap<String, serde_json::Value>>>,

    /// Updated worker configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<HashMap<String, MissionWorkerConfig>>,

    /// Additional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Updated max steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<i32>,

    /// Additional context to inject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Request body for confirming/rejecting a mission structure.
#[derive(Debug, Clone, Serialize)]
pub struct MissionConfirmStructure {
    /// Whether the structure is approved.
    pub confirmed: bool,

    /// Rejection reason or modification notes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<String>,
}

/// Request body for approving a completed mission.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MissionApproveRequest {
    /// Git commit SHA associated with the mission output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,

    /// Approval comment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Request body for importing a plan as a new mission.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MissionImportRequest {
    /// Mission goal.
    pub goal: String,

    /// Strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,

    /// Conductor model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_model: Option<String>,

    /// Worker configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<HashMap<String, MissionWorkerConfig>>,

    /// Pre-defined tasks.
    #[serde(default)]
    pub tasks: Vec<HashMap<String, serde_json::Value>>,

    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Maximum steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<i32>,

    /// Auto-execute after import.
    #[serde(default)]
    pub auto_execute: bool,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from mission creation.
#[derive(Debug, Clone, Deserialize)]
pub struct MissionCreateResponse {
    /// Mission identifier.
    pub mission_id: String,

    /// Initial status.
    #[serde(default)]
    pub status: String,

    /// Session ID for conversation context.
    #[serde(default)]
    pub session_id: Option<String>,

    /// Conductor model used.
    #[serde(default)]
    pub conductor_model: Option<String>,

    /// Strategy used.
    #[serde(default)]
    pub strategy: Option<String>,

    /// Worker configuration.
    #[serde(default)]
    pub workers: Option<HashMap<String, MissionWorkerConfig>>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Request identifier.
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Mission detail (from GET /missions/{id}).
#[derive(Debug, Clone, Deserialize)]
pub struct MissionDetail {
    /// Mission identifier.
    #[serde(default)]
    pub id: Option<String>,

    /// User who created the mission.
    #[serde(default)]
    pub user_id: Option<String>,

    /// Mission goal.
    #[serde(default)]
    pub goal: Option<String>,

    /// Strategy.
    #[serde(default)]
    pub strategy: Option<String>,

    /// Conductor model.
    #[serde(default)]
    pub conductor_model: Option<String>,

    /// Current status.
    #[serde(default)]
    pub status: Option<String>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Start timestamp.
    #[serde(default)]
    pub started_at: Option<String>,

    /// Completion timestamp.
    #[serde(default)]
    pub completed_at: Option<String>,

    /// Error message if failed.
    #[serde(default)]
    pub error: Option<String>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Number of steps executed.
    #[serde(default)]
    pub total_steps: i32,

    /// Session ID.
    #[serde(default)]
    pub session_id: Option<String>,

    /// Final result text.
    #[serde(default)]
    pub result: Option<String>,

    /// Tasks within the mission.
    #[serde(default)]
    pub tasks: Vec<MissionTask>,

    /// Whether the mission was approved.
    #[serde(default)]
    pub approved: bool,

    /// Commit SHA (if approved).
    #[serde(default)]
    pub commit_sha: Option<String>,
}

/// A task within a mission.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MissionTask {
    /// Task identifier.
    #[serde(default)]
    pub id: Option<String>,

    /// Task name.
    #[serde(default)]
    pub name: Option<String>,

    /// Task description.
    #[serde(default)]
    pub description: Option<String>,

    /// Assigned worker name.
    #[serde(default)]
    pub worker: Option<String>,

    /// Model used.
    #[serde(default)]
    pub model: Option<String>,

    /// Task status.
    #[serde(default)]
    pub status: Option<String>,

    /// Task result.
    #[serde(default)]
    pub result: Option<String>,

    /// Error message if failed.
    #[serde(default)]
    pub error: Option<String>,

    /// Step number.
    #[serde(default)]
    pub step: i32,

    /// Input tokens used.
    #[serde(default)]
    pub tokens_in: i32,

    /// Output tokens used.
    #[serde(default)]
    pub tokens_out: i32,
}

/// Response from listing missions.
#[derive(Debug, Clone, Deserialize)]
pub struct MissionListResponse {
    /// List of missions.
    #[serde(default)]
    pub missions: Vec<MissionDetail>,
}

/// Response from chatting with the architect.
#[derive(Debug, Clone, Deserialize)]
pub struct MissionChatResponse {
    /// Mission identifier.
    #[serde(default)]
    pub mission_id: Option<String>,

    /// Architect's response content.
    #[serde(default)]
    pub content: Option<String>,

    /// Model used.
    #[serde(default)]
    pub model: Option<String>,

    /// Cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Token usage.
    #[serde(default)]
    pub usage: Option<MissionChatUsage>,
}

/// Token usage for a mission chat response.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MissionChatUsage {
    #[serde(default)]
    pub input_tokens: i32,
    #[serde(default)]
    pub output_tokens: i32,
}

/// A git checkpoint within a mission.
#[derive(Debug, Clone, Deserialize)]
pub struct MissionCheckpoint {
    /// Checkpoint identifier.
    #[serde(default)]
    pub id: Option<String>,

    /// Commit SHA.
    #[serde(default)]
    pub commit_sha: Option<String>,

    /// Checkpoint message.
    #[serde(default)]
    pub message: Option<String>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Response from listing checkpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct MissionCheckpointsResponse {
    #[serde(default)]
    pub mission_id: Option<String>,
    #[serde(default)]
    pub checkpoints: Vec<MissionCheckpoint>,
}

/// Generic status response for mission operations.
#[derive(Debug, Clone, Deserialize)]
pub struct MissionStatusResponse {
    #[serde(default)]
    pub mission_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub confirmed: Option<bool>,
    #[serde(default)]
    pub approved: Option<bool>,
    #[serde(default)]
    pub deleted: Option<bool>,
    #[serde(default)]
    pub updated: Option<bool>,
    #[serde(default)]
    pub commit_sha: Option<String>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Create and execute a mission asynchronously.
    pub async fn mission_create(&self, req: &MissionCreateRequest) -> Result<MissionCreateResponse> {
        let (resp, _) = self.post_json("/qai/v1/missions/create", req).await?;
        Ok(resp)
    }

    /// List missions for the authenticated user.
    pub async fn mission_list(&self, status: Option<&str>) -> Result<MissionListResponse> {
        let path = match status {
            Some(s) => format!("/qai/v1/missions/list?status={}", s),
            None => "/qai/v1/missions/list".into(),
        };
        let (resp, _) = self.get_json(&path).await?;
        Ok(resp)
    }

    /// Get mission details including tasks.
    pub async fn mission_get(&self, mission_id: &str) -> Result<MissionDetail> {
        let (resp, _) = self.get_json(&format!("/qai/v1/missions/{}", mission_id)).await?;
        Ok(resp)
    }

    /// Delete a mission.
    pub async fn mission_delete(&self, mission_id: &str) -> Result<MissionStatusResponse> {
        let (resp, _) = self.delete_json(&format!("/qai/v1/missions/{}", mission_id)).await?;
        Ok(resp)
    }

    /// Cancel a running mission.
    pub async fn mission_cancel(&self, mission_id: &str) -> Result<MissionStatusResponse> {
        let (resp, _) = self.post_json_empty(&format!("/qai/v1/missions/{}/cancel", mission_id)).await?;
        Ok(resp)
    }

    /// Pause a running mission.
    pub async fn mission_pause(&self, mission_id: &str) -> Result<MissionStatusResponse> {
        let (resp, _) = self.post_json_empty(&format!("/qai/v1/missions/{}/pause", mission_id)).await?;
        Ok(resp)
    }

    /// Resume a paused mission.
    pub async fn mission_resume(&self, mission_id: &str) -> Result<MissionStatusResponse> {
        let (resp, _) = self.post_json_empty(&format!("/qai/v1/missions/{}/resume", mission_id)).await?;
        Ok(resp)
    }

    /// Chat with the mission's architect.
    pub async fn mission_chat(&self, mission_id: &str, req: &MissionChatRequest) -> Result<MissionChatResponse> {
        let (resp, _) = self.post_json(&format!("/qai/v1/missions/{}/chat", mission_id), req).await?;
        Ok(resp)
    }

    /// Retry a failed task.
    pub async fn mission_retry_task(&self, mission_id: &str, task_id: &str) -> Result<MissionStatusResponse> {
        let (resp, _) = self.post_json_empty(&format!("/qai/v1/missions/{}/retry/{}", mission_id, task_id)).await?;
        Ok(resp)
    }

    /// Approve a completed mission.
    pub async fn mission_approve(&self, mission_id: &str, req: &MissionApproveRequest) -> Result<MissionStatusResponse> {
        let (resp, _) = self.post_json(&format!("/qai/v1/missions/{}/approve", mission_id), req).await?;
        Ok(resp)
    }

    /// Update the mission plan.
    pub async fn mission_update_plan(&self, mission_id: &str, req: &MissionPlanUpdate) -> Result<MissionStatusResponse> {
        let (resp, _) = self.put_json(&format!("/qai/v1/missions/{}/plan", mission_id), req).await?;
        Ok(resp)
    }

    /// Confirm or reject the proposed execution structure.
    pub async fn mission_confirm_structure(&self, mission_id: &str, req: &MissionConfirmStructure) -> Result<MissionStatusResponse> {
        let (resp, _) = self.post_json(&format!("/qai/v1/missions/{}/confirm-structure", mission_id), req).await?;
        Ok(resp)
    }

    /// List git checkpoints for a mission.
    pub async fn mission_checkpoints(&self, mission_id: &str) -> Result<MissionCheckpointsResponse> {
        let (resp, _) = self.get_json(&format!("/qai/v1/missions/{}/checkpoints", mission_id)).await?;
        Ok(resp)
    }

    /// Import an existing plan as a new mission.
    pub async fn mission_import(&self, req: &MissionImportRequest) -> Result<MissionCreateResponse> {
        let (resp, _) = self.post_json("/qai/v1/missions/import", req).await?;
        Ok(resp)
    }
}
