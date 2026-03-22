//! 3D model pipeline via Meshy: generate → remesh → retexture → rig → animate.
//!
//! All operations run through the async job system. Each method submits a job
//! and polls until completion. Use the typed request structs or call
//! [`Client::create_job`] directly with the appropriate `job_type`.

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

/// Request for AI retexturing of an existing 3D model.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RetextureRequest {
    /// ID of a completed 3D task to retexture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_task_id: Option<String>,

    /// Direct URL to a 3D model file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_url: Option<String>,

    /// Text prompt describing the desired texture.
    pub prompt: String,

    /// Enable PBR texture maps (metallic, roughness, normal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_pbr: Option<bool>,

    /// Meshy AI model to use (default: "meshy-6").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_model: Option<String>,
}

/// Request for auto-rigging a humanoid 3D model.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RigRequest {
    /// ID of a completed 3D task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_task_id: Option<String>,

    /// Direct URL to a 3D model file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_url: Option<String>,

    /// Height of the character in meters (for skeleton scaling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_meters: Option<f64>,
}

/// Request for applying an animation to a rigged character.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AnimateRequest {
    /// ID of a completed rigging task.
    pub rig_task_id: String,

    /// Animation action ID from Meshy's animation library.
    pub action_id: i32,

    /// Optional post-processing (e.g. FPS conversion, format conversion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_process: Option<AnimationPostProcess>,
}

/// Post-processing options for animation export.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AnimationPostProcess {
    /// Operation: "change_fps", "fbx2usdz", "extract_armature".
    pub operation_type: String,
    /// Target FPS (for "change_fps"): 24, 25, 30, 60.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<i32>,
}

// ── Convenience methods ──

impl Client {
    /// Submit a 3D remesh job and poll until completion.
    ///
    /// Returns the job result containing `model_urls` with download links
    /// for each requested format (including STL for 3D printing).
    pub async fn remesh(&self, req: &RemeshRequest) -> Result<JobStatusResponse> {
        self.submit_and_poll("3d/remesh", req).await
    }

    /// Submit a retexture job — apply new AI-generated textures to a 3D model.
    ///
    /// Returns the job result containing `model_urls` with the retextured model.
    pub async fn retexture(&self, req: &RetextureRequest) -> Result<JobStatusResponse> {
        self.submit_and_poll("3d/retexture", req).await
    }

    /// Submit a rigging job — add a humanoid skeleton to a 3D model.
    ///
    /// Returns the job result containing rigged FBX/GLB URLs and basic animations.
    pub async fn rig(&self, req: &RigRequest) -> Result<JobStatusResponse> {
        self.submit_and_poll("3d/rig", req).await
    }

    /// Submit an animation job — apply a motion to a rigged character.
    ///
    /// Returns the job result containing animated FBX/GLB URLs.
    pub async fn animate(&self, req: &AnimateRequest) -> Result<JobStatusResponse> {
        self.submit_and_poll("3d/animate", req).await
    }

    /// Internal: submit a job and poll until completion (shared by all 3D ops).
    async fn submit_and_poll(
        &self,
        job_type: &str,
        params: &impl serde::Serialize,
    ) -> Result<JobStatusResponse> {
        let params = serde_json::to_value(params)?;

        let create_resp = self
            .create_job(&JobCreateRequest {
                job_type: job_type.into(),
                params,
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
