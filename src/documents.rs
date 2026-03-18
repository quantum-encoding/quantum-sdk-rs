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
}
