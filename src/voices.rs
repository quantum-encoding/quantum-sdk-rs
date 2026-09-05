use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::keys::StatusResponse;
use crate::serde_util::null_as_default;

/// A voice available for TTS.
#[derive(Debug, Clone, Deserialize)]
pub struct Voice {
    /// Voice identifier.
    pub voice_id: String,

    /// Human-readable voice name.
    pub name: String,

    /// Voice category (e.g. "premade", "cloned", "professional").
    #[serde(default)]
    pub category: String,

    /// Provider (e.g. "elevenlabs", "openai", "gemini").
    #[serde(default)]
    pub provider: Option<String>,

    /// TTS model id to pass to [`Client::speak`] for this voice.
    #[serde(default)]
    pub model: Option<String>,

    /// Language/locale codes supported (not sent by the gateway today).
    #[serde(default)]
    pub languages: Option<Vec<String>>,

    /// Voice gender (not sent by the gateway today).
    #[serde(default)]
    pub gender: Option<String>,

    /// Whether this is a cloned voice.
    #[serde(default)]
    pub is_cloned: Option<bool>,

    /// Voice description.
    #[serde(default)]
    pub description: Option<String>,

    /// Preview audio URL.
    #[serde(default)]
    pub preview_url: Option<String>,
}

/// Response from listing voices.
#[derive(Debug, Clone, Deserialize)]
pub struct VoicesResponse {
    /// Available voices: built-in catalogs first, then the live ElevenLabs
    /// library (omitted when that fetch fails).
    #[serde(default, deserialize_with = "null_as_default")]
    pub voices: Vec<Voice>,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Describes an available voice with detail info.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceInfo {
    /// Voice identifier.
    pub voice_id: String,

    /// Human-readable voice name.
    pub name: String,

    /// Voice category (e.g. "premade", "cloned").
    #[serde(default)]
    pub category: String,

    /// Voice description.
    #[serde(default)]
    pub description: Option<String>,

    /// Preview audio URL.
    #[serde(default)]
    pub preview_url: Option<String>,
}

/// Request body for instant voice cloning from base64 audio samples.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CloneVoiceRequest {
    /// Display name for the cloned voice (required).
    pub name: String,

    /// Description of the voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Base64-encoded audio files for cloning (at least one).
    pub audio_samples: Vec<String>,
}

/// Response from cloning a voice.
#[derive(Debug, Clone, Deserialize)]
pub struct CloneVoiceResponse {
    /// The new voice identifier.
    pub voice_id: String,

    /// The name assigned to the cloned voice (echo of the request).
    #[serde(default)]
    pub name: String,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
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

    /// Free-text descriptive tag.
    #[serde(default)]
    pub descriptive: Option<String>,

    /// Characters synthesised with this voice across the library.
    #[serde(default)]
    pub usage_character_count: Option<i64>,

    /// Average rating.
    #[serde(default)]
    pub rate: Option<f64>,

    /// Number of times this voice has been cloned.
    #[serde(default)]
    pub cloned_by_count: Option<i64>,

    /// Whether free-tier users can use this voice.
    #[serde(default)]
    pub free_users_allowed: Option<bool>,

    /// Whether live moderation applies to this voice.
    #[serde(default)]
    pub live_moderation_enabled: Option<bool>,
}

/// Response from browsing the voice library.
#[derive(Debug, Clone, Deserialize)]
pub struct SharedVoicesResponse {
    /// Shared voices matching the query.
    #[serde(default, deserialize_with = "null_as_default")]
    pub voices: Vec<SharedVoice>,

    /// Whether more results are available.
    #[serde(default)]
    pub has_more: bool,

    /// Pagination cursor: pass as [`VoiceLibraryQuery::cursor`] to fetch
    /// the next page while `has_more` is true.
    #[serde(default)]
    pub last_sort_id: Option<String>,
}

/// Request parameters for browsing the voice library.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VoiceLibraryQuery {
    /// Search text. Wire param: `q`.
    #[serde(rename = "q", skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Maximum number of results per page (gateway default 30).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,

    /// Pagination cursor: the previous response's `last_sort_id`.
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

/// Request body for adding a shared voice from the library.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AddVoiceFromLibraryRequest {
    /// Public owner identifier.
    pub public_owner_id: String,

    /// Voice identifier in the library.
    pub voice_id: String,

    /// Optional display name (defaults to the library name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Response from adding a voice from the library.
#[derive(Debug, Clone, Deserialize)]
pub struct AddVoiceFromLibraryResponse {
    /// The voice ID added to the user's account.
    pub voice_id: String,

    /// Always "added".
    #[serde(default)]
    pub status: String,
}

/// Percent-encodes a query parameter value using the urlencoding crate.
fn encode_query_value(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

/// Builds the query string for the voice library route.
fn voice_library_params(query: &VoiceLibraryQuery) -> Vec<String> {
    let mut params = Vec::new();
    if let Some(ref q) = query.query {
        params.push(format!("q={}", encode_query_value(q)));
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
    params
}

impl Client {
    /// Lists all available TTS voices (built-in and cloned).
    pub async fn list_voices(&self) -> Result<VoicesResponse> {
        let (resp, _meta) = self.get_json::<VoicesResponse>("/qai/v1/voices").await?;
        Ok(resp)
    }

    /// Clones a voice from base64 audio samples (ElevenLabs instant clone;
    /// flat charge per clone, 402 preflight when under-funded).
    pub async fn clone_voice(&self, req: &CloneVoiceRequest) -> Result<CloneVoiceResponse> {
        let (mut resp, meta) = self
            .post_json::<CloneVoiceRequest, CloneVoiceResponse>("/qai/v1/voices/clone", req)
            .await?;
        if resp.request_id.is_empty() {
            resp.request_id = meta.request_id;
        }
        Ok(resp)
    }

    /// Deletes a cloned voice by its ID (403 unless the caller owns it,
    /// 404 when unknown upstream).
    pub async fn delete_voice(&self, id: &str) -> Result<StatusResponse> {
        let path = format!("/qai/v1/voices/{id}");
        let (resp, _meta) = self.delete_json::<StatusResponse>(&path).await?;
        Ok(resp)
    }

    /// Browses the shared voice library with optional filters. Page
    /// through with `cursor = last_sort_id` while `has_more` is true.
    pub async fn voice_library(&self, query: &VoiceLibraryQuery) -> Result<SharedVoicesResponse> {
        let params = voice_library_params(query);
        let path = if params.is_empty() {
            "/qai/v1/voices/library".to_string()
        } else {
            format!("/qai/v1/voices/library?{}", params.join("&"))
        };

        let (resp, _meta) = self.get_json::<SharedVoicesResponse>(&path).await?;
        Ok(resp)
    }

    /// Adds a shared voice from the library to the user's account.
    pub async fn add_voice_from_library(
        &self,
        public_owner_id: &str,
        voice_id: &str,
        name: Option<&str>,
    ) -> Result<AddVoiceFromLibraryResponse> {
        let body = AddVoiceFromLibraryRequest {
            public_owner_id: public_owner_id.to_owned(),
            voice_id: voice_id.to_owned(),
            name: name.map(str::to_owned),
        };
        let (resp, _meta) = self
            .post_json::<AddVoiceFromLibraryRequest, AddVoiceFromLibraryResponse>(
                "/qai/v1/voices/library/add",
                &body,
            )
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_request_is_the_json_shape_the_handler_decodes() {
        let json = serde_json::to_value(CloneVoiceRequest {
            name: "Me".into(),
            description: None,
            audio_samples: vec!["AQID".into()],
        })
        .unwrap();
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["audio_samples", "name"]);

        let resp: CloneVoiceResponse =
            serde_json::from_str(r#"{"voice_id":"v_new","name":"Me","request_id":"req_c"}"#)
                .unwrap();
        assert_eq!(resp.voice_id, "v_new");
        assert_eq!(resp.request_id, "req_c");
    }

    #[test]
    fn library_query_sends_q_and_reads_last_sort_id() {
        let params = voice_library_params(&VoiceLibraryQuery {
            query: Some("deep narrator".into()),
            cursor: Some("abc".into()),
            ..Default::default()
        });
        assert_eq!(params, ["q=deep%20narrator", "cursor=abc"]);

        let resp: SharedVoicesResponse = serde_json::from_str(
            r#"{"voices":[{"public_owner_id":"o1","voice_id":"v1","name":"N","category":"professional",
                "preview_url":"https://x/p.mp3","usage_character_count":10,"cloned_by_count":2,
                "rate":4.5,"free_users_allowed":true,"live_moderation_enabled":false}],
                "has_more":true,"last_sort_id":"cursor_2"}"#,
        )
        .unwrap();
        assert_eq!(resp.last_sort_id.as_deref(), Some("cursor_2"));
        assert_eq!(resp.voices[0].usage_character_count, Some(10));

        let empty: SharedVoicesResponse =
            serde_json::from_str(r#"{"voices":null,"has_more":false,"last_sort_id":""}"#).unwrap();
        assert!(empty.voices.is_empty());
    }

    #[test]
    fn voices_and_library_add_decode_handler_shapes() {
        let list: VoicesResponse = serde_json::from_str(
            r#"{"voices":[{"voice_id":"alloy","name":"alloy","category":"premade","provider":"openai",
                "model":"openai-tts-1","is_cloned":false}],"request_id":"req_l"}"#,
        )
        .unwrap();
        assert_eq!(list.voices[0].model.as_deref(), Some("openai-tts-1"));
        assert_eq!(list.voices[0].category, "premade");

        let added: AddVoiceFromLibraryResponse =
            serde_json::from_str(r#"{"voice_id":"v9","status":"added"}"#).unwrap();
        assert_eq!(added.status, "added");
    }
}
