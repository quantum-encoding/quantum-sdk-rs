//! Multimodal file uploads.
//!
//! `POST /qai/v1/files` proxies a single file to Gemini's Files API and returns
//! a `file_uri` that can be attached to a subsequent chat request as a
//! `file_uri` part, pinned by a context cache
//! ([`Client::cache_create`](crate::Client::cache_create)), or used to open a
//! media session ([`Client::media_session_create`](crate::Client::media_session_create)).
//!
//! The gateway holds the provider credential, caps the body at 100 MiB, and
//! enforces a MIME allowlist: PNG / JPEG / WebP / HEIC / HEIF images, MP4 /
//! WebM / QuickTime video, MPEG / WAV / OGG / FLAC audio, and PDF.

use serde::Deserialize;

use crate::client::Client;
use crate::error::{ApiError, Error, Result};

/// Response from `POST /qai/v1/files`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FileUploadResponse {
    /// The provider resource URI to attach to later calls.
    #[serde(default)]
    pub file_uri: String,

    /// Provider resource name (`files/<id>`).
    #[serde(default)]
    pub name: String,

    /// MIME type as recorded by the provider.
    #[serde(default)]
    pub mime_type: String,

    /// Size of the stored file in bytes.
    #[serde(default)]
    pub size_bytes: i64,

    /// Duration in seconds for video uploads; zero for images, audio, and PDFs.
    #[serde(default)]
    pub duration_seconds: i64,

    /// RFC3339 expiry, when the provider reported one. Files are transient —
    /// re-upload after expiry.
    #[serde(default)]
    pub expires_at: String,
}

impl Client {
    /// Uploads one file for multimodal use and returns its `file_uri`.
    ///
    /// `mime_type` must be in the gateway's allowlist; the upload is rejected
    /// at intake otherwise.
    ///
    /// `POST /qai/v1/files` (multipart, field `file`)
    pub async fn file_upload(
        &self,
        filename: &str,
        mime_type: &str,
        content: Vec<u8>,
    ) -> Result<FileUploadResponse> {
        let part = reqwest::multipart::Part::bytes(content)
            .file_name(filename.to_string())
            .mime_str(mime_type)
            .map_err(|e| {
                Error::Api(ApiError {
                    status_code: 0,
                    code: "multipart_error".into(),
                    message: format!("invalid MIME type {mime_type:?}: {e}"),
                    request_id: String::new(),
                })
            })?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let (resp, _meta) = self
            .post_multipart::<FileUploadResponse>("/qai/v1/files", form)
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_response_tolerates_omitted_optional_fields() {
        let resp: FileUploadResponse = serde_json::from_str(
            r#"{"file_uri":"https://generativelanguage.googleapis.com/v1beta/files/abc",
                "name":"files/abc","mime_type":"application/pdf","size_bytes":4096}"#,
        )
        .expect("decode");
        assert_eq!(resp.name, "files/abc");
        assert_eq!(resp.size_bytes, 4096);
        assert_eq!(resp.duration_seconds, 0);
        assert!(resp.expires_at.is_empty());
    }
}
