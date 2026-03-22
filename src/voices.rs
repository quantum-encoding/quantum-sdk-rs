use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::keys::StatusResponse;

/// A voice available for TTS.
#[derive(Debug, Clone, Deserialize)]
pub struct Voice {
    /// Voice identifier.
    pub voice_id: String,

    /// Human-readable voice name.
    pub name: String,

    /// Provider (e.g. "elevenlabs", "openai").
    #[serde(default)]
    pub provider: Option<String>,

    /// Language/locale codes supported.
    #[serde(default)]
    pub languages: Option<Vec<String>>,

    /// Voice gender.
    #[serde(default)]
    pub gender: Option<String>,

    /// Whether this is a cloned voice.
    #[serde(default)]
    pub is_cloned: Option<bool>,

    /// Preview audio URL.
    #[serde(default)]
    pub preview_url: Option<String>,
}

/// Response from listing voices.
#[derive(Debug, Clone, Deserialize)]
pub struct VoicesResponse {
    /// Available voices.
    pub voices: Vec<Voice>,
}

/// A file to include in a voice clone request.
#[derive(Debug, Clone)]
pub struct CloneVoiceFile {
    /// Original filename (e.g. "sample.mp3").
    pub filename: String,

    /// Raw file bytes.
    pub data: Vec<u8>,

    /// MIME type (e.g. "audio/mpeg").
    pub mime_type: String,
}

/// Response from cloning a voice.
#[derive(Debug, Clone, Deserialize)]
pub struct CloneVoiceResponse {
    /// The new voice identifier.
    pub voice_id: String,

    /// The name assigned to the cloned voice.
    pub name: String,

    /// Status message.
    #[serde(default)]
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Voice Library (shared/community voices)
// ---------------------------------------------------------------------------

/// A shared voice from the voice library.
#[derive(Debug, Clone, Deserialize)]
pub struct SharedVoice {
    /// Owner's public identifier.
    pub public_owner_id: String,

    /// Voice identifier.
    pub voice_id: String,

    /// Voice display name.
    pub name: String,

    /// Voice category (e.g. "professional", "generated").
    #[serde(default)]
    pub category: Option<String>,

    /// Voice description.
    #[serde(default)]
    pub description: Option<String>,

    /// Preview audio URL.
    #[serde(default)]
    pub preview_url: Option<String>,

    /// Voice gender.
    #[serde(default)]
    pub gender: Option<String>,

    /// Perceived age range.
    #[serde(default)]
    pub age: Option<String>,

    /// Accent (e.g. "british", "american").
    #[serde(default)]
    pub accent: Option<String>,

    /// Primary language.
    #[serde(default)]
    pub language: Option<String>,

    /// Intended use case (e.g. "narration", "conversational").
    #[serde(default)]
    pub use_case: Option<String>,

    /// Average rating.
    #[serde(default)]
    pub rate: Option<f64>,

    /// Number of times this voice has been cloned.
    #[serde(default)]
    pub cloned_by_count: Option<i64>,

    /// Whether free-tier users can use this voice.
    #[serde(default)]
    pub free_users_allowed: Option<bool>,
}

/// Response from browsing the voice library.
#[derive(Debug, Clone, Deserialize)]
pub struct SharedVoicesResponse {
    /// Shared voices matching the query.
    pub voices: Vec<SharedVoice>,

    /// Cursor for pagination (pass as `cursor` in next request).
    #[serde(default)]
    pub next_cursor: Option<String>,

    /// Whether more results are available.
    #[serde(default)]
    pub has_more: bool,
}

/// Request parameters for browsing the voice library.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VoiceLibraryQuery {
    /// Search query string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Maximum number of results per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,

    /// Pagination cursor from a previous response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// Filter by gender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,

    /// Filter by language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Filter by use case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_case: Option<String>,
}

/// Response from adding a voice from the library.
#[derive(Debug, Clone, Deserialize)]
pub struct AddVoiceFromLibraryResponse {
    /// The voice ID added to the user's account.
    pub voice_id: String,
}

/// Simple percent-encoding for query parameter values.
fn encode_query_value(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{b:02X}"));
            }
        }
    }
    encoded
}

impl Client {
    /// Lists all available TTS voices (built-in and cloned).
    pub async fn list_voices(&self) -> Result<VoicesResponse> {
        let (resp, _meta) = self
            .get_json::<VoicesResponse>("/qai/v1/voices")
            .await?;
        Ok(resp)
    }

    /// Clones a voice from audio samples.
    ///
    /// Sends audio files as multipart form data along with a name for the new voice.
    pub async fn clone_voice(
        &self,
        name: &str,
        files: Vec<CloneVoiceFile>,
    ) -> Result<CloneVoiceResponse> {
        let mut form = reqwest::multipart::Form::new().text("name", name.to_string());

        for file in files {
            let part = reqwest::multipart::Part::bytes(file.data)
                .file_name(file.filename)
                .mime_str(&file.mime_type)
                .map_err(|e| crate::error::Error::Http(e.into()))?;
            form = form.part("files", part);
        }

        let (resp, _meta) = self
            .post_multipart::<CloneVoiceResponse>("/qai/v1/voices/clone", form)
            .await?;
        Ok(resp)
    }

    /// Deletes a cloned voice by its ID.
    pub async fn delete_voice(&self, id: &str) -> Result<StatusResponse> {
        let path = format!("/qai/v1/voices/{id}");
        let (resp, _meta) = self.delete_json::<StatusResponse>(&path).await?;
        Ok(resp)
    }

    /// Browses the shared voice library with optional filters.
    pub async fn voice_library(
        &self,
        query: &VoiceLibraryQuery,
    ) -> Result<SharedVoicesResponse> {
        let mut params = Vec::new();
        if let Some(ref q) = query.query {
            params.push(format!("query={}", encode_query_value(q)));
        }
        if let Some(ps) = query.page_size {
            params.push(format!("page_size={ps}"));
        }
        if let Some(ref c) = query.cursor {
            params.push(format!("cursor={}", encode_query_value(c)));
        }
        if let Some(ref g) = query.gender {
            params.push(format!("gender={}", encode_query_value(g)));
        }
        if let Some(ref l) = query.language {
            params.push(format!("language={}", encode_query_value(l)));
        }
        if let Some(ref u) = query.use_case {
            params.push(format!("use_case={}", encode_query_value(u)));
        }

        let path = if params.is_empty() {
            "/qai/v1/voices/library".to_string()
        } else {
            format!("/qai/v1/voices/library?{}", params.join("&"))
        };

        let (resp, _meta) = self
            .get_json::<SharedVoicesResponse>(&path)
            .await?;
        Ok(resp)
    }

    /// Adds a shared voice from the library to the user's account.
    pub async fn add_voice_from_library(
        &self,
        public_owner_id: &str,
        voice_id: &str,
        name: Option<&str>,
    ) -> Result<AddVoiceFromLibraryResponse> {
        let mut body = serde_json::json!({
            "public_owner_id": public_owner_id,
            "voice_id": voice_id,
        });
        if let Some(n) = name {
            body["name"] = serde_json::Value::String(n.to_string());
        }
        let (resp, _meta) = self
            .post_json::<serde_json::Value, AddVoiceFromLibraryResponse>(
                "/qai/v1/voices/library/add",
                &body,
            )
            .await?;
        Ok(resp)
    }
}
