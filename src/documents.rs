//! Document extraction and chunking.
//!
//! The gateway extracts PDF and DOCX in-process and bills mechanical compute
//! at $0.001 per MB; there is no OCR, so a scanned PDF with no text layer
//! answers 422 `extraction_failed`. Uploads are multipart (field `file`),
//! capped at 50 MB.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::{ApiError, Error, Result};
use crate::serde_util::null_as_default;

/// A document upload for extraction.
#[derive(Debug, Clone, Default)]
pub struct DocumentRequest {
    /// Raw file bytes.
    pub content: Vec<u8>,

    /// Filename, used with `mime_type` to pick the extractor.
    pub filename: String,

    /// MIME type of the file (e.g. `application/pdf`). Sniffed from the
    /// filename when omitted.
    pub mime_type: Option<String>,

    /// Also return the images embedded in a PDF, base64-encoded.
    pub extract_images: bool,
}

/// An image pulled out of a PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentImage {
    /// Image name within the document.
    pub name: String,

    /// MIME type of the encoded image.
    pub mime: String,

    /// Base64 of the complete encoded image file.
    pub data: String,
}

/// How the document was processed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMeta {
    /// Extractor that handled the file (e.g. `pdf`, `docx`).
    #[serde(default)]
    pub extraction_method: String,

    /// Images found in the document (present when images were requested).
    #[serde(default)]
    pub images_found: i32,

    /// Number of chunks produced (chunk and process routes).
    #[serde(default)]
    pub chunk_count: i32,
}

/// Response from document extraction.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentResponse {
    /// Extracted text as Markdown.
    pub markdown: String,

    /// Output format (`markdown`).
    #[serde(default)]
    pub format: String,

    /// Extraction metadata.
    #[serde(default)]
    pub meta: DocumentMeta,

    /// Embedded images, when `extract_images` was set.
    #[serde(default, deserialize_with = "null_as_default")]
    pub images: Vec<DocumentImage>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// A document upload for chunking.
#[derive(Debug, Clone, Default)]
pub struct ChunkDocumentRequest {
    /// Raw file bytes.
    pub content: Vec<u8>,

    /// Filename, used with `mime_type` to pick the extractor.
    pub filename: String,

    /// MIME type of the file. Sniffed from the filename when omitted.
    pub mime_type: Option<String>,

    /// Chunk size in characters (gateway default 2000).
    pub chunk_size: Option<u32>,

    /// Overlap between consecutive chunks in characters (gateway default 200).
    pub overlap: Option<u32>,
}

/// A single document chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// Chunk index.
    pub index: i32,

    /// Chunk text content.
    pub text: String,
}

/// Backwards-compatible alias.
pub type ChunkRequest = ChunkDocumentRequest;

/// Response from document chunking.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkDocumentResponse {
    /// Output format (`markdown`).
    #[serde(default)]
    pub format: String,

    /// Extraction metadata; `chunk_count` is the length of `chunks`.
    #[serde(default)]
    pub meta: DocumentMeta,

    /// Document chunks.
    #[serde(default, deserialize_with = "null_as_default")]
    pub chunks: Vec<DocumentChunk>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Backwards-compatible alias.
pub type ChunkResponse = ChunkDocumentResponse;

/// A document upload for the full pipeline: extraction, chunking and
/// optional image extraction in one call.
#[derive(Debug, Clone, Default)]
pub struct ProcessDocumentRequest {
    /// Raw file bytes.
    pub content: Vec<u8>,

    /// Filename, used with `mime_type` to pick the extractor.
    pub filename: String,

    /// MIME type of the file. Sniffed from the filename when omitted.
    pub mime_type: Option<String>,

    /// Also return the images embedded in a PDF, base64-encoded.
    pub extract_images: bool,

    /// Chunk size in characters (gateway default 2000).
    pub chunk_size: Option<u32>,

    /// Overlap between consecutive chunks in characters (gateway default 200).
    pub overlap: Option<u32>,
}

/// Backwards-compatible alias.
pub type ProcessRequest = ProcessDocumentRequest;

/// Response from the document pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessDocumentResponse {
    /// Extracted text as Markdown.
    pub markdown: String,

    /// Output format (`markdown`).
    #[serde(default)]
    pub format: String,

    /// Extraction metadata.
    #[serde(default)]
    pub meta: DocumentMeta,

    /// Document chunks.
    #[serde(default, deserialize_with = "null_as_default")]
    pub chunks: Vec<DocumentChunk>,

    /// Embedded images, when `extract_images` was set.
    #[serde(default, deserialize_with = "null_as_default")]
    pub images: Vec<DocumentImage>,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Backwards-compatible alias.
pub type ProcessResponse = ProcessDocumentResponse;

/// Builds the multipart form the document routes read: the file under
/// `file`, then the optional tuning fields.
fn document_form(
    content: Vec<u8>,
    filename: &str,
    mime_type: Option<&str>,
    extract_images: bool,
    chunk_size: Option<u32>,
    overlap: Option<u32>,
) -> Result<reqwest::multipart::Form> {
    let mut part = reqwest::multipart::Part::bytes(content).file_name(filename.to_string());
    if let Some(mime) = mime_type {
        part = part.mime_str(mime).map_err(|e| {
            Error::Api(ApiError {
                status_code: 0,
                code: "multipart_error".into(),
                message: format!("invalid mime type {mime:?}: {e}"),
                request_id: String::new(),
            })
        })?;
    }
    let mut form = reqwest::multipart::Form::new().part("file", part);
    if extract_images {
        form = form.text("extract_images", "true");
    }
    if let Some(size) = chunk_size {
        form = form.text("chunk_size", size.to_string());
    }
    if let Some(overlap) = overlap {
        form = form.text("overlap", overlap.to_string());
    }
    Ok(form)
}

impl Client {
    /// Extracts a PDF or DOCX to Markdown.
    ///
    /// `POST /qai/v1/documents/extract` (multipart, field `file`)
    pub async fn extract_document(&self, req: &DocumentRequest) -> Result<DocumentResponse> {
        let form = document_form(
            req.content.clone(),
            &req.filename,
            req.mime_type.as_deref(),
            req.extract_images,
            None,
            None,
        )?;
        let (mut resp, meta) = self
            .post_multipart::<DocumentResponse>("/qai/v1/documents/extract", form)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Extracts a document and splits the Markdown into overlapping chunks
    /// sized in characters, for embeddings or RAG.
    ///
    /// `POST /qai/v1/documents/chunk` (multipart, field `file`)
    pub async fn chunk_document(
        &self,
        req: &ChunkDocumentRequest,
    ) -> Result<ChunkDocumentResponse> {
        let form = document_form(
            req.content.clone(),
            &req.filename,
            req.mime_type.as_deref(),
            false,
            req.chunk_size,
            req.overlap,
        )?;
        let (mut resp, meta) = self
            .post_multipart::<ChunkDocumentResponse>("/qai/v1/documents/chunk", form)
            .await?;
        if resp.cost_ticks == 0 {
            resp.cost_ticks = meta.cost_ticks;
        }
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Runs the whole pipeline: extraction, chunking, and images when asked.
    /// No model is involved; the price is the same mechanical rate.
    ///
    /// `POST /qai/v1/documents/process` (multipart, field `file`)
    pub async fn process_document(
        &self,
        req: &ProcessDocumentRequest,
    ) -> Result<ProcessDocumentResponse> {
        let form = document_form(
            req.content.clone(),
            &req.filename,
            req.mime_type.as_deref(),
            req.extract_images,
            req.chunk_size,
            req.overlap,
        )?;
        let (mut resp, meta) = self
            .post_multipart::<ProcessDocumentResponse>("/qai/v1/documents/process", form)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_response_reads_the_gateway_shape() {
        let body = r##"{"markdown":"# Title","format":"markdown",
            "meta":{"extraction_method":"pdf","images_found":1},
            "images":[{"name":"img0","mime":"image/png","data":"AAAA"}]}"##;
        let r: DocumentResponse = serde_json::from_str(body).unwrap();
        assert_eq!(r.markdown, "# Title");
        assert_eq!(r.meta.images_found, 1);
        assert_eq!(r.images.len(), 1);

        // A chunk response with the gateway's null-for-empty lists.
        let body = r#"{"format":"markdown","meta":{"extraction_method":"docx","chunk_count":0},"chunks":null}"#;
        let r: ChunkDocumentResponse = serde_json::from_str(body).unwrap();
        assert!(r.chunks.is_empty());
    }
}
