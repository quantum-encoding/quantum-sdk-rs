use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Request body for Vertex AI RAG search.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RagSearchRequest {
    /// Search query.
    pub query: String,

    /// Filter by corpus name or ID (fuzzy match). Omit to search all corpora.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corpus: Option<String>,

    /// Maximum number of results to return (default 10).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
}

/// Response from RAG search.
#[derive(Debug, Clone, Deserialize)]
pub struct RagSearchResponse {
    /// Matching document chunks.
    pub results: Vec<RagResult>,

    /// Original search query.
    pub query: String,

    /// Corpora that were searched.
    #[serde(default)]
    pub corpora: Option<Vec<String>>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// A single result from RAG search.
#[derive(Debug, Clone, Deserialize)]
pub struct RagResult {
    /// Source document URI.
    pub source_uri: String,

    /// Display name of the source.
    pub source_name: String,

    /// Matching text chunk.
    pub text: String,

    /// Relevance score.
    pub score: f64,

    /// Vector distance (lower is more similar).
    pub distance: f64,
}

/// Describes an available RAG corpus.
#[derive(Debug, Clone, Deserialize)]
pub struct RagCorpus {
    /// Full resource name.
    pub name: String,

    /// Human-readable name.
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Describes the corpus contents.
    pub description: String,

    /// Corpus state (e.g. "ACTIVE").
    pub state: String,
}

#[derive(Deserialize)]
struct RagCorporaResponse {
    corpora: Vec<RagCorpus>,
}

/// Request body for SurrealDB-backed RAG search.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SurrealRagSearchRequest {
    /// Search query.
    pub query: String,

    /// Filter by documentation provider (e.g. "xai", "claude", "heygen").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    /// Maximum number of results (default 10, max 50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

/// Response from SurrealDB RAG search.
#[derive(Debug, Clone, Deserialize)]
pub struct SurrealRagSearchResponse {
    /// Matching documentation chunks.
    pub results: Vec<SurrealRagResult>,

    /// Original search query.
    pub query: String,

    /// Provider filter that was applied.
    #[serde(default)]
    pub provider: Option<String>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// A single result from SurrealDB RAG search.
#[derive(Debug, Clone, Deserialize)]
pub struct SurrealRagResult {
    /// Documentation provider.
    pub provider: String,

    /// Document title.
    pub title: String,

    /// Section heading.
    pub heading: String,

    /// Original source file path.
    pub source_file: String,

    /// Matching text chunk.
    pub content: String,

    /// Cosine similarity score.
    pub score: f64,
}

/// A SurrealDB RAG provider.
#[derive(Debug, Clone, Deserialize)]
pub struct SurrealRagProvider {
    /// Provider identifier (e.g. "xai", "claude").
    pub provider: String,

    /// Number of document chunks for this provider.
    #[serde(default)]
    pub chunk_count: Option<i64>,
}

/// Response from listing SurrealDB RAG providers.
#[derive(Debug, Clone, Deserialize)]
pub struct SurrealRagProvidersResponse {
    pub providers: Vec<SurrealRagProvider>,
}

// ── xAI Collection Proxy Types ──────────────────────────────────────────────

/// A user-scoped xAI collection (proxied through quantum-ai).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// Collection ID (xAI-issued).
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,

    /// Number of documents in the collection.
    #[serde(default)]
    pub document_count: Option<u64>,

    /// Owner: user ID or "shared".
    #[serde(default)]
    pub owner: Option<String>,

    /// Backend provider (e.g. "xai").
    #[serde(default)]
    pub provider: Option<String>,

    /// ISO timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Request body for creating a collection.
#[derive(Debug, Clone, Serialize)]
pub struct CreateCollectionRequest {
    pub name: String,
}

/// A document within a collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDocument {
    pub file_id: String,
    pub name: String,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub processing_status: Option<String>,
    #[serde(default)]
    pub document_status: Option<String>,
    #[serde(default)]
    pub indexed: Option<bool>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// A search result from collection search.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectionSearchResult {
    pub content: String,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub file_id: Option<String>,
    #[serde(default)]
    pub collection_id: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Request body for collection search.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionSearchRequest {
    pub query: String,
    pub collection_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<usize>,
}

/// Upload result for a document added to a collection.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectionUploadResult {
    pub file_id: String,
    pub filename: String,
    #[serde(default)]
    pub bytes: Option<u64>,
}

// Wrapper types for API responses.

#[derive(Deserialize)]
struct CollectionsListResponse {
    collections: Vec<Collection>,
}

#[derive(Deserialize)]
struct CollectionDocumentsResponse {
    documents: Vec<CollectionDocument>,
}

#[derive(Deserialize)]
struct CollectionSearchResponse {
    results: Vec<CollectionSearchResult>,
}

#[derive(Deserialize)]
struct DeleteCollectionResponse {
    #[serde(default)]
    message: String,
}

impl Client {
    /// Searches Vertex AI RAG corpora for relevant documentation.
    pub async fn rag_search(&self, req: &RagSearchRequest) -> Result<RagSearchResponse> {
        let (mut resp, meta) = self
            .post_json::<RagSearchRequest, RagSearchResponse>("/qai/v1/rag/search", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Lists available Vertex AI RAG corpora.
    pub async fn rag_corpora(&self) -> Result<Vec<RagCorpus>> {
        let (resp, _meta) = self
            .get_json::<RagCorporaResponse>("/qai/v1/rag/corpora")
            .await?;
        Ok(resp.corpora)
    }

    /// Searches provider API documentation via SurrealDB vector search.
    pub async fn surreal_rag_search(
        &self,
        req: &SurrealRagSearchRequest,
    ) -> Result<SurrealRagSearchResponse> {
        let (mut resp, meta) = self
            .post_json::<SurrealRagSearchRequest, SurrealRagSearchResponse>(
                "/qai/v1/rag/surreal/search",
                req,
            )
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Lists available SurrealDB RAG documentation providers.
    pub async fn surreal_rag_providers(&self) -> Result<SurrealRagProvidersResponse> {
        let (resp, _meta) = self
            .get_json::<SurrealRagProvidersResponse>("/qai/v1/rag/surreal/providers")
            .await?;
        Ok(resp)
    }

    // ── xAI Collection Proxy (user-scoped) ──────────────────────────────────

    /// Lists the user's collections plus shared collections.
    pub async fn collections_list(&self) -> Result<Vec<Collection>> {
        let (resp, _meta) = self
            .get_json::<CollectionsListResponse>("/qai/v1/rag/collections")
            .await?;
        Ok(resp.collections)
    }

    /// Creates a new user-owned collection.
    pub async fn collections_create(&self, name: &str) -> Result<Collection> {
        let req = CreateCollectionRequest {
            name: name.to_string(),
        };
        let (resp, _meta) = self
            .post_json::<CreateCollectionRequest, Collection>("/qai/v1/rag/collections", &req)
            .await?;
        Ok(resp)
    }

    /// Gets details for a single collection (must be owned or shared).
    pub async fn collections_get(&self, id: &str) -> Result<Collection> {
        let (resp, _meta) = self
            .get_json::<Collection>(&format!("/qai/v1/rag/collections/{id}"))
            .await?;
        Ok(resp)
    }

    /// Deletes a collection (owner only).
    pub async fn collections_delete(&self, id: &str) -> Result<String> {
        let (resp, _meta) = self
            .delete_json::<DeleteCollectionResponse>(&format!("/qai/v1/rag/collections/{id}"))
            .await?;
        Ok(resp.message)
    }

    /// Lists documents in a collection.
    pub async fn collections_documents(&self, collection_id: &str) -> Result<Vec<CollectionDocument>> {
        let (resp, _meta) = self
            .get_json::<CollectionDocumentsResponse>(&format!(
                "/qai/v1/rag/collections/{collection_id}/documents"
            ))
            .await?;
        Ok(resp.documents)
    }

    /// Uploads a file to a collection. The server handles the two-step
    /// xAI upload (files API + management API) with the master key.
    pub async fn collections_upload(
        &self,
        collection_id: &str,
        filename: &str,
        content: Vec<u8>,
    ) -> Result<CollectionUploadResult> {
        let part = reqwest::multipart::Part::bytes(content)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| crate::error::Error::Api(crate::error::ApiError {
                status_code: 0,
                code: "multipart_error".into(),
                message: e.to_string(),
                request_id: String::new(),
            }))?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let (resp, _meta) = self
            .post_multipart::<CollectionUploadResult>(
                &format!("/qai/v1/rag/collections/{collection_id}/upload"),
                form,
            )
            .await?;
        Ok(resp)
    }

    /// Searches across collections (user's + shared) with hybrid/semantic/keyword mode.
    pub async fn collections_search(
        &self,
        req: &CollectionSearchRequest,
    ) -> Result<Vec<CollectionSearchResult>> {
        let (resp, _meta) = self
            .post_json::<CollectionSearchRequest, CollectionSearchResponse>(
                "/qai/v1/rag/search/collections",
                req,
            )
            .await?;
        Ok(resp.results)
    }
}
