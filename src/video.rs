use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// Request body for video generation.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoRequest {
    /// Video generation model (e.g. "heygen", "grok-imagine-video", "sora-2", "veo-2").
    pub model: String,

    /// Describes the video to generate.
    pub prompt: String,

    /// Target video duration in seconds (default 8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,

    /// Video aspect ratio (e.g. "16:9", "9:16").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
}

/// Response from video generation.
#[derive(Debug, Clone, Deserialize)]
pub struct VideoResponse {
    /// Generated videos.
    pub videos: Vec<GeneratedVideo>,

    /// Model that generated the videos.
    pub model: String,

    /// Total cost in ticks.
    #[serde(default)]
    pub cost_ticks: i64,

    /// Unique request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// A single generated video.
#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedVideo {
    /// Base64-encoded video data (or a URL).
    pub base64: String,

    /// Video format (e.g. "mp4").
    pub format: String,

    /// Video file size.
    pub size_bytes: i64,

    /// Video index within the batch.
    pub index: i32,
}

impl Client {
    /// Generates a video from a text prompt.
    ///
    /// Video generation is slow (30s-5min). For production use, consider submitting
    /// via the Jobs API instead.
    pub async fn generate_video(&self, req: &VideoRequest) -> Result<VideoResponse> {
        let (mut resp, meta) = self
            .post_json::<VideoRequest, VideoResponse>("/qai/v1/video/generate", req)
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
