//! Realtime voice sessions via WebSocket.
//!
//! Connects to the QAI Realtime API (proxied xAI Realtime) for bidirectional
//! audio streaming with voice activity detection, transcription, and tool calling.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() -> quantum_sdk::Result<()> {
//! let client = quantum_sdk::Client::new("qai_key_xxx")?;
//! let config = quantum_sdk::RealtimeConfig::default();
//!
//! let (mut sender, mut receiver) = client.realtime_connect(&config).await?;
//!
//! // Send audio in a task, receive events in another
//! tokio::spawn(async move {
//!     while let Some(event) = receiver.recv().await {
//!         match event {
//!             quantum_sdk::RealtimeEvent::AudioDelta { delta } => { /* play PCM */ }
//!             quantum_sdk::RealtimeEvent::TranscriptDone { transcript, .. } => {
//!                 println!("Transcript: {transcript}");
//!             }
//!             _ => {}
//!         }
//!     }
//! });
//!
//! // sender.send_audio(base64_pcm).await?;
//! # Ok(())
//! # }
//! ```

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::client::Client;
use crate::error::{ApiError, Error, Result};

type WsSink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsStream = futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

// ── Public types ──

/// Configuration for a realtime voice session.
#[derive(Debug, Clone, Serialize)]
pub struct RealtimeConfig {
    /// Voice to use (e.g. "Sal", "Eve", "Vesper" on xAI).
    pub voice: String,

    /// System instructions for the AI.
    pub instructions: String,

    /// PCM sample rate in Hz.
    pub sample_rate: u32,

    /// Tool definitions (xAI Realtime API format).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<serde_json::Value>,

    /// Model for the session (e.g. "grok-realtime-beta"). Sent to the gateway
    /// as the `model` query parameter, which it forwards upstream and bills
    /// against; empty means the gateway default. Also placed in the
    /// `session.update` frame.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
}

impl Default for RealtimeConfig {
    fn default() -> Self {
        Self {
            voice: "Sal".into(),
            instructions: String::new(),
            sample_rate: 24000,
            tools: Vec::new(),
            model: String::new(),
        }
    }
}

/// Parsed incoming event from the realtime API.
#[derive(Debug, Clone)]
pub enum RealtimeEvent {
    /// Session configuration acknowledged.
    SessionReady,

    /// Base64-encoded PCM audio chunk from the assistant.
    AudioDelta { delta: String },

    /// Partial transcript text.
    TranscriptDelta {
        delta: String,
        /// "input" for user speech, "output" for assistant speech.
        source: String,
    },

    /// Final transcript for a completed utterance.
    TranscriptDone {
        transcript: String,
        /// "input" for user speech, "output" for assistant speech.
        source: String,
    },

    /// Voice activity detected — user started speaking.
    SpeechStarted,

    /// Voice activity ended — user stopped speaking.
    SpeechStopped,

    /// The model is requesting a function/tool call.
    FunctionCall {
        name: String,
        call_id: String,
        arguments: String,
    },

    /// The model finished its response turn.
    ResponseDone,

    /// An error from the realtime API.
    Error { message: String },

    /// The peer closed the socket. `reason` carries the close frame's text,
    /// which the gateway uses to say why ("insufficient balance", "session
    /// duration limit reached"); empty on a plain hang-up.
    Closed { code: Option<u16>, reason: String },

    /// The socket failed while reading; nothing more will arrive.
    TransportError { message: String },

    /// An event type we don't explicitly handle.
    Unknown(serde_json::Value),
}

/// Write half of a realtime session — send audio and control messages.
pub struct RealtimeSender {
    sink: tokio::sync::Mutex<WsSink>,
}

/// Read half of a realtime session — receive audio, transcripts, and tool calls.
pub struct RealtimeReceiver {
    stream: WsStream,
}

// ── Client method ──

impl Client {
    /// Opens a realtime voice session through the gateway proxy.
    ///
    /// Returns `(sender, receiver)` for bidirectional communication. The
    /// connection is made to `{base_url}/qai/v1/realtime` with the client's
    /// credentials, `config.model` rides as the `model` query parameter,
    /// and a `session.update` frame built from `config` is sent first.
    pub async fn realtime_connect(
        &self,
        config: &RealtimeConfig,
    ) -> Result<(RealtimeSender, RealtimeReceiver)> {
        let path = if config.model.is_empty() {
            "/qai/v1/realtime".to_string()
        } else {
            format!(
                "/qai/v1/realtime?model={}",
                urlencoding::encode(&config.model)
            )
        };
        let (sender, receiver) = self.connect_gateway_websocket(&path).await?;

        let session_update = build_session_update(config);
        sender
            .send_raw(&serde_json::to_string(&session_update)?)
            .await?;

        Ok((sender, receiver))
    }
}

/// Response from the QAI realtime session endpoint.
///
/// Two shapes share this type. An xAI session carries `ephemeral_token`,
/// `url` and no `provider`; an ElevenLabs session carries `signed_url`
/// (whose query string is the credential) and `provider = "elevenlabs"`.
/// [`ws_url`](Self::ws_url) picks whichever is set.
#[derive(Clone, serde::Deserialize)]
pub struct RealtimeSession {
    /// Ephemeral token for a direct xAI WebSocket connection.
    #[serde(default)]
    pub ephemeral_token: String,
    /// WebSocket URL for an xAI session ("wss://api.x.ai/v1/realtime").
    #[serde(default)]
    pub url: String,
    /// Signed WebSocket URL for an ElevenLabs session; the credential is in
    /// the URL.
    #[serde(default)]
    pub signed_url: String,
    /// Session ID for billing (pass to realtime/end on disconnect).
    #[serde(default)]
    pub session_id: String,
    /// `"elevenlabs"` for ElevenLabs sessions; empty for xAI.
    #[serde(default)]
    pub provider: String,
}

/// The token and the signed URL are credentials, so `Debug` masks them.
impl std::fmt::Debug for RealtimeSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let masked = |v: &str| if v.is_empty() { "" } else { "<redacted>" };
        f.debug_struct("RealtimeSession")
            .field("ephemeral_token", &masked(&self.ephemeral_token))
            .field("url", &self.url)
            .field("signed_url", &masked(&self.signed_url))
            .field("session_id", &self.session_id)
            .field("provider", &self.provider)
            .finish()
    }
}

/// Backwards-compatible alias for [`RealtimeSession`].
pub type RealtimeSessionResponse = RealtimeSession;

impl RealtimeSession {
    /// Get the WebSocket URL — checks both `url` and `signed_url` fields.
    pub fn ws_url(&self) -> &str {
        if !self.signed_url.is_empty() {
            &self.signed_url
        } else {
            &self.url
        }
    }
}

impl Client {
    /// Request an ephemeral token from the QAI proxy for direct voice connection.
    /// Call this before `realtime_connect_direct` to get a scoped token.
    pub async fn realtime_session(&self) -> Result<RealtimeSession> {
        self.realtime_session_for(None).await
    }

    /// Request an ephemeral token for a backend. The gateway recognises
    /// `"elevenlabs"`; any other value, or `None`, mints an xAI token.
    pub async fn realtime_session_for(&self, provider: Option<&str>) -> Result<RealtimeSession> {
        self.realtime_session_with(provider, serde_json::json!({}))
            .await
    }

    /// Request a realtime session with full configuration.
    /// The body is sent as-is to POST /qai/v1/realtime/session.
    /// Use this to pass voice, prompt, tools, etc. for ElevenLabs ConvAI.
    pub async fn realtime_session_with(
        &self,
        provider: Option<&str>,
        mut body: serde_json::Value,
    ) -> Result<RealtimeSession> {
        if let Some(p) = provider {
            body["provider"] = serde_json::Value::String(p.to_string());
        }
        let (session, _meta): (RealtimeSession, _) =
            self.post_json("/qai/v1/realtime/session", &body).await?;
        Ok(session)
    }

    /// End a realtime session and settle its bill. The gateway charges the
    /// longer of its own clock and `duration_seconds`, less the minute it
    /// pre-authorised at session start.
    pub async fn realtime_end(&self, session_id: &str, duration_seconds: u64) -> Result<()> {
        let _: (serde_json::Value, _) = self
            .post_json(
                "/qai/v1/realtime/end",
                &serde_json::json!({
                    "session_id": session_id,
                    "duration_seconds": duration_seconds,
                }),
            )
            .await?;
        Ok(())
    }

    /// Refresh an ephemeral token for long sessions (>4 min).
    pub async fn realtime_refresh(&self, session_id: &str) -> Result<String> {
        let (resp, _): (serde_json::Value, _) = self
            .post_json(
                "/qai/v1/realtime/refresh",
                &serde_json::json!({ "session_id": session_id }),
            )
            .await?;
        Ok(resp["ephemeral_token"].as_str().unwrap_or("").to_string())
    }
}

/// Opens a realtime voice session directly to xAI (bypassing the proxy).
///
/// Use with an ephemeral token from `client.realtime_session()`.
/// Much lower latency than the proxy path — no extra hop.
pub async fn realtime_connect_direct(
    ephemeral_token: &str,
    config: &RealtimeConfig,
) -> Result<(RealtimeSender, RealtimeReceiver)> {
    realtime_connect_direct_to("wss://api.x.ai/v1/realtime", ephemeral_token, config).await
}

/// Opens a realtime voice session to a specific WebSocket URL, sending
/// `token` as a bearer and a `session.update` frame built from `config`.
/// This is the xAI/OpenAI protocol: an ElevenLabs signed URL speaks a
/// different one and belongs with [`Client::elevenlabs_connect`].
pub async fn realtime_connect_direct_to(
    url: &str,
    token: &str,
    config: &RealtimeConfig,
) -> Result<(RealtimeSender, RealtimeReceiver)> {
    // Extract host from URL
    let host = url
        .trim_start_matches("wss://")
        .trim_start_matches("ws://")
        .split('/')
        .next()
        .unwrap_or("api.x.ai");

    let request = Request::builder()
        .uri(url)
        .header("Host", host)
        .header("Authorization", format!("Bearer {token}"))
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .map_err(|e| {
            Error::Api(ApiError {
                status_code: 0,
                code: "websocket_request".into(),
                message: format!("Failed to build WebSocket request: {e}"),
                request_id: String::new(),
            })
        })?;

    let (ws_stream, _response) = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio_tungstenite::connect_async(request),
    )
    .await
    .map_err(|_| {
        Error::Api(ApiError {
            status_code: 0,
            code: "timeout".into(),
            message: "Direct xAI WebSocket connection timed out (10s)".into(),
            request_id: String::new(),
        })
    })?
    .map_err(Error::from)?;

    let (sink, stream) = ws_stream.split();
    let sender = RealtimeSender {
        sink: tokio::sync::Mutex::new(sink),
    };
    let receiver = RealtimeReceiver { stream };

    // Send session.update
    let session_update = build_session_update(config);
    sender
        .send_raw(&serde_json::to_string(&session_update)?)
        .await?;

    Ok((sender, receiver))
}

// ── Session update builder ──

/// Build the `session.update` JSON payload from config. A `gpt-` model gets
/// the OpenAI frame shape; everything else, including the gateway's
/// `grok-realtime` defaults, gets xAI's.
fn build_session_update(config: &RealtimeConfig) -> serde_json::Value {
    let is_openai = config.model.starts_with("gpt-");

    let mut session = serde_json::json!({
        "voice": config.voice,
        "instructions": config.instructions,
        "turn_detection": { "type": "server_vad" },
        "tools": config.tools,
    });

    if !config.model.is_empty() {
        session["model"] = serde_json::Value::String(config.model.clone());
    }

    if is_openai {
        // OpenAI Realtime API format: modalities + input_audio_format/output_audio_format
        session["modalities"] = serde_json::json!(["text", "audio"]);
        session["input_audio_format"] = serde_json::json!("pcm16");
        session["output_audio_format"] = serde_json::json!("pcm16");
        session["input_audio_transcription"] =
            serde_json::json!({ "model": "gpt-4o-mini-transcribe" });
    } else {
        // xAI Realtime API format
        session["input_audio_transcription"] = serde_json::json!({ "model": "grok-2-audio" });
        session["audio"] = serde_json::json!({
            "input": { "format": { "type": "audio/pcm", "rate": config.sample_rate } },
            "output": { "format": { "type": "audio/pcm", "rate": config.sample_rate } },
        });
    }

    serde_json::json!({
        "type": "session.update",
        "session": session,
    })
}

// ── RealtimeSender ──

impl RealtimeSender {
    /// Send a base64-encoded PCM audio chunk.
    pub async fn send_audio(&self, base64_pcm: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "input_audio_buffer.append",
            "audio": base64_pcm,
        });
        self.send_raw(&serde_json::to_string(&msg)?).await
    }

    /// Send a text message (creates a conversation item and requests a response).
    pub async fn send_text(&self, text: &str) -> Result<()> {
        let item = serde_json::json!({
            "type": "conversation.item.create",
            "item": {
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": text,
                }]
            }
        });
        self.send_raw(&serde_json::to_string(&item)?).await?;

        let response = serde_json::json!({
            "type": "response.create",
            "response": {
                "modalities": ["text", "audio"],
            }
        });
        self.send_raw(&serde_json::to_string(&response)?).await
    }

    /// Send a function/tool call result back to the model.
    pub async fn send_function_result(&self, call_id: &str, output: &str) -> Result<()> {
        let item = serde_json::json!({
            "type": "conversation.item.create",
            "item": {
                "type": "function_call_output",
                "call_id": call_id,
                "output": output,
            }
        });
        self.send_raw(&serde_json::to_string(&item)?).await?;

        let response = serde_json::json!({
            "type": "response.create",
        });
        self.send_raw(&serde_json::to_string(&response)?).await
    }

    /// Cancel the current response (interrupt the model).
    pub async fn cancel_response(&self) -> Result<()> {
        let msg = serde_json::json!({ "type": "response.cancel" });
        self.send_raw(&serde_json::to_string(&msg)?).await
    }

    /// Send one base64-encoded PCM chunk on an ElevenLabs conversational
    /// socket opened with
    /// [`Client::elevenlabs_connect`].
    ///
    /// That protocol takes microphone audio as `user_audio_chunk`, not the
    /// `input_audio_buffer.append` frame [`RealtimeSender::send_audio`] sends.
    pub async fn send_elevenlabs_audio(&self, base64_pcm: &str) -> Result<()> {
        let msg = serde_json::json!({ "user_audio_chunk": base64_pcm });
        self.send_raw(&serde_json::to_string(&msg)?).await
    }

    /// Send an arbitrary JSON frame — the escape hatch for provider protocols
    /// the typed helpers do not cover.
    pub async fn send_json(&self, value: &serde_json::Value) -> Result<()> {
        self.send_raw(&serde_json::to_string(value)?).await
    }

    /// Close the WebSocket connection gracefully.
    pub async fn close(self) -> Result<()> {
        let mut sink = self.sink.into_inner();
        sink.close().await.map_err(Error::from)
    }

    /// Send a raw text frame.
    async fn send_raw(&self, text: &str) -> Result<()> {
        let mut sink = self.sink.lock().await;
        sink.send(Message::Text(text.into()))
            .await
            .map_err(Error::from)
    }
}

// ── RealtimeReceiver ──

impl RealtimeReceiver {
    /// Receive the next event. A close frame or a read failure is delivered
    /// as [`RealtimeEvent::Closed`] / [`RealtimeEvent::TransportError`] and
    /// then `None` on every later call.
    pub async fn recv(&mut self) -> Option<RealtimeEvent> {
        loop {
            let msg = self.stream.next().await?;
            match msg {
                Ok(Message::Text(text)) => {
                    return Some(parse_event(&text));
                }
                Ok(Message::Close(frame)) => {
                    return Some(RealtimeEvent::Closed {
                        code: frame.as_ref().map(|f| u16::from(f.code)),
                        reason: frame.map(|f| f.reason.to_string()).unwrap_or_default(),
                    });
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => continue,
                Ok(Message::Binary(_)) => continue,
                Err(e) => {
                    return Some(RealtimeEvent::TransportError {
                        message: e.to_string(),
                    });
                }
            }
        }
    }
}

// ── Event parsing ──

fn parse_event(text: &str) -> RealtimeEvent {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
        return RealtimeEvent::Unknown(serde_json::Value::String(text.to_string()));
    };

    let event_type = v["type"].as_str().unwrap_or("");

    match event_type {
        "session.updated" => RealtimeEvent::SessionReady,

        "response.audio.delta" => RealtimeEvent::AudioDelta {
            delta: v["delta"].as_str().unwrap_or("").to_string(),
        },

        // Some API versions use "response.output_audio.delta"
        "response.output_audio.delta" => RealtimeEvent::AudioDelta {
            delta: v["delta"].as_str().unwrap_or("").to_string(),
        },

        "response.audio_transcript.delta" | "response.output_audio_transcript.delta" => {
            RealtimeEvent::TranscriptDelta {
                delta: v["delta"].as_str().unwrap_or("").to_string(),
                source: "output".into(),
            }
        }

        "response.audio_transcript.done" | "response.output_audio_transcript.done" => {
            RealtimeEvent::TranscriptDone {
                transcript: v["transcript"].as_str().unwrap_or("").to_string(),
                source: "output".into(),
            }
        }

        "conversation.item.input_audio_transcription.completed" => RealtimeEvent::TranscriptDone {
            transcript: v["transcript"].as_str().unwrap_or("").to_string(),
            source: "input".into(),
        },

        "input_audio_buffer.speech_started" => RealtimeEvent::SpeechStarted,
        "input_audio_buffer.speech_stopped" => RealtimeEvent::SpeechStopped,

        "response.function_call_arguments.done" => RealtimeEvent::FunctionCall {
            name: v["name"].as_str().unwrap_or("").to_string(),
            call_id: v["call_id"].as_str().unwrap_or("").to_string(),
            arguments: v["arguments"].as_str().unwrap_or("").to_string(),
        },

        "response.done" => RealtimeEvent::ResponseDone,

        "error" => RealtimeEvent::Error {
            message: v["error"]["message"]
                .as_str()
                .or_else(|| v["message"].as_str())
                .unwrap_or("unknown error")
                .to_string(),
        },

        _ => RealtimeEvent::Unknown(v),
    }
}

/// Maps a failed upgrade onto the crate's error types. The gateway refuses
/// an upgrade with its usual JSON error body (401 unauthenticated, 402
/// insufficient balance, 503 not configured), which becomes an
/// [`Error::Api`] so `status_code` and `typed_code` work on this path too.
fn handshake_error(e: tokio_tungstenite::tungstenite::Error) -> Error {
    use tokio_tungstenite::tungstenite::Error as WsError;
    match e {
        WsError::Http(resp) => {
            let status_code = resp.status().as_u16();
            let body = resp
                .body()
                .as_deref()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            let parsed: Option<serde_json::Value> = serde_json::from_str(&body).ok();
            let field = |k: &str| {
                parsed
                    .as_ref()
                    .and_then(|v| v["error"][k].as_str().or_else(|| v[k].as_str()))
                    .map(str::to_string)
            };
            Error::Api(ApiError {
                status_code,
                code: field("code").unwrap_or_else(|| "websocket_upgrade".into()),
                message: field("message").unwrap_or(body),
                request_id: String::new(),
            })
        }
        other => Error::from(other),
    }
}

// ── ElevenLabs conversational proxy ──

/// Connection parameters for the ElevenLabs conversational-voice proxy.
///
/// Every field is optional: the gateway falls back to its own default voice
/// and model, and creates a conversational agent on the fly when `agent_id` is
/// absent.
#[derive(Debug, Clone, Default)]
pub struct ElevenLabsProxyConfig {
    /// ElevenLabs voice id. Applied when the gateway creates the agent for
    /// this session; an existing `agent_id` keeps its own voice.
    pub voice_id: Option<String>,

    /// ElevenLabs model id. Applied when the gateway creates the agent for
    /// this session, like `voice_id`.
    pub model: Option<String>,

    /// An existing conversational agent to connect to. Omit to have the
    /// gateway create one for this session.
    pub agent_id: Option<String>,
}

impl ElevenLabsProxyConfig {
    /// Renders the config as the proxy's query string (empty when nothing is
    /// set).
    fn query(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref voice_id) = self.voice_id {
            params.push(format!("voice_id={}", urlencoding::encode(voice_id)));
        }
        if let Some(ref model) = self.model {
            params.push(format!("model={}", urlencoding::encode(model)));
        }
        if let Some(ref agent_id) = self.agent_id {
            params.push(format!("agent_id={}", urlencoding::encode(agent_id)));
        }
        if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        }
    }
}

impl Client {
    /// Opens an ElevenLabs conversational voice session through the gateway
    /// proxy.
    ///
    /// The gateway holds the ElevenLabs credential, reserves one funded minute
    /// up front, and meters the session minute by minute, so the connection
    /// closes when the balance runs out.
    ///
    /// The frames on this socket are ElevenLabs' conversational protocol, not
    /// the xAI/OpenAI realtime one: send microphone audio with
    /// [`RealtimeSender::send_elevenlabs_audio`] and anything else with
    /// [`RealtimeSender::send_json`]. Incoming frames the shared parser does
    /// not recognise arrive as [`RealtimeEvent::Unknown`] carrying the raw
    /// JSON.
    ///
    /// `GET /qai/v1/realtime/elevenlabs` (WebSocket upgrade)
    pub async fn elevenlabs_connect(
        &self,
        config: &ElevenLabsProxyConfig,
    ) -> Result<(RealtimeSender, RealtimeReceiver)> {
        let path = format!("/qai/v1/realtime/elevenlabs{}", config.query());
        self.connect_gateway_websocket(&path).await
    }

    /// Opens a WebSocket to a gateway path, carrying this client's credentials
    /// on the handshake.
    async fn connect_gateway_websocket(
        &self,
        path: &str,
    ) -> Result<(RealtimeSender, RealtimeReceiver)> {
        let base = self.base_url();
        let ws_base = if let Some(rest) = base.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = base.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            return Err(Error::Api(ApiError {
                status_code: 0,
                code: "invalid_base_url".into(),
                message: format!("Cannot convert base URL to WebSocket: {base}"),
                request_id: String::new(),
            }));
        };

        let host = base
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();

        let auth = self.auth_header().to_str().unwrap_or("").to_string();
        let raw_token = auth.strip_prefix("Bearer ").unwrap_or(&auth);

        let request = Request::builder()
            .uri(format!("{ws_base}{path}"))
            .header("Host", &host)
            .header("Authorization", &auth)
            .header("X-API-Key", raw_token)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .map_err(|e| {
                Error::Api(ApiError {
                    status_code: 0,
                    code: "websocket_request".into(),
                    message: format!("Failed to build WebSocket request: {e}"),
                    request_id: String::new(),
                })
            })?;

        let (ws_stream, _response) = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            tokio_tungstenite::connect_async(request),
        )
        .await
        .map_err(|_| {
            Error::Api(ApiError {
                status_code: 0,
                code: "timeout".into(),
                message: "WebSocket connection timed out (15s)".into(),
                request_id: String::new(),
            })
        })?
        .map_err(handshake_error)?;

        let (sink, stream) = ws_stream.split();
        Ok((
            RealtimeSender {
                sink: tokio::sync::Mutex::new(sink),
            },
            RealtimeReceiver { stream },
        ))
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevenlabs_config_renders_only_the_fields_that_are_set() {
        let empty = ElevenLabsProxyConfig::default();
        assert_eq!(empty.query(), "");

        let config = ElevenLabsProxyConfig {
            voice_id: Some("21m00Tcm4TlvDq8ikWAM".into()),
            agent_id: Some("agent 7".into()),
            ..Default::default()
        };
        assert_eq!(
            config.query(),
            "?voice_id=21m00Tcm4TlvDq8ikWAM&agent_id=agent%207"
        );
    }

    #[test]
    fn default_config() {
        let config = RealtimeConfig::default();
        assert_eq!(config.voice, "Sal");
        assert_eq!(config.sample_rate, 24000);
        assert!(config.instructions.is_empty());
        assert!(config.tools.is_empty());
        assert!(config.model.is_empty());
    }

    #[test]
    fn config_serialization() {
        let config = RealtimeConfig {
            voice: "Eve".into(),
            instructions: "You are a helpful assistant.".into(),
            sample_rate: 16000,
            tools: vec![serde_json::json!({
                "type": "function",
                "name": "get_weather",
                "description": "Get weather for a location",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": { "type": "string" }
                    },
                    "required": ["location"]
                }
            })],
            model: String::new(),
        };

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["voice"], "Eve");
        assert_eq!(json["sample_rate"], 16000);
        assert_eq!(json["tools"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn grok_models_get_the_xai_frame() {
        let mut config = RealtimeConfig {
            model: "grok-realtime-beta".into(),
            ..RealtimeConfig::default()
        };
        let frame = build_session_update(&config);
        assert!(frame["session"]["audio"].is_object());
        assert!(frame["session"].get("modalities").is_none());

        config.model = "gpt-4o-realtime-preview".into();
        let frame = build_session_update(&config);
        assert_eq!(frame["session"]["input_audio_format"], "pcm16");
    }

    #[test]
    fn session_debug_masks_credentials() {
        let session: RealtimeSession = serde_json::from_str(
            r#"{"ephemeral_token":"sk-secret","url":"wss://api.x.ai/v1/realtime","session_id":"vs_1"}"#,
        )
        .unwrap();
        let text = format!("{session:?}");
        assert!(!text.contains("sk-secret"));
        assert!(text.contains("vs_1"));
    }

    #[test]
    fn parse_session_ready() {
        let event = parse_event(r#"{"type":"session.updated","session":{}}"#);
        assert!(matches!(event, RealtimeEvent::SessionReady));
    }

    #[test]
    fn parse_audio_delta() {
        let event = parse_event(r#"{"type":"response.audio.delta","delta":"AQID"}"#);
        match event {
            RealtimeEvent::AudioDelta { delta } => assert_eq!(delta, "AQID"),
            _ => panic!("expected AudioDelta"),
        }
    }

    #[test]
    fn parse_transcript_done() {
        let event = parse_event(
            r#"{"type":"conversation.item.input_audio_transcription.completed","transcript":"hello"}"#,
        );
        match event {
            RealtimeEvent::TranscriptDone { transcript, source } => {
                assert_eq!(transcript, "hello");
                assert_eq!(source, "input");
            }
            _ => panic!("expected TranscriptDone"),
        }
    }

    #[test]
    fn parse_function_call() {
        let event = parse_event(
            r#"{"type":"response.function_call_arguments.done","name":"get_weather","call_id":"call_123","arguments":"{\"location\":\"London\"}"}"#,
        );
        match event {
            RealtimeEvent::FunctionCall {
                name,
                call_id,
                arguments,
            } => {
                assert_eq!(name, "get_weather");
                assert_eq!(call_id, "call_123");
                assert!(arguments.contains("London"));
            }
            _ => panic!("expected FunctionCall"),
        }
    }

    #[test]
    fn parse_error() {
        let event = parse_event(r#"{"type":"error","error":{"message":"rate limited"}}"#);
        match event {
            RealtimeEvent::Error { message } => assert_eq!(message, "rate limited"),
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn parse_unknown() {
        let event = parse_event(r#"{"type":"some.future.event","data":42}"#);
        assert!(matches!(event, RealtimeEvent::Unknown(_)));
    }

    #[test]
    fn parse_speech_events() {
        assert!(matches!(
            parse_event(r#"{"type":"input_audio_buffer.speech_started"}"#),
            RealtimeEvent::SpeechStarted
        ));
        assert!(matches!(
            parse_event(r#"{"type":"input_audio_buffer.speech_stopped"}"#),
            RealtimeEvent::SpeechStopped
        ));
        assert!(matches!(
            parse_event(r#"{"type":"response.done"}"#),
            RealtimeEvent::ResponseDone
        ));
    }

    #[ignore]
    #[tokio::test]
    async fn live_connect() {
        // Requires a running QAI server and valid API key.
        let key = std::env::var("QAI_API_KEY").expect("QAI_API_KEY required");
        let client = crate::Client::new(key).unwrap();
        let config = RealtimeConfig::default();

        let (sender, mut receiver) = client.realtime_connect(&config).await.unwrap();

        // Should receive SessionReady
        let event = receiver.recv().await.unwrap();
        assert!(matches!(event, RealtimeEvent::SessionReady));

        sender.close().await.unwrap();
    }
}
