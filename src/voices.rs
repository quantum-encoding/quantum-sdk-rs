use serde::Deserialize;

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
}
