//! Realtime voice sessions via WebSocket.
//!
//! Connects to the QAI Realtime API (proxied xAI Realtime) for bidirectional
//! audio streaming with voice activity detection, transcription, and tool calling.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() -> quantum_sdk::Result<()> {
//! let client = quantum_sdk::Client::new("qai_key_xxx");
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
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::client::Client;
use crate::error::{ApiError, Error, Result};

type WsSink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsStream = futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

// ── Public types ──

/// Configuration for a realtime voice session.
#[derive(Debug, Clone, Serialize)]
pub struct RealtimeConfig {
    /// Voice to use (e.g. "Sal", "Eve", "Vesper").
    pub voice: String,

    /// System instructions for the AI.
    pub instructions: String,

    /// PCM sample rate in Hz.
    pub sample_rate: u32,

    /// Tool definitions (xAI Realtime API format).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<serde_json::Value>,
}

impl Default for RealtimeConfig {
    fn default() -> Self {
        Self {
            voice: "Sal".into(),
            instructions: String::new(),
            sample_rate: 24000,
            tools: Vec::new(),
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
    /// Opens a realtime voice session via WebSocket.
    ///
    /// Returns `(sender, receiver)` for bidirectional communication.
    /// The connection is made to `{base_url}/qai/v1/realtime` with the
    /// client's auth token.
    pub async fn realtime_connect(
        &self,
        config: &RealtimeConfig,
    ) -> Result<(RealtimeSender, RealtimeReceiver)> {
        // Convert https:// → wss://, http:// → ws://
        let base = self.base_url();
        let ws_base = if base.starts_with("https://") {
            format!("wss://{}", &base[8..])
        } else if base.starts_with("http://") {
            format!("ws://{}", &base[7..])
        } else {
            return Err(Error::Api(ApiError {
                status_code: 0,
                code: "invalid_base_url".into(),
                message: format!("Cannot convert base URL to WebSocket: {base}"),
                request_id: String::new(),
            }));
        };

        let url = format!("{ws_base}/qai/v1/realtime");
        let auth = self
            .auth_header()
            .to_str()
            .unwrap_or("")
            .to_string();

        // Extract raw token (strip "Bearer " prefix) for X-API-Key
        let raw_token = auth.strip_prefix("Bearer ").unwrap_or(&auth);

        let request = Request::builder()
            .uri(&url)
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
            .map_err(|e| Error::Api(ApiError {
                status_code: 0,
                code: "websocket_request".into(),
                message: format!("Failed to build WebSocket request: {e}"),
                request_id: String::new(),
            }))?;

        // Connect with timeout
        let (ws_stream, _response) = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            tokio_tungstenite::connect_async(request),
        )
        .await
        .map_err(|_| Error::Api(ApiError {
            status_code: 0,
            code: "timeout".into(),
            message: "WebSocket connection timed out (15s)".into(),
            request_id: String::new(),
        }))?
        .map_err(Error::WebSocket)?;

        let (sink, stream) = ws_stream.split();
        let sender = RealtimeSender {
            sink: tokio::sync::Mutex::new(sink),
        };
        let receiver = RealtimeReceiver { stream };

        // Send session.update with config
        let session_update = serde_json::json!({
            "type": "session.update",
            "session": {
                "voice": config.voice,
                "instructions": config.instructions,
                "input_audio_format": "pcm16",
                "output_audio_format": "pcm16",
                "input_audio_transcription": {
                    "model": "grok-2-audio",
                },
                "turn_detection": {
                    "type": "server_vad",
                },
                "tools": config.tools,
            }
        });

        sender.send_raw(&serde_json::to_string(&session_update)?).await?;

        Ok((sender, receiver))
    }
}

// ── RealtimeSender ──

// SAFETY: WsSink contains a TcpStream which is Send, and we wrap in tokio::sync::Mutex.
unsafe impl Send for RealtimeSender {}
unsafe impl Sync for RealtimeSender {}

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

    /// Close the WebSocket connection gracefully.
    pub async fn close(self) -> Result<()> {
        let mut sink = self.sink.into_inner();
        sink.close().await.map_err(Error::WebSocket)
    }

    /// Send a raw text frame.
    async fn send_raw(&self, text: &str) -> Result<()> {
        let mut sink = self.sink.lock().await;
        sink.send(Message::Text(text.into()))
            .await
            .map_err(Error::WebSocket)
    }
}

// ── RealtimeReceiver ──

impl RealtimeReceiver {
    /// Receive the next event. Returns `None` when the connection closes.
    pub async fn recv(&mut self) -> Option<RealtimeEvent> {
        loop {
            let msg = self.stream.next().await?;
            match msg {
                Ok(Message::Text(text)) => {
                    return Some(parse_event(&text));
                }
                Ok(Message::Close(_)) => return None,
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => continue,
                Ok(Message::Binary(_)) => continue,
                Err(_) => return None,
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

        "conversation.item.input_audio_transcription.completed" => {
            RealtimeEvent::TranscriptDone {
                transcript: v["transcript"].as_str().unwrap_or("").to_string(),
                source: "input".into(),
            }
        }

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

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = RealtimeConfig::default();
        assert_eq!(config.voice, "Sal");
        assert_eq!(config.sample_rate, 24000);
        assert!(config.instructions.is_empty());
        assert!(config.tools.is_empty());
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
        };

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["voice"], "Eve");
        assert_eq!(json["sample_rate"], 16000);
        assert_eq!(json["tools"].as_array().unwrap().len(), 1);
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
            RealtimeEvent::FunctionCall { name, call_id, arguments } => {
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
        let client = crate::Client::new(key);
        let config = RealtimeConfig::default();

        let (sender, mut receiver) = client.realtime_connect(&config).await.unwrap();

        // Should receive SessionReady
        let event = receiver.recv().await.unwrap();
        assert!(matches!(event, RealtimeEvent::SessionReady));

        sender.close().await.unwrap();
    }
}
