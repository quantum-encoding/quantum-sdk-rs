use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::chat::{ChatResponse, ChatTool};
use crate::client::Client;
use crate::error::Result;

/// Configuration for session context management.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextConfig {
    /// Maximum number of tokens to retain before compaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,

    /// Whether to enable automatic context compaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact: Option<bool>,
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

/// A tool execution result from the client (used in SessionChatRequest.tool_results).
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

/// Context metadata returned in session responses (includes tools_cleared).
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

    /// Enable streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// System prompt for the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Context management configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_config: Option<ContextConfig>,

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

impl Client {
    /// Sends a message within a persistent session.
    ///
    /// Sessions maintain conversation history server-side with automatic
    /// context compaction. Omit `session_id` to start a new session.
    pub async fn chat_session(&self, req: &SessionChatRequest) -> Result<SessionChatResponse> {
        let (resp, _meta) = self
            .post_json::<SessionChatRequest, SessionChatResponse>("/qai/v1/chat/session", req)
            .await?;
        Ok(resp)
    }
}
