//! 3D model generation and remeshing via Meshy.
//!
//! Models are generated and remeshed through the async job system.
//! Use [`Client::create_job`] with `job_type: "3d/generate"` or `"3d/remesh"`,
//! then poll with [`Client::poll_job`].
//!
//! This module provides typed request structs and a convenience method
//! for remesh operations.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::jobs::{JobCreateRequest, JobStatusResponse};

/// Request for a 3D remesh operation.
///
/// Submit via `client.remesh()` or via `client.create_job()` with
/// `job_type: "3d/remesh"`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RemeshRequest {
    /// ID of a completed 3D generation task (from Meshy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_task_id: Option<String>,

    /// Direct URL to a 3D model file (alternative to input_task_id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_url: Option<String>,

    /// Output formats: "glb", "fbx", "obj", "usdz", "stl", "blend".
    /// Default: ["glb", "stl"].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_formats: Option<Vec<String>>,

    /// Mesh topology: "quad" or "triangle".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology: Option<String>,

    /// Target polygon count (100–300,000). Default: 30000.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_polycount: Option<i32>,

    /// Resize height in meters (0 = no resize).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resize_height: Option<f64>,

    /// Origin placement: "bottom", "center", or "" (no change).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_at: Option<String>,

    /// If true, skip remeshing and only convert formats.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convert_format_only: Option<bool>,
}

/// URLs for each exported format in a remesh result.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModelUrls {
    #[serde(default)]
    pub glb: String,
    #[serde(default)]
    pub fbx: String,
    #[serde(default)]
    pub obj: String,
    #[serde(default)]
    pub usdz: String,
    #[serde(default)]
    pub stl: String,
    #[serde(default)]
    pub blend: String,
}

impl Client {
    /// Submit a 3D remesh job and poll until completion.
    ///
    /// Returns the job result containing `model_urls` with download links
    /// for each requested format (including STL for 3D printing).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example(client: &quantum_sdk::Client) -> quantum_sdk::Result<()> {
    /// let result = client.remesh(&quantum_sdk::RemeshRequest {
    ///     input_task_id: Some("meshy_task_abc123".into()),
    ///     target_formats: Some(vec!["glb".into(), "stl".into()]),
    ///     target_polycount: Some(10000),
    ///     ..Default::default()
    /// }).await?;
    ///
    /// if let Some(urls) = result.result {
    ///     let stl_url = urls["model_urls"]["stl"].as_str();
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remesh(&self, req: &RemeshRequest) -> Result<JobStatusResponse> {
        let params = serde_json::to_value(req)?;

        let create_resp = self
            .create_job(&JobCreateRequest {
                job_type: "3d/remesh".into(),
                params: serde_json::json!(params),
            })
            .await?;

        self.poll_job(
            &create_resp.job_id,
            std::time::Duration::from_secs(5),
            120, // 10 minutes max
        )
        .await
    }
}
