use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

use crate::serde_util::null_as_default as null_as_empty_vec;

/// Deserialize an `Option<Vec<T>>` field the gateway may send as `null`
/// (a Go nil slice): null → None, [] → Some([]), [...] → Some([...]).
/// A malformed array is an error, not `None`.
fn deserialize_opt_vec<'de, D, T>(deserializer: D) -> std::result::Result<Option<Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer)
}

/// Request body for text generation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatRequest {
    /// Model ID that determines provider routing (e.g. "claude-sonnet-4-6",
    /// "grok-4-1-fast-non-reasoning", "qwen3.8-max"). See `Client::list_models`.
    pub model: String,

    /// Conversation history.
    pub messages: Vec<ChatMessage>,

    /// Functions the model can call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatTool>>,

    /// Constrains tool use: "auto" (default), "any" (force tool use), "none", or a specific tool name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,

    /// JSON Schema for structured output constraints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,

    /// Enables server-sent event streaming. Set automatically by `chat_stream`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Controls randomness (0.0-2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Limits the response length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// How much chain-of-thought a reasoning model runs before answering.
    /// One of "none", "low", "medium", "high", "xhigh", "max"; `None` =
    /// provider default (medium on GPT-5.5+). `max` is Anthropic Opus 4.7+
    /// only (OpenAI will 400 on it). On hybrid-thinking Qwen models
    /// (qwen3.8-max, qwen3.7-plus, qwen3.6-flash, qwen3-coder-*) any value
    /// but "none" enables thinking and "none" disables it. An unknown value
    /// is rejected with 400.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    /// Vertex resource name of a previously created context cache (e.g.
    /// "cachedContents/abc123"). When set, the cached content is billed at
    /// the cached-read rate and need not be re-sent. Gemini-only; the
    /// cache's model must match this request's model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,

    /// Provider-specific settings (e.g. Anthropic thinking, xAI search).
    /// The routing-region override (`provider_options.region`) rides here
    /// too — prefer the typed [`ChatRequest::region`] for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<HashMap<String, serde_json::Value>>,
}

impl ChatRequest {
    /// Overrides the routing region for this one request — rides
    /// `provider_options.region` on the wire and wins over the key's scope
    /// region. Honored by `/qai/v1/chat` only: the agent endpoint carries no
    /// provider_options and routes by the key's scope.
    pub fn region(mut self, region: crate::region::Region) -> Self {
        let opts = self.provider_options.get_or_insert_with(HashMap::new);
        opts.insert(
            "region".to_string(),
            serde_json::Value::String(region.as_str().to_string()),
        );
        self
    }
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
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_opt_vec",
        default
    )]
    pub content_blocks: Option<Vec<ContentBlock>>,

    /// Required when role is "tool" — references the tool_use ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Whether a tool result is an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,

    /// Provider-side reasoning state (OpenAI Responses API). Pass back the
    /// `phase` received on the previous turn's [`ChatResponse`] so reasoning
    /// state is preserved across replay. `None` for providers without phase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
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

    /// MIME type for file/image/file_uri content blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Remote-resource URL for `file_uri` content blocks. Gemini
    /// accepts YouTube URLs verbatim here (with `mime_type: "video/mp4"`)
    /// — no upload step needed for public videos. Other providers
    /// may require a pre-uploaded resource URI; unsupported URIs are
    /// silently skipped server-side rather than erroring the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_uri: Option<String>,
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

    /// Enable guaranteed schema validation on tool inputs (Anthropic, OpenAI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
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

    /// Why generation stopped: a canonical value from the [`stop_reason`]
    /// module, the same space regardless of which provider served the
    /// request. A provider-specific reason with no canonical mapping passes
    /// through lowercased, so match the known constants and treat anything
    /// else as terminal. A `String` rather than an enum so an unrecognized
    /// value never fails to deserialize.
    #[serde(default)]
    pub stop_reason: String,

    /// Citations from web search (when search is enabled via provider_options).
    #[serde(default, deserialize_with = "null_as_empty_vec")]
    pub citations: Vec<Citation>,

    /// Provider-side reasoning-state tag (OpenAI Responses API). Echo it back
    /// on the corresponding assistant [`ChatMessage::phase`] of the next turn
    /// to preserve reasoning state across replay. Empty when the provider
    /// doesn't surface phase.
    #[serde(default)]
    pub phase: String,

    /// `Some(true)` when this response was served from the semantic cache
    /// (the same signal the `X-QAI-Cache: hit-tier-N` header carries at
    /// the transport layer). `None`/`Some(false)` on a fresh provider
    /// response. A hit is served before any credit reservation: nothing
    /// is charged or metered, `usage.cost_ticks` is 0, no
    /// `X-QAI-Cost-Ticks` header is sent, and [`cost_ticks`](Self::cost_ticks)
    /// is 0.
    #[serde(default)]
    pub cached: Option<bool>,

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

    /// True when the model is requesting tool execution
    /// (`stop_reason == "tool_use"`). Every provider is normalised the same
    /// way: a natural stop with tool_use blocks present becomes `tool_use`.
    /// A provider that reports `max_tokens`, `content_filter` or `error`
    /// alongside tool calls keeps that reason, so check
    /// [`tool_calls`](Self::tool_calls) too if you must act on a partial
    /// tool request.
    pub fn is_tool_use(&self) -> bool {
        self.stop_reason == stop_reason::TOOL_USE
    }

    /// True when a safety classifier declined the request
    /// (`stop_reason == "refusal"`). On a refusal the content may be empty
    /// or a partial, already-streamed prefix that should be discarded —
    /// check this before reading [`text`](Self::text). A refusal arrives as
    /// an HTTP 200, so it is never surfaced as an error.
    pub fn is_refusal(&self) -> bool {
        self.stop_reason == stop_reason::REFUSAL
    }

    /// True when output was cut off by the token cap
    /// (`stop_reason == "max_tokens"`) — the response is incomplete; raise
    /// `max_tokens` or continue the turn.
    pub fn is_max_tokens(&self) -> bool {
        self.stop_reason == stop_reason::MAX_TOKENS
    }
}

/// Canonical `stop_reason` values emitted by the gateway.
///
/// Every provider's native finish reason is normalized into this
/// Anthropic-flavored space before it reaches you, so matching these
/// constants works regardless of which model served the request. The
/// gateway may still pass through a provider-specific reason it cannot map
/// (lowercased); treat any value outside this set as terminal.
pub mod stop_reason {
    /// Natural completion.
    pub const END_TURN: &str = "end_turn";
    /// Model is requesting tool execution (tool_use blocks present).
    pub const TOOL_USE: &str = "tool_use";
    /// Output token cap reached — the response is truncated.
    pub const MAX_TOKENS: &str = "max_tokens";
    /// A requested stop sequence matched.
    pub const STOP_SEQUENCE: &str = "stop_sequence";
    /// Provider-side safety/policy stop.
    pub const CONTENT_FILTER: &str = "content_filter";
    /// A safety classifier declined the request; discard any partial output.
    pub const REFUSAL: &str = "refusal";
    /// Provider reported a terminal failure.
    pub const ERROR: &str = "error";
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
///
/// The two paths count output differently. On the non-streaming
/// envelope `output_tokens` is completion plus reasoning. On the
/// streaming `usage` event `output_tokens` is the visible completion
/// only and reasoning is reported beside it; `cost_ticks` covers both
/// either way, so the billed output on a stream is
/// `output_tokens + reasoning_tokens`.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatUsage {
    pub input_tokens: i32,
    /// Output tokens billed at the output rate. Includes reasoning on the
    /// non-streaming envelope; excludes it on the streaming usage event.
    pub output_tokens: i32,
    /// What the call cost, covering input, output and reasoning.
    pub cost_ticks: i64,

    /// Input tokens served from the provider's prompt cache, billed at the
    /// lower cached rate. Omitted on responses with no cache hit and on
    /// the streaming usage event.
    #[serde(default)]
    pub cached_tokens: Option<i64>,

    /// Reasoning / thinking tokens, billed at the output rate. Omitted on
    /// responses from non-reasoning models. Already inside
    /// `output_tokens` on the non-streaming envelope; on top of it on the
    /// streaming usage event.
    #[serde(default)]
    pub reasoning_tokens: Option<i64>,
}

/// Response shape from `POST /qai/v1/chat/estimate`. Returned by
/// `Client::estimate_chat`.
///
/// `estimated_cost_ticks` is the upfront reservation a `chat` call with the
/// same request would book: a worst-case ceiling the caller must have
/// available, not a prediction of the final settle. Text-only payloads
/// settle close to it; video and other multimodal inputs can over-estimate,
/// and the post-call settle refunds the difference.
#[derive(Debug, Clone, Deserialize)]
pub struct EstimateResponse {
    pub estimated_cost_ticks: i64,
    /// The same value converted to USD at the gateway's tick rate.
    pub estimated_cost_usd: f64,
    /// Model the estimate was computed against.
    #[serde(default)]
    pub model: String,
}

/// A single event from an SSE chat stream.
///
/// A tool call streams as a triplet: one `tool_use_start`, zero or more
/// `tool_use_input_delta`, then one `tool_use_complete` carrying the full
/// arguments. Some backends emit a single atomic `tool_use` event instead,
/// so a consumer handles both forms.
///
/// A stream that fails after the HTTP 200 is locked in reports the
/// failure as an event whose type is `error`, `invalid_request` (the
/// request was rejected: do not retry as-is) or `rate_limit` (the
/// provider throttled: retry later); all three carry the message in
/// [`error`](Self::error), and `done` follows.
#[derive(Debug, Clone)]
pub struct StreamEvent {
    /// Event type: "content_delta", "thinking_delta",
    /// "tool_use_start", "tool_use_input_delta", "tool_use_complete",
    /// "tool_use" (atomic), "citations", "session", "usage", "heartbeat",
    /// "error", "invalid_request", "rate_limit", "done".
    pub event_type: String,

    /// Incremental text for content_delta and thinking_delta events.
    pub delta: Option<StreamDelta>,

    /// Populated for atomic tool_use events.
    pub tool_use: Option<StreamToolUse>,

    /// Populated for tool_use_start events.
    pub tool_use_start: Option<StreamToolUseStart>,

    /// Populated for tool_use_input_delta events.
    pub tool_use_input_delta: Option<StreamToolUseInputDelta>,

    /// Populated for tool_use_complete events.
    pub tool_use_complete: Option<StreamToolUseComplete>,

    /// Populated for usage events.
    pub usage: Option<ChatUsage>,

    /// Web-search grounding sources, on a `citations` event. The gateway
    /// sends it once, before the first content delta, on streams where
    /// search results were injected; empty on every other event.
    pub citations: Vec<Citation>,

    /// Populated for the `session` event that opens a
    /// [`chat_session_stream`](Client::chat_session_stream).
    pub session: Option<StreamSession>,

    /// The failure message, on `error`, `invalid_request` and `rate_limit`
    /// events, and on an `error` the SDK raises for a payload it could
    /// not parse.
    pub error: Option<String>,

    /// True when the stream is complete.
    pub done: bool,
}

impl StreamEvent {
    fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            delta: None,
            tool_use: None,
            tool_use_start: None,
            tool_use_input_delta: None,
            tool_use_complete: None,
            usage: None,
            citations: Vec::new(),
            session: None,
            error: None,
            done: false,
        }
    }

    /// True when this event reports a failure, whichever of the three
    /// failure types the gateway used.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// The `session` event a session stream opens with.
#[derive(Debug, Clone)]
pub struct StreamSession {
    /// The session identifier (newly created when the request had none).
    pub session_id: String,
    /// Whether the history was compacted before this turn.
    pub compacted: bool,
}

/// Incremental text in a streaming event.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamDelta {
    pub text: String,
}

/// A tool call from an atomic `tool_use` streaming event.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamToolUse {
    pub id: String,
    pub name: String,
    pub input: HashMap<String, serde_json::Value>,
}

/// Tool-call start event — fires once before any input deltas.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamToolUseStart {
    pub id: String,
    pub name: String,
}

/// Tool-call input delta — fires zero or more times with raw JSON fragments.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamToolUseInputDelta {
    pub id: String,
    /// Raw JSON fragment. May not parse on its own; accumulate until
    /// the corresponding `tool_use_complete` event arrives with the
    /// authoritative `input`.
    pub partial_json: String,
}

/// Tool-call completion event — fires exactly once per call with the
/// server-accumulated, fully-parsed arguments.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamToolUseComplete {
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
    /// Carried by `tool_use_input_delta` events — a raw JSON fragment.
    #[serde(default)]
    partial_json: Option<String>,
    #[serde(default)]
    input_tokens: Option<i32>,
    #[serde(default)]
    output_tokens: Option<i32>,
    /// Portion of `output_tokens` spent on reasoning; carried by `usage`
    /// events, absent on non-reasoning models.
    #[serde(default)]
    reasoning_tokens: Option<i64>,
    #[serde(default)]
    cost_ticks: Option<i64>,
    #[serde(default)]
    message: Option<String>,
    /// Carried by the `citations` event.
    #[serde(default)]
    citations: Option<Vec<Citation>>,
    /// Carried by the `session` event that opens a session stream.
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    compacted: Option<bool>,
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

impl ChatStream {
    /// Wraps an SSE response body as a stream of parsed events.
    pub(crate) fn from_response(resp: reqwest::Response) -> Self {
        Self {
            inner: Box::pin(sse_to_events(resp.bytes_stream())),
        }
    }
}

impl Client {
    /// Applies the client-level routing region ([`crate::ClientBuilder::region`])
    /// to a chat request — unless the request already chose one
    /// ([`ChatRequest::region`] wins).
    fn apply_region(&self, req: &mut ChatRequest) {
        let Some(region) = self.region() else {
            return;
        };
        let already = req
            .provider_options
            .as_ref()
            .is_some_and(|o| o.contains_key("region"));
        if already {
            return;
        }
        let opts = req.provider_options.get_or_insert_with(HashMap::new);
        opts.insert(
            "region".to_string(),
            serde_json::Value::String(region.as_str().to_string()),
        );
    }

    /// Sends a non-streaming text generation request.
    pub async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse> {
        let mut req = req.clone();
        req.stream = Some(false);
        self.apply_region(&mut req);

        let (mut resp, meta) = self
            .post_json::<ChatRequest, ChatResponse>("/qai/v1/chat", &req)
            .await?;
        resp.cost_ticks = meta.cost_ticks;
        resp.request_id = meta.request_id;
        if resp.model.is_empty() {
            resp.model = meta.model;
        }
        Ok(resp)
    }

    /// Estimates the upfront credit reservation a `chat` call with the same
    /// `ChatRequest` would book, without calling the provider or deducting
    /// credits. Use it to show a cost hint before the user commits to an
    /// expensive payload such as a long video attached via
    /// `ContentBlock.file_uri`.
    ///
    /// Wraps `POST /qai/v1/chat/estimate`. Same auth as `chat()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> quantum_sdk::Result<()> {
    /// let client = quantum_sdk::Client::new("qai_...")?;
    /// let req = quantum_sdk::ChatRequest {
    ///     model: "gemini-flash-latest".into(),
    ///     messages: vec![quantum_sdk::ChatMessage::user("hi")],
    ///     ..Default::default()
    /// };
    /// let est = client.estimate_chat(&req).await?;
    /// println!("would cost ~{} ticks (~${})", est.estimated_cost_ticks, est.estimated_cost_usd);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn estimate_chat(&self, req: &ChatRequest) -> Result<EstimateResponse> {
        // Streaming does not change the cost ceiling, so `stream` stays off
        // the estimate payload.
        let mut req = req.clone();
        req.stream = None;
        self.apply_region(&mut req);
        let (resp, _meta) = self
            .post_json::<ChatRequest, EstimateResponse>("/qai/v1/chat/estimate", &req)
            .await?;
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
    /// let client = quantum_sdk::Client::new("key")?;
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
        self.apply_region(&mut req);

        let (resp, _meta) = self.post_stream_raw("/qai/v1/chat", &req).await?;
        Ok(ChatStream::from_response(resp))
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
                let mut ev = StreamEvent::new("done");
                ev.done = true;
                return Some((ev, lines));
            }

            let raw: RawStreamEvent = match serde_json::from_str(payload) {
                Ok(r) => r,
                Err(e) => {
                    let mut ev = StreamEvent::new("error");
                    ev.error = Some(format!("parse SSE: {e}"));
                    return Some((ev, lines));
                }
            };

            let mut ev = StreamEvent::new(raw.event_type.as_str());

            match raw.event_type.as_str() {
                "content_delta" | "thinking_delta" => {
                    ev.delta = raw.delta;
                }
                "tool_use" => {
                    // Atomic form, from backends that do not stream the triplet.
                    ev.tool_use = Some(StreamToolUse {
                        id: raw.id.unwrap_or_default(),
                        name: raw.name.unwrap_or_default(),
                        input: raw.input.unwrap_or_default(),
                    });
                }
                "tool_use_start" => {
                    ev.tool_use_start = Some(StreamToolUseStart {
                        id: raw.id.unwrap_or_default(),
                        name: raw.name.unwrap_or_default(),
                    });
                }
                "tool_use_input_delta" => {
                    ev.tool_use_input_delta = Some(StreamToolUseInputDelta {
                        id: raw.id.unwrap_or_default(),
                        partial_json: raw.partial_json.unwrap_or_default(),
                    });
                }
                "tool_use_complete" => {
                    ev.tool_use_complete = Some(StreamToolUseComplete {
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
                        // The streaming usage event carries reasoning_tokens
                        // but not cached_tokens; the cache split arrives only
                        // on the non-streaming envelope.
                        cached_tokens: None,
                        reasoning_tokens: raw.reasoning_tokens,
                    });
                }
                // The gateway classifies a failed stream as one of three
                // types; the message rides the same field on all of them.
                "error" | "invalid_request" | "rate_limit" => {
                    ev.error = Some(raw.message.unwrap_or_default());
                }
                "citations" => {
                    ev.citations = raw.citations.unwrap_or_default();
                }
                "session" => {
                    ev.session = Some(StreamSession {
                        session_id: raw.session_id.unwrap_or_default(),
                        compacted: raw.compacted.unwrap_or(false),
                    });
                }
                "heartbeat" => {}
                _ => {}
            }

            return Some((ev, lines));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client;
    use crate::region::Region;

    fn base_request() -> ChatRequest {
        ChatRequest {
            model: "qwen3.8-27b".into(),
            messages: vec![ChatMessage::user("hi")],
            ..Default::default()
        }
    }

    #[test]
    fn client_without_region_leaves_requests_alone() {
        let client = Client::new("qai_k_test").unwrap();
        let mut req = base_request();
        client.apply_region(&mut req);
        assert!(req.provider_options.is_none());
    }

    #[test]
    fn client_region_rides_provider_options() {
        let client = Client::builder("qai_k_test")
            .region(Region::Asia)
            .build()
            .unwrap();
        let mut req = base_request();
        client.apply_region(&mut req);
        let opts = req.provider_options.unwrap();
        assert_eq!(opts.get("region").and_then(|v| v.as_str()), Some("asia"));
    }

    #[test]
    fn client_region_preserves_other_provider_options() {
        let client = Client::builder("qai_k_test")
            .region(Region::Europe)
            .build()
            .unwrap();
        let mut req = base_request();
        req.provider_options = Some(HashMap::from([(
            "thinking".to_string(),
            serde_json::Value::Bool(true),
        )]));
        client.apply_region(&mut req);
        let opts = req.provider_options.unwrap();
        assert_eq!(opts.get("region").and_then(|v| v.as_str()), Some("europe"));
        assert_eq!(opts.get("thinking"), Some(&serde_json::Value::Bool(true)));
    }

    #[test]
    fn request_level_region_wins_over_client_level() {
        let client = Client::builder("qai_k_test")
            .region(Region::Europe)
            .build()
            .unwrap();
        let mut req = base_request().region(Region::Americas);
        client.apply_region(&mut req);
        let opts = req.provider_options.unwrap();
        assert_eq!(
            opts.get("region").and_then(|v| v.as_str()),
            Some("americas"),
            "the request-level choice must win"
        );
    }

    /// Runs the SSE parser over a canned body and collects the events.
    async fn parse_sse(body: &'static str) -> Vec<StreamEvent> {
        use futures_util::StreamExt;
        let chunks = futures_util::stream::iter(vec![Ok::<bytes::Bytes, reqwest::Error>(
            bytes::Bytes::from_static(body.as_bytes()),
        )]);
        sse_to_events(chunks).collect().await
    }

    #[tokio::test]
    async fn a_failed_stream_carries_its_message_whatever_the_type() {
        let events = parse_sse(concat!(
            "data: {\"type\":\"invalid_request\",\"message\":\"stream failed: bad model\"}\n\n",
            "data: {\"type\":\"rate_limit\",\"message\":\"stream failed: 429\"}\n\n",
            "data: {\"type\":\"error\",\"message\":\"request timeout\"}\n\n",
            "data: [DONE]\n\n",
        ))
        .await;
        assert_eq!(events.len(), 4);
        for (ev, expected) in events.iter().zip([
            ("invalid_request", "stream failed: bad model"),
            ("rate_limit", "stream failed: 429"),
            ("error", "request timeout"),
        ]) {
            assert_eq!(ev.event_type, expected.0);
            assert!(ev.is_error());
            assert_eq!(ev.error.as_deref(), Some(expected.1));
        }
        assert!(events[3].done);
    }

    #[tokio::test]
    async fn citations_and_session_events_are_parsed() {
        let events = parse_sse(concat!(
            "data: {\"type\":\"session\",\"session_id\":\"sess_1\",\"compacted\":true}\n\n",
            "data: {\"type\":\"citations\",\"citations\":[{\"title\":\"Rust\",\"url\":\"https://rust-lang.org\",\"text\":\"snippet\",\"index\":1}]}\n\n",
            ": ping\n\n",
            "data: {\"type\":\"content_delta\",\"delta\":{\"text\":\"hi\"}}\n\n",
            "data: {\"type\":\"usage\",\"input_tokens\":3,\"output_tokens\":1,\"reasoning_tokens\":7,\"cost_ticks\":42}\n\n",
            "data: [DONE]\n\n",
        ))
        .await;
        let session = events[0].session.as_ref().expect("session event");
        assert_eq!(session.session_id, "sess_1");
        assert!(session.compacted);
        assert_eq!(events[1].event_type, "citations");
        assert_eq!(events[1].citations.len(), 1);
        assert_eq!(events[1].citations[0].url, "https://rust-lang.org");
        assert_eq!(events[1].citations[0].index, 1);
        assert_eq!(events[2].delta.as_ref().unwrap().text, "hi");
        let usage = events[3].usage.as_ref().unwrap();
        assert_eq!(usage.output_tokens, 1);
        assert_eq!(usage.reasoning_tokens, Some(7));
        assert_eq!(usage.cached_tokens, None);
        assert!(events[4].done);
    }

    #[test]
    fn a_malformed_content_block_array_is_an_error_not_none() {
        let null: ChatMessage =
            serde_json::from_str(r#"{"role":"assistant","content_blocks":null}"#).unwrap();
        assert!(null.content_blocks.is_none());
        let bad = serde_json::from_str::<ChatMessage>(
            r#"{"role":"assistant","content_blocks":[{"type":42}]}"#,
        );
        assert!(
            bad.is_err(),
            "a malformed block array must not decode as None"
        );
    }
}
