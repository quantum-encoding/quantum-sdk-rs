//! Rust client SDK for the Quantum AI API.
//!
//! Supports text generation (with streaming), session chat, multi-agent orchestration,
//! image/video/audio generation, embeddings, RAG search, compute provisioning,
//! voice management, API key management, and model listing through a single API endpoint.
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

pub mod account;
pub mod agent;
pub mod audio;
pub mod chat;
pub mod client;
pub mod compute;
pub mod contact;
pub mod documents;
pub mod embeddings;
pub mod error;
pub mod image;
pub mod jobs;
pub mod keys;
pub mod models;
pub mod rag;
pub mod session;
pub mod video;
pub mod voices;

// Re-export primary types at crate root for convenience.
pub use client::{Client, ClientBuilder, ResponseMeta, DEFAULT_BASE_URL, TICKS_PER_USD};
pub use error::{ApiError, Error, Result};

// Chat types
pub use chat::{
    ChatMessage, ChatRequest, ChatResponse, ChatStream, ChatTool, ChatUsage, ContentBlock,
    StreamDelta, StreamEvent, StreamToolUse,
};

// Session types
pub use session::{ContextConfig, SessionChatRequest, SessionChatResponse, SessionContext, ToolResult};

// Agent types
pub use agent::{
    AgentRequest, AgentStream, AgentStreamEvent, AgentWorker, MissionRequest, MissionWorker,
};

// Image types
pub use image::{
    GeneratedImage, ImageEditRequest, ImageEditResponse, ImageRequest, ImageResponse,
};

// Video types
pub use video::{
    Avatar, AvatarsResponse, DigitalTwinRequest, GeneratedVideo, HeyGenVoice,
    HeyGenVoicesResponse, JobResponse, PhotoAvatarRequest, StudioClip, StudioVideoRequest,
    TranslateRequest, VideoRequest, VideoResponse, VideoTemplate, VideoTemplatesResponse,
};

// Audio types
pub use audio::{
    AlignRequest, AlignResponse, AlignmentSegment, AudioResponse, DialogueRequest, DialogueTurn,
    DubRequest, IsolateRequest, MusicClip, MusicRequest, MusicResponse, RemixRequest,
    SoundEffectRequest, SoundEffectResponse, SpeechToSpeechRequest, StarfishTTSRequest,
    SttRequest, SttResponse, TtsRequest, TtsResponse, VoiceDesignRequest,
};

// Account types
pub use account::{
    BalanceResponse, PricingEntry, PricingResponse, UsageEntry, UsageQuery, UsageResponse,
    UsageSummaryMonth, UsageSummaryResponse,
};

// Jobs types
pub use jobs::{JobCreateRequest, JobCreateResponse, JobStatusResponse, JobSummary, ListJobsResponse};

// Keys types
pub use keys::{CreateKeyRequest, CreateKeyResponse, KeyDetails, ListKeysResponse, StatusResponse};

// Compute types
pub use compute::{
    ComputeInstance, ComputeTemplate, DeleteResponse, InstanceResponse, InstancesResponse,
    ProvisionRequest, ProvisionResponse, SSHKeyRequest, TemplatesResponse,
};

// Voices types
pub use voices::{CloneVoiceFile, CloneVoiceResponse, Voice, VoicesResponse};

// Contact types
pub use contact::ContactRequest;

// Embeddings types
pub use embeddings::{EmbedRequest, EmbedResponse};

// Document types
pub use documents::{
    ChunkRequest, ChunkResponse, DocumentChunk, DocumentRequest, DocumentResponse, ProcessRequest,
    ProcessResponse,
};

// RAG types
pub use rag::{
    RagCorpus, RagResult, RagSearchRequest, RagSearchResponse, SurrealRagProvider,
    SurrealRagProvidersResponse, SurrealRagResult, SurrealRagSearchRequest,
    SurrealRagSearchResponse,
};

// Model types
pub use models::{ModelInfo, PricingInfo};

// Error helpers
pub use error::{is_auth_error, is_not_found_error, is_rate_limit_error};
