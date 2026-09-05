//! Rust client SDK for the Quantum AI API.
//!
//! One [`Client`] covers the whole gateway surface: chat and streaming
//! ([`chat`], [`session`], [`media_sessions`]), agents and missions ([`agent`],
//! [`agent_runtime`], [`missions`], [`cloudrun`], [`jobs`], [`batch`]),
//! media generation ([`image`], [`video`], [`avatar`], [`audio`], [`voices`],
//! [`mesh`], [`vision`]), realtime voice ([`realtime`]), retrieval
//! ([`embeddings`], [`documents`], [`rag`], [`search`], [`scraper`], [`caches`],
//! [`files`]), code and security scanning ([`scanner`], [`security`]), GPU
//! compute and model deployments ([`compute`], [`inference`]), and account
//! administration ([`account`], [`auth`], [`credits`], [`keys`], [`licenses`],
//! [`models`], [`region`], [`contact`]).
//!
//! # Example
//!
//! ```no_run
//! # async fn example() -> quantum_sdk::Result<()> {
//! let client = quantum_sdk::Client::new("your-api-key")?;
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
pub mod agent_runtime;
pub mod audio;
pub mod auth;
pub mod avatar;
pub mod batch;
pub mod caches;
pub mod chat;
pub mod client;
pub mod cloudrun;
pub mod compute;
pub mod contact;
pub mod credits;
pub mod documents;
pub mod embeddings;
pub mod error;
pub mod files;
pub mod image;
pub mod inference;
pub mod jobs;
pub mod keys;
pub mod licenses;
pub mod managed_agents;
pub mod media_sessions;
pub mod mesh;
pub mod missions;
pub mod models;
pub mod rag;
pub mod realtime;
pub mod region;
pub mod scanner;
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
    ContentBlock, EstimateResponse, StreamDelta, StreamEvent, StreamSession, StreamToolUse,
    StreamToolUseComplete, StreamToolUseInputDelta, StreamToolUseStart,
};
// Canonical stop_reason constants (`stop_reason::END_TURN`, etc.).
pub use chat::stop_reason;

// Session types
pub use session::{
    ContextConfig, ContextMetadata, SessionChatRequest, SessionChatResponse, SessionChatStream,
    SessionContext, SessionToolResult, ToolResult,
};

// Agent types
pub use agent::{
    AgentContentPart, AgentEvent, AgentMessage, AgentRequest, AgentResponse, AgentStream,
    AgentStreamEvent, AgentToolDef, AgentToolUse, AgentUsage, MissionEvent, MissionRequest,
    MissionWorker, MissionWorkerConfig,
};

// Image types
pub use image::{GeneratedImage, ImageEditRequest, ImageEditResponse, ImageRequest, ImageResponse};

// Video types
pub use video::{
    Avatar, AvatarsResponse, DigitalTwinCreateRequest, DigitalTwinCreateResponse, GeneratedVideo,
    HeyGenVoice, HeyGenVoicesResponse, JobResponse, PhotoAvatarRequest, StudioVideoRequest,
    TranslateRequest, TwinVideoRequest, VideoBatchItem, VideoBatchItemError, VideoBatchStatusQuery,
    VideoBatchStatusResponse, VideoBatchSubmitRequest, VideoBatchSubmitResponse, VideoRequest,
    VideoResponse, VideoStudioRequest, VideoSubtitlePosition, VideoTemplate, VideoTemplateDetail,
    VideoTemplateDetailResponse, VideoTemplateDimension, VideoTemplateGenerateRequest,
    VideoTemplateScene, VideoTemplateSceneVariable, VideoTemplateSubtitles, VideoTemplatesResponse,
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
    AlignRequest, AlignResponse, AlignedWord, AudioSound, AudioSoundsQuery, AudioSoundsResponse,
    DialogueRequest, DialogueResponse, DialogueTurn, DialogueVoice, DubRequest, DubResponse,
    ElevenMusicClip, ElevenMusicRequest, ElevenMusicResponse, FinetuneInfo, IsolateRequest,
    IsolateVoiceRequest, IsolateVoiceResponse, ListFinetunesResponse, MusicClip,
    MusicFinetuneCreateRequest, MusicRequest, MusicResponse, MusicSection, RemixRequest,
    RemixVoiceRequest, RemixVoiceResponse, SoundEffectRequest, SoundEffectResponse,
    SpeechToSpeechRequest, SpeechToSpeechResponse, SpeechToTextRequest, SpeechToTextResponse,
    StarfishTTSRequest, StarfishTTSResponse, SttRequest, SttResponse, TextToSpeechRequest,
    TextToSpeechResponse, TtsRequest, TtsResponse, VoiceDesignRequest, VoiceDesignResponse,
    VoicePreview,
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

// Lifetime plan types
pub use credits::{
    LifetimePlan, LifetimePlansResponse, LifetimePurchaseRequest, LifetimePurchaseResponse,
};

// Device, ephemeral and partner key types
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
    JobStatusResponse, JobStreamEvent, ListJobsResponse,
};

// Keys types
pub use keys::{CreateKeyRequest, CreateKeyResponse, KeyDetails, ListKeysResponse, StatusResponse};
pub use region::Region;

// Compute types
pub use compute::{
    ComputeInstanceInfo, ComputeTemplate, DeleteResponse, InstancesResponse, ProvisionRequest,
    ProvisionResponse, SSHKeyRequest, TemplatesResponse,
};

// Voices types
pub use voices::{
    AddVoiceFromLibraryRequest, AddVoiceFromLibraryResponse, CloneVoiceRequest, CloneVoiceResponse,
    SharedVoice, SharedVoicesResponse, Voice, VoiceInfo, VoiceLibraryQuery, VoicesResponse,
};

// 3D Mesh pipeline types
pub use mesh::{
    AnimateRequest, AnimationPostProcess, BasicAnimations, ModelUrls, PostProcess, RemeshRequest,
    RetextureRequest, RigOutput, RigRequest,
};

// Contact types
pub use contact::ContactRequest;

// Embeddings types
pub use embeddings::{EmbedRequest, EmbedResponse};

// Document types
pub use documents::{
    ChunkDocumentRequest, ChunkDocumentResponse, ChunkRequest, ChunkResponse, DocumentChunk,
    DocumentImage, DocumentMeta, DocumentRequest, DocumentResponse, ProcessDocumentRequest,
    ProcessDocumentResponse, ProcessRequest, ProcessResponse,
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
    Discussion, DiscussionResult, GoogleSearchCitation, GoogleSearchRequest, GoogleSearchResponse,
    GoogleSearchSupport, Infobox, InfoboxResult, MetaUrl, NewsResult, QueryInfo,
    SearchAnswerChoice, SearchAnswerCitation, SearchAnswerMessage, SearchAnswerRequest,
    SearchAnswerResponse, SearchContextChunk, SearchContextRequest, SearchContextResponse,
    SearchContextSource, Thumbnail, VideoResult, WebResult, WebSearchRequest, WebSearchResponse,
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
    MissionPlanUpdate, MissionRetryResponse, MissionStatusResponse, MissionTask,
};

// Security types
pub use security::{
    CodeScanFinding, CodeScanReport, SecurityAssessment, SecurityBlocklistEntry,
    SecurityBlocklistResponse, SecurityCheckResponse, SecurityFinding, SecurityReportRequest,
    SecurityReportResponse, SecurityScanCodeRequest, SecurityScanHtmlRequest, SecurityScanResponse,
    SecurityScanUrlRequest,
};

// Media session types
pub use media_sessions::{
    MediaSession, MediaSessionChatRequest, MediaSessionChatResponse, MediaSessionCreateRequest,
    MediaSessionDeleteResponse, MediaSessionListResponse, MediaSessionTurn,
};

// Multimodal file upload types
pub use files::FileUploadResponse;

// Gemini context cache types
pub use caches::{CacheCreateRequest, CacheCreateResponse, CacheDeleteResponse};

// Licence types
pub use licenses::{
    License, LicenseJwk, LicensePublicKeyResponse, LicenseRevocationsResponse, LicensesResponse,
};

// Code scanner types
pub use scanner::{
    AuditFileResult, AuditJobResponse, AuditResult, Blueprint, BlueprintFunction, BlueprintModule,
    BlueprintType, CallEdge, CodeField, CodeFunction, CodeModule, CodeType, CodebaseGraph,
    ConventionDiff, DiffRequest, DiffResult, FileStatus, ScanDeleteResponse, ScanListResponse,
    ScanRequest, ScanResult, ScanStats, ScanTypeDetail, ScanTypeListResponse, ScanTypeSummary,
    VerifyRequest, VerifyResult, VulnerabilityScanOptions, VulnerabilityScanRequest,
};

// Agent-runtime types
pub use agent_runtime::{
    AppendEventRequest, OverlayConfig, RuntimeAgent, RuntimeAgentRequest, RuntimeAgentUpdate,
    RuntimeAgentUpdateResponse, RuntimeAgentsResponse, RuntimeEnvironment,
    RuntimeEnvironmentRequest, RuntimeEnvironmentsResponse, RuntimeEvent, RuntimeEventStream,
    RuntimeOkResponse, RuntimeSession, RuntimeTool, StageWorkspaceResponse, StartSessionRequest,
};

// Sandbox-backed orchestration types
pub use cloudrun::{CloudRunEvent, CloudRunRequest, CloudRunWorker};

// Model-deployment types
pub use compute::{
    ComputeCatalogResponse, DeployModelAccepted, DeployModelEstimate, DeployModelRequest,
    DeploymentDeleteResponse, DeploymentsResponse, ExtendDeploymentRequest,
    ExtendDeploymentResponse, KnownModel, ModelDeployment,
};

// Realtime speech-to-text token + ElevenLabs proxy types
pub use audio::RealtimeSttTokenResponse;
pub use realtime::ElevenLabsProxyConfig;

// RAG collection types
pub use rag::{CollectionDetail, CollectionSearchResponse, DeleteCollectionResponse};

// Error helpers
pub use error::{is_auth_error, is_not_found_error, is_rate_limit_error};

/// The README's code blocks compile as doctests (`cargo test --doc`), so
/// an example that drifts from the API fails the build instead of the
/// reader.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_examples {}
