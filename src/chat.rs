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

    /// Pins every turn of one conversation to the same provider prompt-cache
    /// shard. Any stable string the client keeps per conversation — the
    /// gateway hashes it with the caller's identity before forwarding it as
    /// OpenAI/xAI `prompt_cache_key` (or `x-grok-conv-id` on the xAI
    /// chat-completions lane). `None` = derived from the caller's identity
    /// alone, so all of one user's conversations share a shard. Generate one
    /// per conversation object and reuse it on every turn.
    ///
    /// Honored by `/qai/v1/chat` only: the session endpoint derives its key
    /// from the session ID and ignores a client-supplied one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,

    /// Vertex resource name of a previously created context cache (e.g.
    /// "cachedContents/abc123"). When set, the cached content is billed at
    /// the cached-read rate and need not be re-sent. Gemini-only; the
    /// cache's model must match this request's model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,

    /// Provider-specific settings, keyed by provider: an open map, so a key
    /// the gateway documents but this SDK version does not name still rides
    /// through as raw JSON.
    ///
    /// Documented keys:
    /// - `provider_options.openai.reasoning_summary`: `auto` | `concise` |
    ///   `detailed` | `none`
    /// - `provider_options.openai.reasoning_mode`: `standard` | `pro`
    /// - `provider_options.openai.verbosity`: `low` | `medium` | `high`
    /// - `provider_options.openai.text_format`: `text` | `json_object`
    /// - `provider_options.xai.native_files`: bool — send files to xAI
    ///   natively instead of extracting them gateway-side
    /// - `provider_options.anthropic.*` — thinking budget and friends
    ///
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
    /// One of "text", "thinking", "reasoning", "tool_use", "image", "file"
    /// or "file_uri".
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

    /// Gemini thought signature (base64). Present on `tool_use` blocks and,
    /// on Gemini 3, on the `text` block of a turn that ended in text — echo
    /// it back on the corresponding block of the next turn's assistant
    /// message. A streaming turn that ends in text carries it on the
    /// `thought_signature` event instead, see
    /// [`StreamEvent::thought_signature`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,

    /// The provider's own reasoning item, verbatim, on a block of type
    /// `reasoning`. Opaque: never inspect or rebuild it — pass the whole
    /// block back untouched, **in the position it arrived in**, on the next
    /// turn's assistant message. Its place among the `tool_use` blocks is
    /// how the provider learns where the reasoning sat; replaying it behind
    /// the call it reasoned about is a different conversation and the
    /// provider rejects it. Dropping it re-bills the reasoning tokens on
    /// every round of a tool loop.
    ///
    /// Distinct from a `thinking` block, which is the human-readable
    /// summary: one is for the reader, one is for the wire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<serde_json::Value>,

    /// Model that minted a `reasoning` block. Reasoning state is bound to
    /// its model, so a block is never replayed to a different one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minted_by: Option<String>,

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

    /// Total cost, from the X-QAI-Cost-Ticks header on a live call.
    ///
    /// `default` rather than `skip`: skip meant the field was never read from
    /// JSON at all, so a response reconstructed from a stored body — a disk
    /// cache, a replayed fixture, a proxy that moves the header into the
    /// envelope — came back reporting a cost of zero for a call that cost
    /// something. Read it when the body has it, fill it from the header when
    /// it does not.
    #[serde(default)]
    pub cost_ticks: i64,

    /// From the X-QAI-Request-Id header, or the body when it carries one.
    /// See [`cost_ticks`](Self::cost_ticks) for why this is `default`.
    #[serde(default)]
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
    /// i64 to match every other count and the cost beside them. They were i32
    /// while cached_tokens, reasoning_tokens and cost_ticks were i64, so any
    /// arithmetic across the buckets — which is most of what a caller does
    /// with these — needed a cast on two of the five fields and not the rest.
    pub input_tokens: i64,
    /// Output tokens billed at the output rate. Includes reasoning on the
    /// non-streaming envelope; excludes it on the streaming usage event.
    pub output_tokens: i64,
    /// What the call cost, covering input, output and reasoning.
    pub cost_ticks: i64,

    /// Input tokens served from the provider's prompt cache, billed at the
    /// lower cached rate. Omitted when the turn had no cache hit.
    #[serde(default)]
    pub cached_tokens: Option<i64>,

    /// Input tokens that triggered a cache WRITE, billed at a premium over
    /// standard input — Anthropic charges 1.25x base for the 5-minute TTL,
    /// GPT Image 2.5 $12.50/M against $8.00 input.
    ///
    /// These OVERLAP `input_tokens`, so reconciling a bill adds the premium
    /// on the write rate, never the tokens twice. Without the field the four
    /// billed buckets cannot be reconstructed from a response: prompt, cache
    /// reads, cache writes and output. A client could see what a call cost
    /// and not which part of it was the cache being filled. Omitted by
    /// providers that charge no write premium.
    #[serde(default)]
    pub cache_write_tokens: Option<i64>,

    /// The AUDIO share of `input_tokens`. Several Gemini models charge a
    /// premium for audio input — 3.3x the text rate on gemini-2.5-flash, 2x
    /// on gemini-3.1-flash-lite — so a turn carrying audio costs more than
    /// its token counts appear to justify.
    ///
    /// This OVERLAPS `input_tokens` and is never added to it: reconciling a
    /// bill applies the audio rate to these and the text rate to the
    /// remainder. Omitted when the turn carried no audio, which is nearly
    /// all of them, and by models that charge no audio premium — gpt-5.x and
    /// Claude price audio at the text rate and report nothing here.
    #[serde(default)]
    pub audio_tokens: Option<i64>,

    /// The audio share of `cached_tokens`, billed at the model's cached
    /// AUDIO rate rather than its cached text rate. Overlaps
    /// `cached_tokens` the same way `audio_tokens` overlaps `input_tokens`.
    #[serde(default)]
    pub cached_audio_tokens: Option<i64>,

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
    /// "tool_use" (atomic), "citations", "session", "usage",
    /// "thought_signature", "heartbeat", "error", "invalid_request",
    /// "rate_limit", "done".
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

    /// Gemini 3's signature for a stream that ended in text (base64), on the
    /// `thought_signature` event the gateway sends just before `done`. Store
    /// it on the assistant text block echoed back next turn — the same field
    /// [`ContentBlock::thought_signature`] carries on a non-streaming
    /// response. Absent on every other event and on providers that issue no
    /// signature.
    pub thought_signature: Option<String>,

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
            thought_signature: None,
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
    input_tokens: Option<i64>,
    #[serde(default)]
    output_tokens: Option<i64>,
    /// Portion of `output_tokens` spent on reasoning; carried by `usage`
    /// events, absent on non-reasoning models.
    #[serde(default)]
    reasoning_tokens: Option<i64>,
    #[serde(default)]
    cost_ticks: Option<i64>,
    /// Input tokens served from the provider's prompt cache. Carried by
    /// `usage` events, absent when the turn had no cache hit.
    #[serde(default)]
    cached_tokens: Option<i64>,
    /// Input tokens that triggered a cache write. Carried by `usage`
    /// events, absent when the turn wrote nothing to the cache.
    #[serde(default)]
    cache_write_tokens: Option<i64>,
    /// The audio share of the input, and of the cached input. Carried by
    /// `usage` events, absent when the turn carried no audio.
    #[serde(default)]
    audio_tokens: Option<i64>,
    #[serde(default)]
    cached_audio_tokens: Option<i64>,
    #[serde(default)]
    message: Option<String>,
    /// Carried by the `citations` event.
    #[serde(default)]
    citations: Option<Vec<Citation>>,
    /// Carried by the `thought_signature` event (and, for back-compat, by
    /// `tool_use` events) — base64 Gemini 3 signature.
    #[serde(default)]
    thought_signature: Option<String>,
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
        // The header fills these in; it does not overwrite a body that
        // already carried them. Assigning unconditionally would zero a
        // reconstructed response whenever the header is absent, which is the
        // failure `default` above exists to prevent.
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
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
                    // Gemini rides its signature on this event; the client
                    // echoes it on the tool_use block of the next turn.
                    ev.thought_signature = raw.thought_signature.clone();
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
                        cached_tokens: raw.cached_tokens,
                        cache_write_tokens: raw.cache_write_tokens,
                        audio_tokens: raw.audio_tokens,
                        cached_audio_tokens: raw.cached_audio_tokens,
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
                "thought_signature" => {
                    ev.thought_signature = raw.thought_signature;
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

#[cfg(test)]
mod usage_ledger_tests {
    use super::*;

    /// The four billed buckets have to be reconstructable from one response:
    /// prompt, cache reads, cache writes, output. Without cache_write_tokens a
    /// caller could see what a call cost and not which part of it was the
    /// cache being filled — on Anthropic that part bills at 1.25x input.
    #[test]
    fn usage_carries_all_four_billed_buckets() {
        let u: ChatUsage = serde_json::from_str(
            r#"{"input_tokens":1000,"output_tokens":250,"cost_ticks":4200,
                "cached_tokens":800,"cache_write_tokens":150,"reasoning_tokens":90}"#,
        )
        .expect("decode");

        assert_eq!(u.input_tokens, 1000);
        assert_eq!(u.output_tokens, 250);
        assert_eq!(u.cached_tokens, Some(800));
        assert_eq!(u.cache_write_tokens, Some(150));
        assert_eq!(u.reasoning_tokens, Some(90));

        // Every count is i64, so summing across the buckets needs no cast.
        let billed_input = u.input_tokens + u.cached_tokens.unwrap_or(0) + u.cache_write_tokens.unwrap_or(0);
        assert_eq!(billed_input, 1950);
    }

    /// Audio input is priced above text on several Gemini models — 3.3x on
    /// gemini-2.5-flash — so a turn carrying audio costs more than its token
    /// counts appear to justify. The share is reported so a caller can
    /// reconcile the charge.
    ///
    /// It OVERLAPS input_tokens and must never be added to it: the audio
    /// rate applies to these tokens and the text rate to the remainder.
    /// Summing them is the obvious way to get this wrong.
    #[test]
    fn audio_is_a_share_of_the_input_not_an_addition() {
        let u: ChatUsage = serde_json::from_str(
            r#"{"input_tokens":100000,"output_tokens":250,"cost_ticks":4200,
                "cached_tokens":20000,"audio_tokens":40000,"cached_audio_tokens":5000}"#,
        )
        .expect("decode");

        assert_eq!(u.audio_tokens, Some(40000));
        assert_eq!(u.cached_audio_tokens, Some(5000));

        // A share, never larger than the bucket it belongs to.
        assert!(u.audio_tokens.unwrap() <= u.input_tokens);
        assert!(u.cached_audio_tokens.unwrap() <= u.cached_tokens.unwrap());

        // The text remainder is what the base rate applies to.
        let text_input = u.input_tokens - u.audio_tokens.unwrap_or(0);
        assert_eq!(text_input, 60000);
    }

    /// A turn with no audio, or a model that prices audio at its text rate,
    /// reports nothing — the field stays None rather than becoming a zero
    /// that looks measured.
    #[test]
    fn a_turn_without_audio_reports_none() {
        let u: ChatUsage =
            serde_json::from_str(r#"{"input_tokens":10,"output_tokens":2,"cost_ticks":7}"#)
                .expect("decode");
        assert!(u.audio_tokens.is_none());
        assert!(u.cached_audio_tokens.is_none());
    }

    /// A provider that charges no write premium reports no bucket, and the
    /// field stays None rather than becoming a zero that looks measured.
    #[test]
    fn a_provider_without_a_write_premium_reports_none() {
        let u: ChatUsage =
            serde_json::from_str(r#"{"input_tokens":10,"output_tokens":2,"cost_ticks":7}"#)
                .expect("decode");
        assert!(u.cache_write_tokens.is_none());
        assert!(u.cached_tokens.is_none());
    }

    /// cost_ticks and request_id arrive in headers on a live call, and the
    /// fields were `skip` — never read from JSON at all. A response rebuilt
    /// from a stored body then reported a cost of zero for a call that cost
    /// something. `default` reads them when the body has them.
    #[test]
    fn a_response_rebuilt_from_a_stored_body_keeps_its_cost() {
        let r: ChatResponse = serde_json::from_str(
            r#"{"id":"msg_1","model":"claude-opus-5","content":[],
                "cost_ticks":123456,"request_id":"qai_req_stored"}"#,
        )
        .expect("decode");

        assert_eq!(r.cost_ticks, 123_456, "a stored cost was discarded");
        assert_eq!(r.request_id, "qai_req_stored", "a stored request id was discarded");
    }

    /// A live reply carries neither in the body; they default and the header
    /// injection fills them in.
    #[test]
    fn a_live_reply_without_them_defaults_rather_than_failing() {
        let r: ChatResponse =
            serde_json::from_str(r#"{"id":"msg_1","model":"claude-opus-5","content":[]}"#)
                .expect("decode");
        assert_eq!(r.cost_ticks, 0);
        assert!(r.request_id.is_empty());
    }
}

#[cfg(test)]
mod stream_usage_bucket_tests {
    use super::*;

    // The streaming usage event carries the same cache split as the
    // non-streaming envelope. It used to carry neither bucket, so a
    // streaming caller could see cost_ticks and never learn which part of
    // it was the cache.
    #[test]
    fn a_streaming_usage_event_carries_both_cache_buckets() {
        let raw: RawStreamEvent = serde_json::from_str(
            r#"{"type":"usage","input_tokens":900,"output_tokens":40,
                "reasoning_tokens":60,"cached_tokens":300,
                "cache_write_tokens":500,"cost_ticks":12345}"#,
        )
        .expect("usage event parses");
        assert_eq!(raw.cached_tokens, Some(300));
        assert_eq!(raw.cache_write_tokens, Some(500));
    }

    // And the same audio split, for the same reason: a streaming caller on a
    // model that prices audio above text would otherwise see a cost it could
    // not account for. The share reaches ChatUsage, not just the raw event.
    #[test]
    fn a_streaming_usage_event_carries_the_audio_share() {
        let raw: RawStreamEvent = serde_json::from_str(
            r#"{"type":"usage","input_tokens":100000,"output_tokens":40,
                "cached_tokens":20000,"audio_tokens":40000,
                "cached_audio_tokens":5000,"cost_ticks":12345}"#,
        )
        .expect("usage event parses");
        assert_eq!(raw.audio_tokens, Some(40000));
        assert_eq!(raw.cached_audio_tokens, Some(5000));
        // A share of the input it belongs to, never an addition to it.
        assert!(raw.audio_tokens.unwrap() <= raw.input_tokens.unwrap());
        assert!(raw.cached_audio_tokens.unwrap() <= raw.cached_tokens.unwrap());
    }

    // A gateway that reports no cache activity leaves the fields off the
    // wire entirely; absent must stay None rather than reading as zero.
    #[test]
    fn absent_cache_buckets_stay_none_not_zero() {
        let raw: RawStreamEvent = serde_json::from_str(
            r#"{"type":"usage","input_tokens":10,"output_tokens":5,"cost_ticks":1}"#,
        )
        .expect("usage event parses");
        assert_eq!(raw.cached_tokens, None);
        assert_eq!(raw.cache_write_tokens, None);
    }

    // The non-streaming envelope's four billed buckets survive a round
    // trip, and output_tokens there is the billable total.
    #[test]
    fn the_envelope_carries_all_four_billed_buckets() {
        let u: ChatUsage = serde_json::from_str(
            r#"{"input_tokens":1000,"cached_tokens":400,"cache_write_tokens":600,
                "output_tokens":75,"reasoning_tokens":25,"cost_ticks":999}"#,
        )
        .expect("usage parses");
        assert_eq!(u.input_tokens, 1000);
        assert_eq!(u.cached_tokens, Some(400));
        assert_eq!(u.cache_write_tokens, Some(600));
        assert_eq!(u.output_tokens, 75);
        assert_eq!(u.reasoning_tokens, Some(25));
    }
}

#[cfg(test)]
mod wire_contract_tests {
    use super::*;

    /// `prompt_cache_key` rides the request only when set, so a caller who
    /// never names one keeps the gateway's identity-derived default.
    #[test]
    fn prompt_cache_key_serializes_only_when_set() {
        let mut req = ChatRequest {
            model: "gpt-5.6".into(),
            messages: vec![ChatMessage::user("hi")],
            ..Default::default()
        };
        let bare: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert!(bare.get("prompt_cache_key").is_none());

        req.prompt_cache_key = Some("conv-7f3a".into());
        let keyed: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(keyed["prompt_cache_key"], "conv-7f3a");
    }

    /// A `reasoning` block round-trips byte-for-byte and keeps its place
    /// among the tool calls — position is the state the provider reads.
    #[test]
    fn reasoning_block_round_trips_verbatim() {
        let wire = r#"{
            "id": "req_1",
            "model": "gpt-5.6",
            "content": [
                {"type": "reasoning",
                 "reasoning": {"id": "rs_abc", "summary": [], "encrypted_content": "Zm9v"},
                 "minted_by": "gpt-5.6"},
                {"type": "tool_use", "id": "call_1", "name": "lookup", "input": {"q": "x"}}
            ],
            "stop_reason": "tool_use"
        }"#;
        let resp: ChatResponse = serde_json::from_str(wire).unwrap();
        assert_eq!(resp.content.len(), 2);

        let reasoning = &resp.content[0];
        assert_eq!(reasoning.block_type, "reasoning");
        assert_eq!(reasoning.minted_by.as_deref(), Some("gpt-5.6"));
        assert_eq!(
            reasoning.reasoning.as_ref().unwrap()["encrypted_content"],
            "Zm9v"
        );
        assert_eq!(resp.content[1].block_type, "tool_use");

        // Echoed back on the next turn's assistant message, unchanged and in
        // the same order.
        let echoed = serde_json::to_value(&ChatMessage {
            role: "assistant".into(),
            content_blocks: Some(resp.content.clone()),
            ..Default::default()
        })
        .unwrap();
        let blocks = echoed["content_blocks"].as_array().unwrap();
        assert_eq!(blocks[0]["type"], "reasoning");
        assert_eq!(blocks[0]["reasoning"]["id"], "rs_abc");
        assert_eq!(blocks[0]["minted_by"], "gpt-5.6");
        assert_eq!(blocks[1]["type"], "tool_use");
    }

    /// A block with no reasoning state omits both fields rather than
    /// sending nulls a provider would reject.
    #[test]
    fn plain_text_block_omits_reasoning_fields() {
        let block = ContentBlock {
            block_type: "text".into(),
            text: Some("hello".into()),
            ..Default::default()
        };
        let v = serde_json::to_value(&block).unwrap();
        assert!(v.get("reasoning").is_none());
        assert!(v.get("minted_by").is_none());
        assert!(v.get("thought_signature").is_none());
    }

    /// Gemini 3 puts the signature on the TEXT block, not only on tool_use.
    #[test]
    fn thought_signature_rides_a_text_block() {
        let resp: ChatResponse = serde_json::from_str(
            r#"{"id":"r","model":"gemini-3.5-flash","stop_reason":"stop",
                "content":[{"type":"text","text":"hi","thought_signature":"c2ln"}]}"#,
        )
        .unwrap();
        assert_eq!(resp.content[0].thought_signature.as_deref(), Some("c2ln"));

        let echoed = serde_json::to_value(&resp.content[0]).unwrap();
        assert_eq!(echoed["thought_signature"], "c2ln");
    }

    /// provider_options is an open map: an unknown provider key and an
    /// unknown key under a known provider both survive the round trip.
    #[test]
    fn provider_options_passes_through_unknown_keys() {
        let req = ChatRequest {
            model: "gpt-5.6".into(),
            messages: vec![ChatMessage::user("hi")],
            provider_options: Some(HashMap::from([
                (
                    "openai".to_string(),
                    serde_json::json!({
                        "reasoning_summary": "detailed",
                        "reasoning_mode": "pro",
                        "verbosity": "low",
                        "text_format": "json_object",
                        "a_key_this_sdk_never_heard_of": 42
                    }),
                ),
                ("xai".to_string(), serde_json::json!({"native_files": true})),
            ])),
            ..Default::default()
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["provider_options"]["openai"]["reasoning_mode"], "pro");
        assert_eq!(v["provider_options"]["openai"]["text_format"], "json_object");
        assert_eq!(
            v["provider_options"]["openai"]["a_key_this_sdk_never_heard_of"],
            42
        );
        assert_eq!(v["provider_options"]["xai"]["native_files"], true);
    }

    /// Every tier the gateway validates, `max` included, serializes as-is.
    #[test]
    fn reasoning_effort_carries_every_tier() {
        for tier in ["none", "low", "medium", "high", "xhigh", "max"] {
            let req = ChatRequest {
                model: "gpt-5.6".into(),
                messages: vec![ChatMessage::user("hi")],
                reasoning_effort: Some(tier.into()),
                ..Default::default()
            };
            assert_eq!(serde_json::to_value(&req).unwrap()["reasoning_effort"], tier);
        }
    }

    /// The `thought_signature` SSE event the gateway sends before `done`.
    #[tokio::test]
    async fn thought_signature_stream_event_is_parsed() {
        use futures_util::StreamExt;
        let body = concat!(
            "data: {\"type\":\"content_delta\",\"delta\":{\"text\":\"hi\"}}\n\n",
            "data: {\"type\":\"thought_signature\",\"thought_signature\":\"c2ln\"}\n\n",
            "data: [DONE]\n\n",
        );
        let chunks = futures_util::stream::iter(vec![Ok::<bytes::Bytes, reqwest::Error>(
            bytes::Bytes::from_static(body.as_bytes()),
        )]);
        let events: Vec<StreamEvent> = sse_to_events(chunks).collect().await;

        let sig = events
            .iter()
            .find(|e| e.event_type == "thought_signature")
            .expect("thought_signature event");
        assert_eq!(sig.thought_signature.as_deref(), Some("c2ln"));
        assert!(events[0].thought_signature.is_none());
    }
}
