use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::chat::{ChatResponse, ChatStream, ChatTool};
use crate::client::Client;
use crate::error::Result;

/// Configuration for session context management.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextConfig {
    /// Token threshold that triggers automatic compaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compact_at_tokens: Option<i64>,

    /// Number of recent tool call/result pairs to keep uncompacted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_recent_tool_results: Option<i32>,

    /// Strip thinking blocks from older assistant turns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_thinking: Option<bool>,

    /// Summarization strategy. `"plan_and_tools"` is the only strategy
    /// the gateway distinguishes (it keeps the plan and tool history in
    /// the summary); any other value, unset included, gets the default
    /// summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summarize_strategy: Option<String>,

    /// Model to use for summarization (default: gemini-2.5-flash).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summarize_model: Option<String>,
}

/// A tool result to feed back into the session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolResult {
    /// The tool_use ID this result corresponds to.
    pub tool_call_id: String,

    /// The result content.
    pub content: String,

    /// Whether this result is an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// A tool execution result from the client. Same shape as [`ToolResult`],
/// which is the type [`SessionChatRequest::tool_results`] carries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionToolResult {
    /// References the tool_use ID from the previous response.
    pub tool_call_id: String,

    /// The tool execution result content.
    pub content: String,

    /// Whether the tool execution failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Context metadata with a stale-tool-result count. [`SessionChatResponse`]
/// carries [`SessionContext`], which lacks that field.
#[derive(Debug, Clone, Deserialize)]
pub struct ContextMetadata {
    /// Total number of turns in the conversation.
    #[serde(default)]
    pub turn_count: i64,

    /// Estimated token count of the current context.
    #[serde(default)]
    pub estimated_tokens: i64,

    /// Whether the conversation was compacted in this request.
    #[serde(default)]
    pub compacted: bool,

    /// Description of the compaction that occurred.
    #[serde(default)]
    pub compaction_note: Option<String>,

    /// Number of stale tool results that were cleared.
    #[serde(default)]
    pub tools_cleared: Option<i32>,
}

/// Request body for session-based chat.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SessionChatRequest {
    /// Session identifier. Omit to create a new session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Model to use for generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// The user message.
    pub message: String,

    /// Tools the model can call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatTool>>,

    /// Results from previous tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<ToolResult>>,

    /// Streaming flag on the wire. The method sets it: [`Client::chat_session`]
    /// sends `false` and [`Client::chat_session_stream`] sends `true`,
    /// overwriting whatever is here, so the buffered call never receives
    /// an SSE body it cannot decode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// System prompt for the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Context management configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_config: Option<ContextConfig>,

    /// How much chain-of-thought a reasoning model runs before answering.
    /// One of "none", "low", "medium", "high", "xhigh"; `None` = provider
    /// default. Mirrors [`ChatRequest::reasoning_effort`](crate::ChatRequest).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    /// Provider-specific settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<HashMap<String, serde_json::Value>>,
}

/// Context metadata returned with session responses.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionContext {
    /// Number of conversation turns in the session.
    pub turn_count: i64,

    /// Estimated total tokens in the session context.
    pub estimated_tokens: i64,

    /// Whether context was compacted during this turn.
    #[serde(default)]
    pub compacted: bool,

    /// Note about the compaction, if any.
    #[serde(default)]
    pub compaction_note: Option<String>,
}

/// Response from session-based chat.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionChatResponse {
    /// The session identifier (may be newly created).
    pub session_id: String,

    /// The chat response.
    pub response: ChatResponse,

    /// Session context metadata.
    pub context: SessionContext,
}

/// A streaming session turn: the session it belongs to, and the events.
///
/// The gateway sends the session id in the `X-QAI-Session-Id` header and
/// again as the first event (`type: "session"`, see
/// [`StreamEvent::session`](crate::StreamEvent::session)); then
/// `content_delta` / `thinking_delta`, a `usage` event and `done`. Tool
/// calls are not streamed on this route.
pub struct SessionChatStream {
    /// The session identifier (newly created when the request had none).
    pub session_id: String,
    /// The events, in the same shape as [`Client::chat_stream`].
    pub events: ChatStream,
}

impl Client {
    /// Sends a message within a persistent session and waits for the
    /// whole answer.
    ///
    /// Sessions maintain conversation history server-side with automatic
    /// context compaction. Omit `session_id` to start a new session. The
    /// request goes out with `stream: false` whatever the struct says;
    /// use [`chat_session_stream`](Self::chat_session_stream) to stream.
    pub async fn chat_session(&self, req: &SessionChatRequest) -> Result<SessionChatResponse> {
        let req = with_stream(req, false);
        let (resp, _meta) = self
            .post_json::<SessionChatRequest, SessionChatResponse>("/qai/v1/chat/session", &req)
            .await?;
        Ok(resp)
    }

    /// Sends a message within a persistent session and streams the
    /// answer. Same route and semantics as
    /// [`chat_session`](Self::chat_session) with `stream: true`.
    pub async fn chat_session_stream(&self, req: &SessionChatRequest) -> Result<SessionChatStream> {
        let req = with_stream(req, true);
        let (resp, _meta) = self.post_stream_raw("/qai/v1/chat/session", &req).await?;
        let session_id = resp
            .headers()
            .get("X-QAI-Session-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        Ok(SessionChatStream {
            session_id,
            events: ChatStream::from_response(resp),
        })
    }
}

/// The request as the method sends it: the streaming flag belongs to the
/// method, not the caller, so a buffered call can never be answered with
/// an SSE body it cannot decode.
fn with_stream(req: &SessionChatRequest, stream: bool) -> SessionChatRequest {
    let mut req = req.clone();
    req.stream = Some(stream);
    req
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_method_owns_the_stream_flag() {
        let req = SessionChatRequest {
            message: "hi".into(),
            stream: Some(true),
            ..Default::default()
        };
        let buffered = serde_json::to_value(with_stream(&req, false)).unwrap();
        assert_eq!(buffered["stream"], serde_json::Value::Bool(false));
        let streamed = serde_json::to_value(with_stream(&req, true)).unwrap();
        assert_eq!(streamed["stream"], serde_json::Value::Bool(true));
    }
}
