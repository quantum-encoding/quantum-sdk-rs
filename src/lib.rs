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
pub mod auth;
pub mod avatar;
pub mod batch;
pub mod chat;
pub mod client;
pub mod compute;
pub mod contact;
pub mod credits;
pub mod documents;
pub mod embeddings;
pub mod error;
pub mod image;
pub mod jobs;
pub mod keys;
pub mod mesh;
pub mod missions;
pub mod models;
pub mod rag;
pub mod realtime;
pub mod region;
pub mod scraper;
pub mod search;
pub mod security;
mod serde_util;
pub mod session;
pub mod video;
pub mod vision;
pub mod voices;

// Re-export primary types at crate root for convenience.
pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL, ResponseMeta, TICKS_PER_USD};
pub use error::{ApiError, Error, ErrorCode, Result};

// Chat types
pub use chat::{
    ChatMessage, ChatRequest, ChatResponse, ChatStream, ChatTool, ChatUsage, Citation,
    ContentBlock, EstimateResponse, StreamDelta, StreamEvent, StreamToolUse, StreamToolUseComplete,
    StreamToolUseInputDelta, StreamToolUseStart,
};
// Canonical stop_reason constants (`stop_reason::END_TURN`, etc.).
pub use chat::stop_reason;

// Session types
pub use session::{
    ContextConfig, ContextMetadata, SessionChatRequest, SessionChatResponse, SessionContext,
    SessionToolResult, ToolResult,
};

// Agent types
pub use agent::{
    AgentEvent, AgentRequest, AgentStream, AgentStreamEvent, AgentWorker, AgentWorkerConfig,
    MissionEvent, MissionRequest, MissionWorker, MissionWorkerConfig,
};

// Image types
pub use image::{GeneratedImage, ImageEditRequest, ImageEditResponse, ImageRequest, ImageResponse};

// Video types
pub use video::{
    Avatar, AvatarsResponse, DigitalTwinRequest, GeneratedVideo, HeyGenAvatarsResponse,
    HeyGenTemplatesResponse, HeyGenVoice, HeyGenVoicesResponse, JobResponse, PhotoAvatarRequest,
    StudioClip, StudioVideoRequest, TranslateRequest, VideoBatchItem, VideoBatchItemError,
    VideoBatchStatusQuery, VideoBatchStatusResponse, VideoBatchSubmitRequest,
    VideoBatchSubmitResponse, VideoRequest, VideoResponse, VideoStudioRequest,
    VideoSubtitlePosition, VideoTemplate, VideoTemplateDetail, VideoTemplateDetailResponse,
    VideoTemplateDimension, VideoTemplateGenerateRequest, VideoTemplateScene,
    VideoTemplateSceneVariable, VideoTemplateSubtitles, VideoTemplatesResponse,
    VideoTranslateRequest,
};

// Avatar realtime (HeyGen live broadcast) types
pub use avatar::{
    AvatarAudioInput, AvatarRealtimeCancelResponse, AvatarRealtimeCreateResponse,
    AvatarRealtimeRequest, AvatarRealtimeStatusResponse, AvatarRealtimeTextRequest,
    AvatarRealtimeTextResponse,
};

// Audio types
pub use audio::{
    AlignRequest, AlignResponse, AlignedWord, AlignmentSegment, AudioResponse, AudioSound,
    AudioSoundsQuery, AudioSoundsResponse, DialogueRequest, DialogueResponse, DialogueTurn,
    DubRequest, DubResponse, ElevenMusicClip, ElevenMusicRequest, ElevenMusicResponse,
    FinetuneInfo, IsolateRequest, IsolateVoiceRequest, IsolateVoiceResponse, ListFinetunesResponse,
    MusicAdvancedClip, MusicAdvancedRequest, MusicAdvancedResponse, MusicClip,
    MusicFinetuneCreateRequest, MusicFinetuneInfo, MusicFinetuneListResponse, MusicRequest,
    MusicResponse, MusicSection, RemixRequest, RemixVoiceRequest, RemixVoiceResponse,
    SoundEffectRequest, SoundEffectResponse, SpeechToSpeechRequest, SpeechToSpeechResponse,
    SpeechToTextRequest, SpeechToTextResponse, StarfishTTSRequest, StarfishTTSResponse, SttRequest,
    SttResponse, TextToSpeechRequest, TextToSpeechResponse, TtsRequest, TtsResponse,
    VoiceDesignRequest, VoiceDesignResponse, VoicePreview,
};

// Account types
pub use account::{
    BalanceResponse, PricingEntry, PricingResponse, UsageEntry, UsageQuery, UsageResponse,
    UsageSummaryMonth, UsageSummaryResponse,
};

// Auth types
pub use account::{AccountDeleteResponse, DeletionStatus};
pub use auth::{AuthAppleRequest, AuthResponse, AuthUser};
pub use auth::{
    AuthFirebaseRequest, AuthGoogleRequest, RevokeSessionResponse, VerifyKeyRequest,
    VerifyKeyResponse,
};
pub use credits::{
    LifetimePlan, LifetimePlansResponse, LifetimePurchaseRequest, LifetimePurchaseResponse,
};
pub use keys::{
    DeviceKey, EphemeralKeyRequest, EphemeralKeyResponse, KeyUsageDay, KeyUsageModel,
    KeyUsageResponse, ListDeviceKeysResponse, PartnerKeyRequest, PartnerKeyResponse,
    RotateKeyRequest, RotateKeyResponse,
};

// Batch types
pub use batch::{
    BatchJob, BatchJobInfo, BatchJobInput, BatchJobsResponse, BatchJsonlResponse,
    BatchSubmitRequest, BatchSubmitResponse,
};

// Credits types
pub use credits::{
    CreditBalanceResponse, CreditPack, CreditPacksResponse, CreditPurchaseRequest,
    CreditPurchaseResponse, CreditTier, CreditTiersResponse, DevProgramApplyRequest,
    DevProgramApplyResponse,
};

// Jobs types
pub use jobs::{
    JobAcceptedResponse, JobCreateRequest, JobCreateResponse, JobListEntry, JobListResponse,
    JobStatusResponse, JobStreamEvent, JobSummary, ListJobsResponse,
};

// Keys types
pub use keys::{CreateKeyRequest, CreateKeyResponse, KeyDetails, ListKeysResponse, StatusResponse};
pub use region::Region;

// Compute types
pub use compute::{
    BillingEntry, BillingRequest, BillingResponse, ComputeInstance, ComputeInstanceInfo,
    ComputeTemplate, DeleteResponse, InstanceResponse, InstancesResponse, ProvisionRequest,
    ProvisionResponse, SSHKeyRequest, TemplatesResponse,
};

// Voices types
pub use voices::{
    AddVoiceFromLibraryRequest, AddVoiceFromLibraryResponse, CloneVoiceFile, CloneVoiceRequest,
    CloneVoiceResponse, SharedVoice, SharedVoicesResponse, Voice, VoiceInfo, VoiceLibraryQuery,
    VoicesResponse,
};

// 3D Mesh pipeline types
pub use mesh::{
    AnimateRequest, AnimationPostProcess, BasicAnimations, Generate3DRequest, ModelUrls,
    PostProcess, RemeshRequest, RetextureRequest, RigRequest,
};

// Contact types
pub use contact::{ContactRequest, ContactResponse};

// Embeddings types
pub use embeddings::{EmbedRequest, EmbedResponse};

// Document types
pub use documents::{
    ChunkDocumentRequest, ChunkDocumentResponse, ChunkRequest, ChunkResponse, DocumentChunk,
    DocumentRequest, DocumentResponse, ProcessDocumentRequest, ProcessDocumentResponse,
    ProcessRequest, ProcessResponse,
};

// RAG types
pub use rag::{
    Collection, CollectionDocument, CollectionSearchRequest, CollectionSearchResult,
    CollectionUploadResult, CreateCollectionRequest, RagCorpus, RagResult, RagSearchRequest,
    RagSearchResponse, SurrealRagProvider, SurrealRagProviderInfo, SurrealRagProvidersResponse,
    SurrealRagResult, SurrealRagSearchRequest, SurrealRagSearchResponse,
};

// Scraper types
pub use scraper::{
    ScrapeRequest, ScrapeResponse, ScrapeTarget, ScreenshotJobResponse, ScreenshotRequest,
    ScreenshotResponse, ScreenshotResult, ScreenshotURL,
};

// Search types
pub use search::{
    ContextChunk, ContextOptions, Discussion, DiscussionResult, GoogleSearchCitation,
    GoogleSearchRequest, GoogleSearchResponse, GoogleSearchSupport, Infobox, InfoboxResult,
    LLMContextResponse, NewsResult, SearchAnswerChoice, SearchAnswerCitation, SearchAnswerMessage,
    SearchAnswerRequest, SearchAnswerResponse, SearchContextChunk, SearchContextRequest,
    SearchContextResponse, SearchContextSource, SearchMessage, SearchOptions, VideoResult,
    WebResult, WebSearchRequest, WebSearchResponse,
};

// Model types
pub use models::{ModelInfo, PricingInfo};

// Realtime voice types
pub use realtime::{
    RealtimeConfig, RealtimeEvent, RealtimeReceiver, RealtimeSender, RealtimeSession,
    RealtimeSessionResponse, realtime_connect_direct, realtime_connect_direct_to,
};

// Vision types
pub use vision::{
    DetectedObject, OcrResult, QualityAssessment, RelevanceCheck, TextOverlay, VisionContext,
    VisionRequest, VisionResponse,
};

// Mission types
pub use missions::{
    MissionApproveRequest, MissionChatRequest, MissionChatResponse, MissionCheckpoint,
    MissionCheckpointsResponse, MissionConfirmStructure, MissionCreateRequest,
    MissionCreateResponse, MissionDetail, MissionImportRequest, MissionListResponse,
    MissionPlanUpdate, MissionStatusResponse, MissionTask,
};

// Security types
pub use security::{
    SecurityAssessment, SecurityBlocklistEntry, SecurityBlocklistResponse, SecurityCheckResponse,
    SecurityFinding, SecurityReportRequest, SecurityReportResponse, SecurityScanHtmlRequest,
    SecurityScanResponse, SecurityScanUrlRequest,
};

// Error helpers
pub use error::{is_auth_error, is_not_found_error, is_rate_limit_error};
