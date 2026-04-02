use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Deserialize null as empty Vec (Gemini sometimes returns null for array fields).
fn null_as_empty_vec<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer).map(|v| v.unwrap_or_default())
}

/// Deserialize null as None for Option<Vec<T>> fields.
fn deserialize_opt_vec<'de, D, T>(deserializer: D) -> std::result::Result<Option<Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    // null → None, [] → Some([]), [...] → Some([...])
    Ok(Option::<Vec<T>>::deserialize(deserializer).unwrap_or(None))
}

/// Request body for text generation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatRequest {
    /// Model ID that determines provider routing (e.g. "claude-sonnet-4-6", "grok-4-1-fast-non-reasoning").
    pub model: String,

    /// Conversation history.
    pub messages: Vec<ChatMessage>,

    /// Functions the model can call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatTool>>,

    /// Enables server-sent event streaming. Set automatically by `chat_stream`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Controls randomness (0.0-2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Limits the response length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Provider-specific settings (e.g. Anthropic thinking, xAI search).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<HashMap<String, serde_json::Value>>,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatMessage {
    /// One of "system", "user", "assistant", or "tool".
    pub role: String,

    /// Text content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Structured content for assistant messages with tool calls.
    /// When present, takes precedence over `content`.
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_opt_vec", default)]
    pub content_blocks: Option<Vec<ContentBlock>>,

    /// Required when role is "tool" — references the tool_use ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Whether a tool result is an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ChatMessage {
    /// Creates a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            ..Default::default()
        }
    }

    /// Creates an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.into()),
            ..Default::default()
        }
    }

    /// Creates a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            ..Default::default()
        }
    }

    /// Creates a tool result message.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            ..Default::default()
        }
    }

    /// Creates a tool error result message.
    pub fn tool_error(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            is_error: Some(true),
            ..Default::default()
        }
    }
}

/// A single block in the response content array.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentBlock {
    /// One of "text", "thinking", or "tool_use".
    #[serde(rename = "type")]
    pub block_type: String,

    /// Content for "text" and "thinking" blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Tool call identifier for "tool_use" blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Function name for "tool_use" blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Function arguments for "tool_use" blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<HashMap<String, serde_json::Value>>,

    /// Gemini thought signature — must be echoed back with tool results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,

    /// Base64-encoded data for file/image content blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,

    /// Filename for file content blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    /// MIME type for file/image content blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Defines a function the model can call.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatTool {
    /// Function name.
    pub name: String,

    /// Explains what the function does.
    pub description: String,

    /// JSON Schema for the function's arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Response from a non-streaming chat request.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    /// Unique request identifier.
    pub id: String,

    /// Model that generated the response.
    pub model: String,

    /// List of content blocks (text, thinking, tool_use).
    #[serde(default, deserialize_with = "null_as_empty_vec")]
    pub content: Vec<ContentBlock>,

    /// Token counts and cost.
    pub usage: Option<ChatUsage>,

    /// Why generation stopped ("end_turn", "tool_use", "max_tokens").
    #[serde(default)]
    pub stop_reason: String,

    /// Citations from web search (when search is enabled via provider_options).
    #[serde(default, deserialize_with = "null_as_empty_vec")]
    pub citations: Vec<Citation>,

    /// Total cost from the X-QAI-Cost-Ticks header.
    #[serde(skip)]
    pub cost_ticks: i64,

    /// From the X-QAI-Request-Id header.
    #[serde(skip)]
    pub request_id: String,
}

impl ChatResponse {
    /// Returns the concatenated text content, ignoring thinking and tool_use blocks.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter(|b| b.block_type == "text")
            .filter_map(|b| b.text.as_deref())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Returns the concatenated thinking content.
    pub fn thinking(&self) -> String {
        self.content
            .iter()
            .filter(|b| b.block_type == "thinking")
            .filter_map(|b| b.text.as_deref())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Returns all tool_use blocks from the response.
    pub fn tool_calls(&self) -> Vec<&ContentBlock> {
        self.content
            .iter()
            .filter(|b| b.block_type == "tool_use")
            .collect()
    }
}

/// A source reference from web search grounding.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Citation {
    /// Title of the cited source.
    #[serde(default)]
    pub title: String,

    /// URL of the cited source.
    #[serde(default)]
    pub url: String,

    /// Relevant text snippet from the source.
    #[serde(default)]
    pub text: String,

    /// Position in the response.
    #[serde(default)]
    pub index: i32,
}

/// Token counts and cost for a chat response.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cost_ticks: i64,
}

/// A single event from an SSE chat stream.
#[derive(Debug, Clone)]
pub struct StreamEvent {
    /// Event type: "content_delta", "thinking_delta", "tool_use", "usage", "heartbeat", "error", "done".
    pub event_type: String,

    /// Incremental text for content_delta and thinking_delta events.
    pub delta: Option<StreamDelta>,

    /// Populated for tool_use events.
    pub tool_use: Option<StreamToolUse>,

    /// Populated for usage events.
    pub usage: Option<ChatUsage>,

    /// Populated for error events.
    pub error: Option<String>,

    /// True when the stream is complete.
    pub done: bool,
}

/// Incremental text in a streaming event.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamDelta {
    pub text: String,
}

/// A tool call from a streaming event.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamToolUse {
    pub id: String,
    pub name: String,
    pub input: HashMap<String, serde_json::Value>,
}

/// Raw JSON from the SSE stream before parsing into typed fields.
#[derive(Deserialize)]
struct RawStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<StreamDelta>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    input_tokens: Option<i32>,
    #[serde(default)]
    output_tokens: Option<i32>,
    #[serde(default)]
    cost_ticks: Option<i64>,
    #[serde(default)]
    message: Option<String>,
}

pin_project! {
    /// An async stream of [`StreamEvent`]s from an SSE chat response.
    pub struct ChatStream {
        #[pin]
        inner: Pin<Box<dyn Stream<Item = StreamEvent> + Send>>,
    }
}

impl Stream for ChatStream {
    type Item = StreamEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

impl Client {
    /// Sends a non-streaming text generation request.
    pub async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse> {
        let mut req = req.clone();
        req.stream = Some(false);

        let (mut resp, meta) = self.post_json::<ChatRequest, ChatResponse>("/qai/v1/chat", &req).await?;
        resp.cost_ticks = meta.cost_ticks;
        resp.request_id = meta.request_id;
        if resp.model.is_empty() {
            resp.model = meta.model;
        }
        Ok(resp)
    }

    /// Sends a streaming text generation request and returns an async stream of events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use futures_util::StreamExt;
    ///
    /// # async fn example() -> quantum_sdk::Result<()> {
    /// let client = quantum_sdk::Client::new("key");
    /// let req = quantum_sdk::ChatRequest {
    ///     model: "claude-sonnet-4-6".into(),
    ///     messages: vec![quantum_sdk::ChatMessage::user("Hello!")],
    ///     ..Default::default()
    /// };
    /// let mut stream = client.chat_stream(&req).await?;
    /// while let Some(ev) = stream.next().await {
    ///     if let Some(delta) = &ev.delta {
    ///         print!("{}", delta.text);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chat_stream(&self, req: &ChatRequest) -> Result<ChatStream> {
        let mut req = req.clone();
        req.stream = Some(true);

        let (resp, _meta) = self.post_stream_raw("/qai/v1/chat", &req).await?;

        let byte_stream = resp.bytes_stream();
        let event_stream = sse_to_events(byte_stream);

        Ok(ChatStream {
            inner: Box::pin(event_stream),
        })
    }
}

/// Converts a byte stream into a stream of parsed [`StreamEvent`]s.
fn sse_to_events<S>(byte_stream: S) -> impl Stream<Item = StreamEvent> + Send
where
    S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
{
    // Pin the byte stream so we can poll it inside unfold.
    let pinned_stream = Box::pin(byte_stream);

    // Accumulate raw bytes into lines to avoid splitting multi-byte UTF-8 characters.
    // Only convert to String when we have a complete newline-terminated line.
    let line_stream = futures_util::stream::unfold(
        (pinned_stream, Vec::<u8>::new()),
        |(mut stream, mut buffer)| async move {
            use futures_util::StreamExt;
            loop {
                // Check if we have a complete line in the buffer.
                if let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    let mut line_bytes = buffer[..newline_pos].to_vec();
                    buffer = buffer[newline_pos + 1..].to_vec();
                    // Trim trailing \r
                    if line_bytes.last() == Some(&b'\r') {
                        line_bytes.pop();
                    }
                    let line = String::from_utf8_lossy(&line_bytes).into_owned();
                    return Some((line, (stream, buffer)));
                }

                // Read more data.
                match stream.next().await {
                    Some(Ok(chunk)) => {
                        buffer.extend_from_slice(&chunk);
                    }
                    Some(Err(_)) | None => {
                        // Stream ended. Emit remaining buffer if non-empty.
                        if !buffer.is_empty() {
                            let remaining = String::from_utf8_lossy(&buffer).into_owned();
                            buffer.clear();
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
                let ev = StreamEvent {
                    event_type: "done".to_string(),
                    delta: None,
                    tool_use: None,
                    usage: None,
                    error: None,
                    done: true,
                };
                return Some((ev, lines));
            }

            let raw: RawStreamEvent = match serde_json::from_str(payload) {
                Ok(r) => r,
                Err(e) => {
                    let ev = StreamEvent {
                        event_type: "error".to_string(),
                        delta: None,
                        tool_use: None,
                        usage: None,
                        error: Some(format!("parse SSE: {e}")),
                        done: false,
                    };
                    return Some((ev, lines));
                }
            };

            let mut ev = StreamEvent {
                event_type: raw.event_type.clone(),
                delta: None,
                tool_use: None,
                usage: None,
                error: None,
                done: false,
            };

            match raw.event_type.as_str() {
                "content_delta" | "thinking_delta" => {
                    ev.delta = raw.delta;
                }
                "tool_use" => {
                    ev.tool_use = Some(StreamToolUse {
                        id: raw.id.unwrap_or_default(),
                        name: raw.name.unwrap_or_default(),
                        input: raw.input.unwrap_or_default(),
                    });
                }
                "usage" => {
                    ev.usage = Some(ChatUsage {
                        input_tokens: raw.input_tokens.unwrap_or(0),
                        output_tokens: raw.output_tokens.unwrap_or(0),
                        cost_ticks: raw.cost_ticks.unwrap_or(0),
                    });
                }
                "error" => {
                    ev.error = raw.message;
                }
                "heartbeat" => {}
                _ => {}
            }

            return Some((ev, lines));
        }
    })
}
