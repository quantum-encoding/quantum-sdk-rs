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
}
