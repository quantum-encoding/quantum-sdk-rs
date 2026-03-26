use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::session::ContextConfig;

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// Describes a worker agent in a multi-agent run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentWorker {
    /// Worker name.
    pub name: String,

    /// Model ID for this worker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Worker tier (e.g. "fast", "thinking").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,

    /// Description of this worker's role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request body for an agent run.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AgentRequest {
    /// Session identifier for continuity across runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// The task for the agent to accomplish.
    pub task: String,

    /// Model for the conductor agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_model: Option<String>,

    /// Worker agents available to the conductor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<Vec<AgentWorker>>,

    /// Maximum number of steps before stopping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<i32>,

    /// System prompt for the conductor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

// ---------------------------------------------------------------------------
// Mission
// ---------------------------------------------------------------------------

/// Describes a named worker for a mission (map keyed by name).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MissionWorker {
    /// Model ID for this worker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Worker tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,

    /// Description of this worker's purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request body for a mission run.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MissionRequest {
    /// The high-level goal for the mission.
    pub goal: String,

    /// Execution strategy hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,

    /// Model for the conductor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_model: Option<String>,

    /// Named workers (key = worker name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<HashMap<String, MissionWorker>>,

    /// Maximum number of steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<i32>,

    /// System prompt for the conductor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Session identifier for continuity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Whether to auto-plan before execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_plan: Option<bool>,

    /// Context management configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_config: Option<ContextConfig>,

    /// Model for worker nodes (codegen strategy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_model: Option<String>,

    /// Deployment ID — route worker inference to a managed Vertex endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,

    /// Build command to run after codegen (e.g. "cargo build", "npm run build").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,

    /// Workspace directory for generated files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
}

/// Backwards-compatible alias for [`AgentWorker`].
pub type AgentWorkerConfig = AgentWorker;

/// Backwards-compatible alias for [`MissionWorker`].
pub type MissionWorkerConfig = MissionWorker;

// ---------------------------------------------------------------------------
// SSE Stream
// ---------------------------------------------------------------------------

/// A single event from an agent or mission SSE stream.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentStreamEvent {
    /// Event type (e.g. "step", "thought", "tool_call", "tool_result", "message", "error", "done").
    #[serde(rename = "type", default)]
    pub event_type: String,

    /// Raw JSON payload for caller to interpret.
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

/// A single SSE event from an agent run stream.
/// Alias for [`AgentStreamEvent`] for backwards compatibility.
pub type AgentEvent = AgentStreamEvent;

/// A single SSE event from a mission run stream.
/// Alias for [`AgentStreamEvent`] since both use the same SSE format.
pub type MissionEvent = AgentStreamEvent;

pin_project! {
    /// An async stream of [`AgentStreamEvent`]s from an agent or mission SSE response.
    pub struct AgentStream {
        #[pin]
        inner: Pin<Box<dyn Stream<Item = AgentStreamEvent> + Send>>,
    }
}

impl Stream for AgentStream {
    type Item = AgentStreamEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

/// Converts a byte stream into a stream of parsed [`AgentStreamEvent`]s.
fn sse_to_agent_events<S>(byte_stream: S) -> impl Stream<Item = AgentStreamEvent> + Send
where
    S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
{
    let pinned_stream = Box::pin(byte_stream);

    let line_stream = futures_util::stream::unfold(
        (pinned_stream, String::new()),
        |(mut stream, mut buffer)| async move {
            use futures_util::StreamExt;
            loop {
                if let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
                    buffer = buffer[newline_pos + 1..].to_string();
                    return Some((line, (stream, buffer)));
                }

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                    }
                    Some(Err(_)) | None => {
                        if !buffer.is_empty() {
                            let remaining = std::mem::take(&mut buffer);
                            return Some((remaining, (stream, buffer)));
                        }
                        return None;
                    }
                }
            }
        },
    );

    let pinned_lines = Box::pin(line_stream);
    futures_util::stream::unfold(pinned_lines, |mut lines| async move {
        use futures_util::StreamExt;
        loop {
            let line = lines.next().await?;

            if !line.starts_with("data: ") {
                continue;
            }
            let payload = &line["data: ".len()..];

            if payload == "[DONE]" {
                let ev = AgentStreamEvent {
                    event_type: "done".to_string(),
                    data: HashMap::new(),
                };
                return Some((ev, lines));
            }

            match serde_json::from_str::<AgentStreamEvent>(payload) {
                Ok(ev) => return Some((ev, lines)),
                Err(e) => {
                    let mut data = HashMap::new();
                    data.insert(
                        "error".to_string(),
                        serde_json::Value::String(format!("parse SSE: {e}")),
                    );
                    let ev = AgentStreamEvent {
                        event_type: "error".to_string(),
                        data,
                    };
                    return Some((ev, lines));
                }
            }
        }
    })
}

impl Client {
    /// Starts an agent run and returns an SSE event stream.
    ///
    /// The agent orchestrates one or more worker models to accomplish the task,
    /// streaming progress events as it works.
    pub async fn agent_run(&self, req: &AgentRequest) -> Result<AgentStream> {
        let (resp, _meta) = self.post_stream_raw("/qai/v1/agent", req).await?;
        let byte_stream = resp.bytes_stream();
        let event_stream = sse_to_agent_events(byte_stream);
        Ok(AgentStream {
            inner: Box::pin(event_stream),
        })
    }

    /// Starts a mission run and returns an SSE event stream.
    ///
    /// Missions are higher-level than agents -- they can auto-plan, assign
    /// named workers, and manage context across multiple steps.
    pub async fn mission_run(&self, req: &MissionRequest) -> Result<AgentStream> {
        let (resp, _meta) = self.post_stream_raw("/qai/v1/missions", req).await?;
        let byte_stream = resp.bytes_stream();
        let event_stream = sse_to_agent_events(byte_stream);
        Ok(AgentStream {
            inner: Box::pin(event_stream),
        })
    }
}
