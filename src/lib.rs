//! Rust client SDK for the Quantum AI API.
//!
//! Supports text generation (with streaming), image/video/audio generation,
//! embeddings, RAG search, and model listing through a single API endpoint.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() -> quantum_sdk::Result<()> {
//! let client = quantum_sdk::Client::new("your-api-key");
//!
//! let resp = client.chat(&quantum_sdk::ChatRequest {
//!     model: "claude-sonnet-4-6".into(),
//!     messages: vec![quantum_sdk::ChatMessage::user("Hello!")],
//!     ..Default::default()
//! }).await?;
//!
//! println!("{}", resp.text());
//! # Ok(())
//! # }
//! ```

pub mod audio;
pub mod chat;
pub mod client;
pub mod documents;
pub mod embeddings;
pub mod error;
pub mod image;
pub mod models;
pub mod rag;
pub mod video;

// Re-export primary types at crate root for convenience.
pub use client::{Client, ClientBuilder, ResponseMeta, DEFAULT_BASE_URL, TICKS_PER_USD};
pub use error::{ApiError, Error, Result};

// Chat types
pub use chat::{
    ChatMessage, ChatRequest, ChatResponse, ChatStream, ChatTool, ChatUsage, ContentBlock,
    StreamDelta, StreamEvent, StreamToolUse,
};

// Image types
pub use image::{
    GeneratedImage, ImageEditRequest, ImageEditResponse, ImageRequest, ImageResponse,
};

// Video types
pub use video::{GeneratedVideo, VideoRequest, VideoResponse};

// Audio types
pub use audio::{
    MusicClip, MusicRequest, MusicResponse, SttRequest, SttResponse, TtsRequest, TtsResponse,
};

// Embeddings types
pub use embeddings::{EmbedRequest, EmbedResponse};

// Document types
pub use documents::{DocumentRequest, DocumentResponse};

// RAG types
pub use rag::{
    RagCorpus, RagResult, RagSearchRequest, RagSearchResponse, SurrealRagResult,
    SurrealRagSearchRequest, SurrealRagSearchResponse,
};

// Model types
pub use models::{ModelInfo, PricingInfo};

// Error helpers
pub use error::{is_auth_error, is_not_found_error, is_rate_limit_error};
