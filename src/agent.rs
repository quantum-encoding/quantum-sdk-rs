//! Agent turns and mission runs.
//!
//! Two different surfaces live here:
//!
//! - [`Client::agent_step`] is `POST /qai/v1/agent`: one non-streaming model
//!   turn with tool-call passthrough. The gateway never executes tools. It
//!   returns whatever `tool_use` blocks the model produced; the client runs
//!   them itself and sends the results back as `tool` messages on the next
//!   call. This is the loop a native app with its own tool sandbox drives.
//! - [`Client::mission_run`] is `POST /qai/v1/missions`: a server-side
//!   conductor/worker run, streamed back as SSE [`AgentStreamEvent`]s.
//!
//! For a server-side loop that also executes tools in the gateway's sandbox,
//! see [`Client::cloudrun`](crate::Client::cloudrun).

use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::serde_util::null_as_default;
use crate::session::ContextConfig;

// ---------------------------------------------------------------------------
// Agent (single turn, client-executed tools)
// ---------------------------------------------------------------------------

/// A tool call made by the model.
///
/// Returned in [`AgentResponse::tool_use`], and sent back on an `assistant`
/// [`AgentMessage`] when the conversation is replayed so the model sees its
/// own call before the `tool` result that answers it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentToolUse {
    /// Call identifier. A `tool` message answers it via
    /// [`AgentMessage::tool_call_id`].
    #[serde(default)]
    pub id: String,

    /// Name of the tool called.
    #[serde(default)]
    pub name: String,

    /// Parsed arguments, as a JSON object.
    #[serde(default = "empty_object")]
    pub input: serde_json::Value,
}

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// One message in an agent conversation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMessage {
    /// `"user"`, `"assistant"`, `"tool"`, or `"system"`. `system` messages
    /// are folded into the system prompt; an unknown role is sent as `user`.
    pub role: String,

    /// Text content. For a `tool` message this is the tool's output.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content: String,

    /// For `tool` messages: the [`AgentToolUse::id`] this result answers.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_call_id: String,

    /// For `assistant` messages replayed from history: the tool calls the
    /// model made on that turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_use: Vec<AgentToolUse>,

    /// For `tool` messages: whether the tool failed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

impl AgentMessage {
    /// A `user` message.
    pub fn user(content: impl Into<String>) -> Self {
        AgentMessage {
            role: "user".into(),
            content: content.into(),
            ..Default::default()
        }
    }

    /// A `system` message. The gateway appends it to the system prompt.
    pub fn system(content: impl Into<String>) -> Self {
        AgentMessage {
            role: "system".into(),
            content: content.into(),
            ..Default::default()
        }
    }

    /// An `assistant` message replaying a previous turn: its text and the
    /// tool calls it made.
    pub fn assistant(content: impl Into<String>, tool_use: Vec<AgentToolUse>) -> Self {
        AgentMessage {
            role: "assistant".into(),
            content: content.into(),
            tool_use,
            ..Default::default()
        }
    }

    /// A `tool` message carrying the result of one tool call. Consecutive
    /// tool results are grouped onto one provider turn by the gateway.
    pub fn tool_result(
        tool_call_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        AgentMessage {
            role: "tool".into(),
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            is_error,
            ..Default::default()
        }
    }
}

/// A tool the model may call. The gateway forwards the definition to the
/// provider; execution is the client's job.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentToolDef {
    /// Tool name. Also the key the `capabilities` allowlist matches on.
    pub name: String,

    /// What the tool does.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// JSON Schema for the tool's input, as an object. Omitted when null.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub input_schema: serde_json::Value,
}

/// Request body for `POST /qai/v1/agent`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AgentRequest {
    /// Model to run the turn on. Required; unknown models are refused with
    /// 400 `unknown_model`.
    pub model: String,

    /// The conversation so far. Required and non-empty; at most 1000
    /// messages and 2 MB of content in total.
    pub messages: Vec<AgentMessage>,

    /// Tools the model may call. At most 256.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<AgentToolDef>,

    /// Tool allowlist by name. Three-state: omitted forwards every tool,
    /// `Some(vec![])` forwards none (safe mode), and a non-empty list
    /// forwards only the named ones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,

    /// System prompt. `system` messages are appended to it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Maximum output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// One content block in an [`AgentResponse`].
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AgentContentPart {
    /// Block type — `"text"`.
    #[serde(rename = "type", default)]
    pub part_type: String,

    /// The text.
    #[serde(default)]
    pub text: String,
}

/// Token usage for one agent turn.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AgentUsage {
    /// Prompt tokens.
    #[serde(default)]
    pub input_tokens: i64,

    /// Completion tokens.
    #[serde(default)]
    pub output_tokens: i64,
}

/// Response from `POST /qai/v1/agent`: a single JSON document, never a
/// stream.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AgentResponse {
    /// The gateway request id.
    #[serde(default)]
    pub id: String,

    /// Model that ran the turn.
    #[serde(default)]
    pub model: String,

    /// Why the model stopped. `"tool_use"` whenever [`tool_use`] is
    /// non-empty, otherwise the provider's reason (`"end_turn"` when the
    /// provider gave none).
    ///
    /// [`tool_use`]: AgentResponse::tool_use
    #[serde(default)]
    pub stop_reason: String,

    /// Text blocks. Empty when the model only called tools.
    #[serde(default, deserialize_with = "null_as_default")]
    pub content: Vec<AgentContentPart>,

    /// Tool calls to execute. Present only when the model called tools.
    #[serde(default, deserialize_with = "null_as_default")]
    pub tool_use: Vec<AgentToolUse>,

    /// Token usage for the turn.
    #[serde(default)]
    pub usage: AgentUsage,

    /// Cost of the turn in ticks, read from the `X-QAI-Cost-Ticks` response
    /// header. Zero when the gateway sent none.
    #[serde(default)]
    pub cost_ticks: i64,
}

impl AgentResponse {
    /// All text blocks joined.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter(|part| part.part_type == "text")
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("")
    }

    /// This turn as an `assistant` message, for replaying into the next
    /// request's history ahead of the tool results.
    pub fn to_message(&self) -> AgentMessage {
        AgentMessage::assistant(self.text(), self.tool_use.clone())
    }
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

    /// Worker to escalate to on failure (e.g. cheap coder → expensive coder).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalate_to: Option<String>,

    /// Max retries before escalating (default 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<i32>,
}

/// Request body for a mission run.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MissionRequest {
    /// The high-level goal for the mission.
    pub goal: String,

    /// Execution strategy hint: `"wave"` (default), `"dag"`, `"mapreduce"`,
    /// `"refinement"`, `"branch"`, or `"codegen"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,

    /// Model for the conductor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_model: Option<String>,

    /// Conductor tier override. Default: "expensive".
    /// Set to "cheap" when using a fast router as conductor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_tier: Option<String>,

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

    /// Context management configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_config: Option<ContextConfig>,

    /// Deployment whose endpoint worker inference is routed to. Honoured
    /// only when `strategy` is `"codegen"`; ignored otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,

    /// Build command to run after codegen (e.g. `"cargo build"`). Honoured
    /// only when `strategy` is `"codegen"`; ignored otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,

    /// Directory the codegen pipeline writes generated files to, on the
    /// gateway's own filesystem. Interpreted relative to the caller's
    /// per-user workspace root; an absolute path or any `..` segment is
    /// rejected with 400 `invalid_workspace_path`. Honoured only when
    /// `strategy` is `"codegen"`; ignored otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
}

/// Backwards-compatible alias for [`MissionWorker`].
pub type MissionWorkerConfig = MissionWorker;

// ---------------------------------------------------------------------------
// SSE Stream
// ---------------------------------------------------------------------------

/// A single event from a mission, Cloud Run, or inference SSE stream.
///
/// Two events are synthesised client-side: `done` when the `[DONE]` sentinel
/// arrives, and `error` when a `data:` payload does not parse or the
/// transport fails mid-stream. The latter carries `error` (the message) and
/// `transport: true` in [`data`](Self::data); the stream ends after it, so a
/// run that stops without a preceding `done` or result event was cut off.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentStreamEvent {
    /// Event type (e.g. "step", "thought", "tool_call", "tool_result", "message", "error", "done").
    #[serde(rename = "type", default)]
    pub event_type: String,

    /// Raw JSON payload for caller to interpret.
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

/// A single SSE event from an agent-style stream.
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

/// A line of an SSE body, or the transport error that cut the body short.
pub(crate) type SseLine = std::result::Result<String, reqwest::Error>;

/// Splits a byte stream into newline-delimited lines, trailing `\r`
/// stripped. A transport error is yielded as `Err` after any buffered
/// partial line, and the stream ends after it.
fn sse_lines<S>(byte_stream: S) -> impl Stream<Item = SseLine> + Send
where
    S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
{
    let pinned_stream = Box::pin(byte_stream);

    futures_util::stream::unfold(
        (pinned_stream, String::new(), None::<reqwest::Error>, false),
        |(mut stream, mut buffer, mut pending_err, mut finished)| async move {
            use futures_util::StreamExt;
            loop {
                if let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
                    buffer = buffer[newline_pos + 1..].to_string();
                    return Some((Ok(line), (stream, buffer, pending_err, finished)));
                }
                if finished {
                    if !buffer.is_empty() {
                        let remaining = std::mem::take(&mut buffer);
                        return Some((Ok(remaining), (stream, buffer, pending_err, finished)));
                    }
                    if let Some(err) = pending_err.take() {
                        return Some((Err(err), (stream, buffer, pending_err, finished)));
                    }
                    return None;
                }

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                    }
                    Some(Err(err)) => {
                        pending_err = Some(err);
                        finished = true;
                    }
                    None => {
                        finished = true;
                    }
                }
            }
        },
    )
}

/// Yields the payload of each SSE `data:` line, dropping the `[DONE]`
/// sentinel and every non-data line; a transport failure comes through as
/// `Err`. Shared by the surfaces whose SSE events are typed rather than the
/// loose [`AgentStreamEvent`] shape.
pub(crate) fn sse_data_payloads<S>(byte_stream: S) -> impl Stream<Item = SseLine> + Send
where
    S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
{
    let pinned_lines = Box::pin(sse_lines(byte_stream));
    futures_util::stream::unfold(pinned_lines, |mut lines| async move {
        use futures_util::StreamExt;
        loop {
            let line = match lines.next().await? {
                Ok(line) => line,
                Err(err) => return Some((Err(err), lines)),
            };
            if let Some(payload) = line.strip_prefix("data: ") {
                if payload == "[DONE]" {
                    continue;
                }
                return Some((Ok(payload.to_string()), lines));
            }
        }
    })
}

fn error_event(message: String, transport: bool) -> AgentStreamEvent {
    let mut data = HashMap::new();
    data.insert("error".to_string(), serde_json::Value::String(message));
    if transport {
        data.insert("transport".to_string(), serde_json::Value::Bool(true));
    }
    AgentStreamEvent {
        event_type: "error".to_string(),
        data,
    }
}

/// Converts a byte stream into a stream of parsed [`AgentStreamEvent`]s.
fn sse_to_agent_events<S>(byte_stream: S) -> impl Stream<Item = AgentStreamEvent> + Send
where
    S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
{
    let pinned_lines = Box::pin(sse_lines(byte_stream));
    futures_util::stream::unfold(pinned_lines, |mut lines| async move {
        use futures_util::StreamExt;
        loop {
            let line = match lines.next().await? {
                Ok(line) => line,
                Err(err) => {
                    return Some((error_event(format!("transport: {err}"), true), lines));
                }
            };

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
                Err(e) => return Some((error_event(format!("parse SSE: {e}"), false), lines)),
            }
        }
    })
}

impl AgentStream {
    /// Wraps an SSE response body as a stream of [`AgentStreamEvent`]s.
    pub(crate) fn from_response(resp: reqwest::Response) -> Self {
        AgentStream {
            inner: Box::pin(sse_to_agent_events(resp.bytes_stream())),
        }
    }
}

impl Client {
    /// Runs one model turn with tool-call passthrough.
    ///
    /// The gateway calls the provider once, non-streaming, and returns the
    /// text and any tool calls. It executes nothing: when
    /// [`AgentResponse::stop_reason`] is `"tool_use"`, run each
    /// [`AgentResponse::tool_use`] locally, then call again with the history
    /// extended by [`AgentResponse::to_message`] and one
    /// [`AgentMessage::tool_result`] per call. The turn is billed at the
    /// model's chat rate.
    ///
    /// `POST /qai/v1/agent`
    pub async fn agent_step(&self, req: &AgentRequest) -> Result<AgentResponse> {
        let (mut resp, meta) = self
            .post_json::<AgentRequest, AgentResponse>("/qai/v1/agent", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.id.is_empty() {
            resp.id = meta.request_id;
        }
        Ok(resp)
    }

    /// Starts a mission run and returns an SSE event stream.
    ///
    /// Missions are higher-level than a single agent turn: the conductor
    /// plans, assigns named workers, and manages context across steps, all
    /// server-side.
    ///
    /// `POST /qai/v1/missions`
    pub async fn mission_run(&self, req: &MissionRequest) -> Result<AgentStream> {
        let (resp, _meta) = self.post_stream_raw("/qai/v1/missions", req).await?;
        Ok(AgentStream::from_response(resp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[test]
    fn agent_request_serialises_the_keys_the_handler_reads() {
        let req = AgentRequest {
            model: "claude-sonnet-4-6".into(),
            messages: vec![
                AgentMessage::user("list the files"),
                AgentMessage::assistant(
                    "",
                    vec![AgentToolUse {
                        id: "call_1".into(),
                        name: "ls".into(),
                        input: serde_json::json!({"path": "."}),
                    }],
                ),
                AgentMessage::tool_result("call_1", "a.rs\nb.rs", false),
            ],
            tools: vec![AgentToolDef {
                name: "ls".into(),
                description: "list a directory".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }],
            capabilities: Some(vec!["ls".into()]),
            system_prompt: Some("be brief".into()),
            max_tokens: Some(256),
            temperature: None,
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["model"], "claude-sonnet-4-6");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "list the files");
        assert!(json["messages"][0].get("tool_use").is_none());
        assert_eq!(json["messages"][1]["role"], "assistant");
        assert_eq!(json["messages"][1]["tool_use"][0]["id"], "call_1");
        assert_eq!(json["messages"][1]["tool_use"][0]["input"]["path"], ".");
        assert_eq!(json["messages"][2]["role"], "tool");
        assert_eq!(json["messages"][2]["tool_call_id"], "call_1");
        assert!(json["messages"][2].get("is_error").is_none());
        assert_eq!(json["tools"][0]["name"], "ls");
        assert_eq!(json["tools"][0]["input_schema"]["type"], "object");
        assert_eq!(json["capabilities"], serde_json::json!(["ls"]));
        assert_eq!(json["system_prompt"], "be brief");
        assert_eq!(json["max_tokens"], 256);
        assert!(json.get("temperature").is_none());
        assert!(json.get("stream").is_none());
    }

    #[test]
    fn safe_mode_sends_an_empty_capability_list_and_no_tools_key() {
        let req = AgentRequest {
            model: "m".into(),
            messages: vec![AgentMessage::user("hi")],
            capabilities: Some(Vec::new()),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["capabilities"], serde_json::json!([]));
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn agent_response_decodes_a_tool_use_turn() {
        let resp: AgentResponse = serde_json::from_str(
            r#"{"id":"req_1","model":"claude-sonnet-4-6","stop_reason":"tool_use",
                "content":[],
                "tool_use":[{"id":"toolu_1","name":"ls","input":{"path":"src"}}],
                "usage":{"input_tokens":12,"output_tokens":7}}"#,
        )
        .expect("decode");
        assert_eq!(resp.stop_reason, "tool_use");
        assert_eq!(resp.tool_use[0].input["path"], "src");
        assert_eq!(resp.usage.output_tokens, 7);
        assert_eq!(resp.text(), "");
        let replay = resp.to_message();
        assert_eq!(replay.role, "assistant");
        assert_eq!(replay.tool_use[0].id, "toolu_1");
    }

    #[test]
    fn agent_response_decodes_a_text_turn_without_tool_use() {
        let resp: AgentResponse = serde_json::from_str(
            r#"{"id":"req_2","model":"m","stop_reason":"end_turn",
                "content":[{"type":"text","text":"hello "},{"type":"text","text":"there"}],
                "usage":{"input_tokens":1,"output_tokens":2}}"#,
        )
        .expect("decode");
        assert!(resp.tool_use.is_empty());
        assert_eq!(resp.text(), "hello there");
    }

    #[test]
    fn mission_request_carries_only_gateway_fields() {
        let req = MissionRequest {
            goal: "ship it".into(),
            strategy: Some("codegen".into()),
            workspace_path: Some("proj".into()),
            build_command: Some("cargo build".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["workspace_path"], "proj");
        assert_eq!(json["build_command"], "cargo build");
        assert!(json.get("auto_plan").is_none());
        assert!(json.get("worker_model").is_none());
    }

    fn chunks(
        items: Vec<std::result::Result<&'static str, ()>>,
    ) -> impl Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + 'static
    {
        futures_util::stream::iter(items).then(|item| async move {
            match item {
                Ok(s) => Ok(bytes::Bytes::from_static(s.as_bytes())),
                // The only way to mint a reqwest::Error outside reqwest is to
                // trigger one; a bad URL does it without touching the network.
                Err(()) => Err(reqwest::Client::new()
                    .get("http://[::1")
                    .build()
                    .expect_err("invalid url")),
            }
        })
    }

    #[tokio::test]
    async fn transport_failure_surfaces_as_an_error_event_then_ends() {
        let events: Vec<AgentStreamEvent> = sse_to_agent_events(chunks(vec![
            Ok("data: {\"type\":\"agent_step\",\"step\":1}\n\n"),
            Err(()),
        ]))
        .collect()
        .await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "agent_step");
        assert_eq!(events[1].event_type, "error");
        assert_eq!(events[1].data["transport"], serde_json::Value::Bool(true));
        assert!(
            events[1].data["error"]
                .as_str()
                .unwrap_or_default()
                .starts_with("transport: ")
        );
    }

    #[tokio::test]
    async fn clean_finish_yields_done_and_no_error() {
        let events: Vec<AgentStreamEvent> = sse_to_agent_events(chunks(vec![
            Ok("data: {\"type\":\"agent_result\"}\n"),
            Ok("data: [DONE]\n"),
        ]))
        .collect()
        .await;
        let types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(types, vec!["agent_result", "done"]);
    }

    #[tokio::test]
    async fn data_payloads_pass_transport_errors_through() {
        let items: Vec<SseLine> = sse_data_payloads(chunks(vec![
            Ok("event: ping\ndata: {\"a\":1}\ndata: [DONE]\n"),
            Err(()),
        ]))
        .collect()
        .await;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_ref().expect("payload"), "{\"a\":1}");
        assert!(items[1].is_err());
    }
}
