//! Sandbox-backed agent orchestration.
//!
//! `POST /qai/v1/cloudrun` runs the whole conductor/worker loop server-side —
//! tool calls, shell commands included, execute in the gateway's sandbox rather
//! than on the client — and streams progress back as SSE. Use
//! [`Client::agent_step`](crate::Client::agent_step) instead when the client
//! executes the tool calls itself: that route is a single non-streaming turn
//! that hands the tool calls back.
//!
//! Events arrive as [`AgentStreamEvent`]s. The `type` field carries
//! `agent_session` (the opening event naming the conductor, workers, and step
//! budget), `agent_step` per conductor step, `agent_result` with the final
//! content and per-tier token totals, `agent_error`, and the two guard-halt
//! events `agent_budget_exhausted` / `agent_budget_check_unavailable` — a halt
//! bills only the steps that completed.

use serde::Serialize;

use crate::agent::{AgentStream, AgentStreamEvent};
use crate::client::Client;
use crate::error::Result;
use crate::session::ContextConfig;

/// A worker in a Cloud Run agent team.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CloudRunWorker {
    /// Worker name the conductor delegates by.
    pub name: String,

    /// Model this worker runs on.
    pub model: String,

    /// Cost tier: `"cheap"`, `"mid"`, or `"expensive"`. Used only by the
    /// in-loop budget guard, which prices each tier from the worker
    /// registered against it. The final charge ignores tiers: every token
    /// of the run, workers included, is billed at the conductor model's
    /// rate.
    pub tier: String,

    /// What this worker is for — the conductor reads it when delegating.
    pub description: String,
}

/// Request body for `POST /qai/v1/cloudrun`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CloudRunRequest {
    /// Conversation session to continue. A new one is created when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// The task to accomplish. Required.
    pub task: String,

    /// Model the conductor plans and delegates with. Always billed at the
    /// expensive tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_model: Option<String>,

    /// The agent team. A default team is used when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<Vec<CloudRunWorker>>,

    /// Conductor steps to allow. Defaults to 10 and is clamped to a hard
    /// ceiling of 30; the opening `agent_session` event reports both the
    /// requested and the effective value when the clamp fires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<i32>,

    /// System prompt for the conductor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Context management for a newly created session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_config: Option<ContextConfig>,

    /// Tool capability allowlist. Three-state: omitted gives the full tool
    /// suite, `Some(vec![])` gives zero tools (safe mode), and a non-empty
    /// list restricts to those capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,

    /// Directory on the gateway's own filesystem to use as the worker
    /// workspace, relative to the caller's per-user workspace root. An
    /// absolute path or any `..` segment is rejected with 400
    /// `invalid_workspace_path`; it never names a directory on the caller's
    /// machine. An ephemeral per-session directory is used when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
}

impl Client {
    /// Starts a sandbox-backed agent run and returns its SSE event stream.
    ///
    /// `POST /qai/v1/cloudrun`
    pub async fn cloudrun(&self, req: &CloudRunRequest) -> Result<AgentStream> {
        let (resp, _meta) = self.post_stream_raw("/qai/v1/cloudrun", req).await?;
        Ok(AgentStream::from_response(resp))
    }
}

/// Re-exported so callers can name the event type they receive from
/// [`Client::cloudrun`] without also importing the agent module.
pub type CloudRunEvent = AgentStreamEvent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_mode_sends_an_empty_capability_list() {
        let req = CloudRunRequest {
            task: "audit the repo".into(),
            capabilities: Some(Vec::new()),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["capabilities"], serde_json::json!([]));
        assert!(json.get("workspace_path").is_none());
    }

    #[test]
    fn omitted_capabilities_stay_absent_from_the_body() {
        let req = CloudRunRequest {
            task: "audit the repo".into(),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert!(json.get("capabilities").is_none());
        assert_eq!(json["task"], "audit the repo");
    }

    #[test]
    fn workers_serialize_with_their_tiers() {
        let req = CloudRunRequest {
            task: "port the module".into(),
            workers: Some(vec![CloudRunWorker {
                name: "coder".into(),
                model: "claude-sonnet-4-6".into(),
                tier: "mid".into(),
                description: "writes code".into(),
            }]),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["workers"][0]["tier"], "mid");
    }
}
