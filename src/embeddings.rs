use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Request body for text embeddings.
#[derive(Debug, Clone, Serialize, Default)]
pub struct EmbedRequest {
    /// Embedding model (e.g. "text-embedding-3-small", "text-embedding-3-large").
    pub model: String,

    /// Texts to embed.
    pub input: Vec<String>,
}

/// Response from text embedding.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbedResponse {
    /// Embedding vectors, one per input string.
    pub embeddings: Vec<Vec<f64>>,

    /// Model that generated the embeddings.
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

impl Client {
    /// Generates text embeddings for the given inputs.
    pub async fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse> {
        let (mut resp, meta) = self
            .post_json::<EmbedRequest, EmbedResponse>("/qai/v1/embeddings", req)
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
