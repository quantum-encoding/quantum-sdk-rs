use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::serde_util::null_as_default as deserialize_null_as_default;

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
pub struct SurrealRagProviderInfo {
    /// Provider identifier (e.g. "xai", "claude").
    pub provider: String,

    /// Number of document chunks for this provider.
    #[serde(default)]
    pub chunk_count: Option<i64>,
}

/// Backwards-compatible alias.
pub type SurrealRagProvider = SurrealRagProviderInfo;

/// Response from listing SurrealDB RAG providers.
#[derive(Debug, Clone, Deserialize)]
pub struct SurrealRagProvidersResponse {
    pub providers: Vec<SurrealRagProviderInfo>,
    #[serde(default)]
    pub request_id: Option<String>,
}

// ── Collection Proxy Types ──────────────────────────────────────────────────

/// A user-scoped document collection, proxied through the gateway onto the
/// upstream provider.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Collection {
    /// Collection identifier, gateway-issued.
    #[serde(default)]
    pub id: String,

    /// Owner: a user id, or `"shared"` for collections everyone can read.
    #[serde(default)]
    pub owner: String,

    /// Backend the collection lives on (e.g. `"xai"`).
    #[serde(default)]
    pub provider: String,

    /// Human-readable name.
    #[serde(default)]
    pub name: String,

    /// What the collection is for.
    #[serde(default)]
    pub description: String,

    /// The provider's own id for the collection.
    #[serde(default)]
    pub provider_collection_id: String,

    /// Number of documents indexed.
    #[serde(default)]
    pub document_count: i64,

    /// RFC3339 creation timestamp.
    #[serde(default)]
    pub created_at: String,
}

/// Request body for `POST /qai/v1/rag/collections`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CreateCollectionRequest {
    /// Human-readable name. Required.
    pub name: String,

    /// What the collection is for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Backend to create it on. Defaults to `"xai"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// A document within a collection.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectionDocument {
    /// Document identifier, gateway-issued.
    #[serde(default)]
    pub id: String,

    /// The collection the document belongs to.
    #[serde(default)]
    pub collection_id: String,

    /// The provider's own file id.
    #[serde(default)]
    pub file_id: String,

    /// Uploaded filename.
    #[serde(default)]
    pub filename: String,

    /// Indexing status (`pending`, `indexed`, `failed`).
    #[serde(default)]
    pub status: String,

    /// Number of chunks the document was split into.
    #[serde(default)]
    pub chunks: i64,

    /// RFC3339 upload timestamp.
    #[serde(default)]
    pub uploaded_at: String,
}

/// One collection with its documents — the shape
/// `GET /qai/v1/rag/collections/{id}` returns.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CollectionDetail {
    /// The collection itself.
    #[serde(default)]
    pub collection: Collection,

    /// Its documents.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub documents: Vec<CollectionDocument>,
}

/// One chunk matched by a collection search.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CollectionSearchResult {
    /// The matched chunk text.
    #[serde(default)]
    pub content: String,

    /// Relevance score; results come back highest first.
    #[serde(default)]
    pub score: f64,

    /// Name of the collection the chunk came from.
    #[serde(default)]
    pub collection: String,

    /// Id of the collection the chunk came from.
    #[serde(default)]
    pub collection_id: String,

    /// Provider document id, when the provider reported one.
    #[serde(default)]
    pub document_id: String,

    /// Source filename, when the provider reported one.
    #[serde(default)]
    pub filename: String,

    /// Whether the chunk came from a shared collection rather than the
    /// caller's own.
    #[serde(default)]
    pub is_shared: bool,
}

/// Request body for `POST /qai/v1/rag/collections/search`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CollectionSearchRequest {
    /// The search query. Required.
    pub query: String,

    /// Collections to search. Empty searches every collection the caller can
    /// read, their own and shared.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub collection_ids: Vec<String>,

    /// Maximum chunks to return across all collections. Defaults to 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chunks: Option<i64>,
}

/// Full response from `POST /qai/v1/rag/collections/search`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CollectionSearchResponse {
    /// Matched chunks, highest score first.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub results: Vec<CollectionSearchResult>,

    /// The query that was run.
    #[serde(default)]
    pub query: String,

    /// How many collections were searched.
    #[serde(default)]
    pub collections_searched: i64,

    /// Gateway request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// The document record an upload produces.
pub type CollectionUploadResult = CollectionDocument;

/// Response from `DELETE /qai/v1/rag/collections/{id}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeleteCollectionResponse {
    /// True once the collection is gone.
    #[serde(default)]
    pub deleted: bool,

    /// The collection that was deleted.
    #[serde(default)]
    pub id: String,
}

/// Response from `GET /qai/v1/rag/collections`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CollectionsListResponse {
    /// The caller's collections plus the shared ones.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub collections: Vec<Collection>,

    /// Gateway request identifier.
    #[serde(default)]
    pub request_id: String,
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

    // ── Collection proxy (user-scoped) ──────────────────────────────────────

    /// Lists the caller's collections plus the shared ones.
    ///
    /// `GET /qai/v1/rag/collections`
    pub async fn collections_list(&self) -> Result<Vec<Collection>> {
        let (resp, _meta) = self
            .get_json::<CollectionsListResponse>("/qai/v1/rag/collections")
            .await?;
        Ok(resp.collections)
    }

    /// Creates a collection owned by the caller.
    ///
    /// `POST /qai/v1/rag/collections`
    pub async fn collections_create(&self, req: &CreateCollectionRequest) -> Result<Collection> {
        let (resp, _meta) = self
            .post_json::<CreateCollectionRequest, Collection>("/qai/v1/rag/collections", req)
            .await?;
        Ok(resp)
    }

    /// Reads one collection with its documents. The collection must be owned
    /// by the caller or shared.
    ///
    /// `GET /qai/v1/rag/collections/{id}`
    pub async fn collections_get(&self, id: &str) -> Result<CollectionDetail> {
        let (resp, _meta) = self
            .get_json::<CollectionDetail>(&format!("/qai/v1/rag/collections/{id}"))
            .await?;
        Ok(resp)
    }

    /// Deletes a collection. Owner only — a shared collection cannot be
    /// deleted by a reader.
    ///
    /// `DELETE /qai/v1/rag/collections/{id}`
    pub async fn collections_delete(&self, id: &str) -> Result<DeleteCollectionResponse> {
        let (resp, _meta) = self
            .delete_json::<DeleteCollectionResponse>(&format!("/qai/v1/rag/collections/{id}"))
            .await?;
        Ok(resp)
    }

    /// Lists the documents in a collection.
    ///
    /// The gateway serves documents alongside the collection itself, so this
    /// reads the same route as [`Client::collections_get`] and returns just
    /// the documents.
    ///
    /// `GET /qai/v1/rag/collections/{id}`
    pub async fn collections_documents(
        &self,
        collection_id: &str,
    ) -> Result<Vec<CollectionDocument>> {
        Ok(self.collections_get(collection_id).await?.documents)
    }

    /// Uploads a file into a collection. The gateway performs the two-step
    /// provider upload (file store, then index into the collection) with its
    /// own credential.
    ///
    /// `POST /qai/v1/rag/collections/{id}/upload` (multipart, field `file`)
    pub async fn collections_upload(
        &self,
        collection_id: &str,
        filename: &str,
        content: Vec<u8>,
    ) -> Result<CollectionUploadResult> {
        let part = reqwest::multipart::Part::bytes(content)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| {
                crate::error::Error::Api(crate::error::ApiError {
                    status_code: 0,
                    code: "multipart_error".into(),
                    message: e.to_string(),
                    request_id: String::new(),
                })
            })?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let (resp, _meta) = self
            .post_multipart::<CollectionUploadResult>(
                &format!("/qai/v1/rag/collections/{collection_id}/upload"),
                form,
            )
            .await?;
        Ok(resp)
    }

    /// Searches across collections and returns the matched chunks, best score
    /// first.
    ///
    /// Leave [`CollectionSearchRequest::collection_ids`] empty to search
    /// everything the caller can read — their own collections and the shared
    /// ones. Use [`Client::collections_search_full`] when the surrounding
    /// metadata matters.
    ///
    /// `POST /qai/v1/rag/collections/search`
    pub async fn collections_search(
        &self,
        req: &CollectionSearchRequest,
    ) -> Result<Vec<CollectionSearchResult>> {
        Ok(self.collections_search_full(req).await?.results)
    }

    /// Searches across collections and returns the whole response, including
    /// how many collections were reached.
    ///
    /// `POST /qai/v1/rag/collections/search`
    pub async fn collections_search_full(
        &self,
        req: &CollectionSearchRequest,
    ) -> Result<CollectionSearchResponse> {
        let (resp, _meta) = self
            .post_json::<CollectionSearchRequest, CollectionSearchResponse>(
                "/qai/v1/rag/collections/search",
                req,
            )
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod collection_tests {
    use super::*;

    #[test]
    fn search_request_omits_an_empty_collection_filter() {
        let req = CollectionSearchRequest {
            query: "how does billing work".into(),
            max_chunks: Some(5),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["max_chunks"], 5);
        assert!(json.get("collection_ids").is_none());
    }

    #[test]
    fn search_results_carry_the_collection_they_came_from() {
        let resp: CollectionSearchResponse = serde_json::from_str(
            r#"{"results":[{"content":"ticks are 1e-10 USD","score":0.91,
                            "collection":"docs","collection_id":"c1",
                            "document_id":"d1","filename":"billing.md","is_shared":true}],
                "query":"billing","collections_searched":2,"request_id":"req_1"}"#,
        )
        .expect("decode");
        assert_eq!(resp.collections_searched, 2);
        assert_eq!(resp.results[0].collection, "docs");
        assert!(resp.results[0].is_shared);
    }

    #[test]
    fn collection_detail_decodes_a_null_document_list() {
        let detail: CollectionDetail = serde_json::from_str(
            r#"{"collection":{"id":"c1","owner":"u1","provider":"xai","name":"docs",
                              "provider_collection_id":"xc1","document_count":0,
                              "created_at":"2026-01-01T00:00:00Z"},
                "documents":null}"#,
        )
        .expect("decode");
        assert_eq!(detail.collection.provider_collection_id, "xc1");
        assert!(detail.documents.is_empty());
    }

    #[test]
    fn upload_result_decodes_the_document_record() {
        let doc: CollectionUploadResult = serde_json::from_str(
            r#"{"id":"d1","collection_id":"c1","file_id":"file_9","filename":"spec.pdf",
                "status":"indexed","chunks":12,"uploaded_at":"2026-01-01T00:00:00Z"}"#,
        )
        .expect("decode");
        assert_eq!(doc.filename, "spec.pdf");
        assert_eq!(doc.status, "indexed");
        assert_eq!(doc.chunks, 12);
    }
}
