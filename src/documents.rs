use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Request body for document extraction.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DocumentRequest {
    /// Base64-encoded file content.
    pub file_base64: String,

    /// Original filename (helps determine the file type).
    pub filename: String,

    /// Desired output format (e.g. "markdown", "text").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

/// Response from document extraction.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentResponse {
    /// Extracted text content.
    pub content: String,

    /// Format of the extracted content (e.g. "markdown").
    pub format: String,

    /// Provider-specific metadata about the document.
    #[serde(default)]
    pub meta: Option<HashMap<String, serde_json::Value>>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for document chunking.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ChunkRequest {
    /// Base64-encoded file content.
    pub file_base64: String,

    /// Original filename.
    pub filename: String,

    /// Maximum chunk size in tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chunk_tokens: Option<i32>,

    /// Overlap between chunks in tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlap_tokens: Option<i32>,
}

/// A single document chunk.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentChunk {
    /// Chunk index.
    pub index: i32,

    /// Chunk text content.
    pub text: String,

    /// Estimated token count.
    #[serde(default)]
    pub token_count: Option<i32>,
}

/// Response from document chunking.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkResponse {
    /// Document chunks.
    pub chunks: Vec<DocumentChunk>,

    /// Total number of chunks.
    #[serde(default)]
    pub total_chunks: Option<i32>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Request body for document processing (combined extraction + analysis).
#[derive(Debug, Clone, Serialize, Default)]
pub struct ProcessRequest {
    /// Base64-encoded file content.
    pub file_base64: String,

    /// Original filename.
    pub filename: String,

    /// Processing instructions or prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// Model to use for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Response from document processing.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessResponse {
    /// Processed content / analysis result.
    pub content: String,

    /// Model used for processing.
    #[serde(default)]
    pub model: Option<String>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

impl Client {
    /// Extracts text content from a document (PDF, image, etc.).
    pub async fn extract_document(&self, req: &DocumentRequest) -> Result<DocumentResponse> {
        let (mut resp, meta) = self
            .post_json::<DocumentRequest, DocumentResponse>("/qai/v1/documents/extract", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Splits a document into chunks suitable for embeddings or RAG.
    pub async fn chunk_document(&self, req: &ChunkRequest) -> Result<ChunkResponse> {
        let (mut resp, meta) = self
            .post_json::<ChunkRequest, ChunkResponse>("/qai/v1/documents/chunk", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Processes a document with AI (extraction + analysis in one step).
    pub async fn process_document(&self, req: &ProcessRequest) -> Result<ProcessResponse> {
        let (mut resp, meta) = self
            .post_json::<ProcessRequest, ProcessResponse>("/qai/v1/documents/process", req)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }
}
