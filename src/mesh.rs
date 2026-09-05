//! 3D model pipeline via Meshy: remesh → retexture → rig → animate.
//!
//! All operations run through the async job system. Each method submits a job
//! and polls until completion. Use the typed request structs or call
//! [`Client::create_job`] directly with the appropriate `job_type`. Text- and
//! image-to-3D generation is [`Client::generate_3d`] in the jobs module.

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

    /// Target polygon count (100–300,000). Omitted when unset; Meshy applies
    /// its own default.
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
///
/// One of `text_style_prompt` and `image_style_url` is required; the job
/// fails otherwise.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RetextureRequest {
    /// ID of a completed 3D task to retexture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_task_id: Option<String>,

    /// Direct URL to a 3D model file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_url: Option<String>,

    /// Text prompt describing the desired texture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_style_prompt: Option<String>,

    /// URL of a reference image whose style the texture follows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_style_url: Option<String>,

    /// Meshy AI model to use. Omitted when unset; Meshy applies its own
    /// default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_model: Option<String>,

    /// Keep the model's existing UV layout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_original_uv: Option<bool>,

    /// Enable PBR texture maps (metallic, roughness, normal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_pbr: Option<bool>,

    /// Strip baked lighting from the generated texture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_lighting: Option<bool>,

    /// Output formats: "glb", "fbx", "obj", "usdz", "stl", "blend".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_formats: Option<Vec<String>>,
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

    /// URL of a texture image to apply to the rigged character.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_image_url: Option<String>,
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

/// Backwards-compatible alias for [`AnimationPostProcess`].
pub type PostProcess = AnimationPostProcess;

/// URLs for the walk and run cycles Meshy bakes into every rigging result.
/// There are no idle animations; use [`Client::animate`] for anything else.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BasicAnimations {
    /// Walking animation in GLB format.
    #[serde(default)]
    pub walking_glb_url: String,

    /// Walking animation in FBX format.
    #[serde(default)]
    pub walking_fbx_url: String,

    /// Walking animation as an armature-only GLB.
    #[serde(default)]
    pub walking_armature_glb_url: String,

    /// Running animation in GLB format.
    #[serde(default)]
    pub running_glb_url: String,

    /// Running animation in FBX format.
    #[serde(default)]
    pub running_fbx_url: String,

    /// Running animation as an armature-only GLB.
    #[serde(default)]
    pub running_armature_glb_url: String,
}

/// The rigging output of a completed job. The job's `result` is an envelope
/// (`result`, `task_id`, `cost_ticks`, `request_id`); this is its `result`
/// member.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RigOutput {
    /// The rigged character in FBX format.
    #[serde(default)]
    pub rigged_character_fbx_url: String,

    /// The rigged character in GLB format.
    #[serde(default)]
    pub rigged_character_glb_url: String,

    /// Walk and run cycles, when Meshy produced them.
    #[serde(default)]
    pub basic_animations: Option<BasicAnimations>,
}

impl RigOutput {
    /// Decodes the rigging output from a finished job. `None` when the job
    /// carries no `result` (it failed or is still running).
    pub fn from_job(job: &JobStatusResponse) -> Result<Option<RigOutput>> {
        match job
            .result
            .as_ref()
            .and_then(|envelope| envelope.get("result"))
        {
            Some(output) if !output.is_null() => Ok(Some(serde_json::from_value(output.clone())?)),
            _ => Ok(None),
        }
    }
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
    /// The job's `result` is a [`RigOutput`]: rigged FBX/GLB URLs and the
    /// basic walk/run animations. Decode it with [`RigOutput::from_job`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retexture_sends_the_style_keys_meshy_reads() {
        let req = RetextureRequest {
            input_task_id: Some("task_1".into()),
            text_style_prompt: Some("weathered bronze".into()),
            enable_pbr: Some(true),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["input_task_id"], "task_1");
        assert_eq!(json["text_style_prompt"], "weathered bronze");
        assert_eq!(json["enable_pbr"], true);
        assert!(json.get("prompt").is_none());
        assert!(json.get("image_style_url").is_none());
    }

    #[test]
    fn rig_output_decodes_from_the_job_result() {
        let job: JobStatusResponse = serde_json::from_str(
            r#"{"job_id":"j1","status":"completed","type":"3d/rig",
                "result":{"result":{"rigged_character_fbx_url":"https://x/rig.fbx",
                                     "rigged_character_glb_url":"https://x/rig.glb",
                                     "basic_animations":{"walking_glb_url":"https://x/walk.glb",
                                                         "walking_fbx_url":"https://x/walk.fbx",
                                                         "walking_armature_glb_url":"https://x/walk-arm.glb",
                                                         "running_glb_url":"https://x/run.glb",
                                                         "running_fbx_url":"https://x/run.fbx",
                                                         "running_armature_glb_url":"https://x/run-arm.glb"}},
                          "task_id":"m1","cost_ticks":10,"request_id":"r1"},
                "cost_ticks":10}"#,
        )
        .expect("decode job");
        let output = RigOutput::from_job(&job)
            .expect("decode rig output")
            .expect("present");
        assert_eq!(output.rigged_character_glb_url, "https://x/rig.glb");
        let anims = output.basic_animations.expect("animations");
        assert_eq!(anims.walking_glb_url, "https://x/walk.glb");
        assert_eq!(anims.running_armature_glb_url, "https://x/run-arm.glb");
    }

    #[test]
    fn rig_output_from_job_is_none_without_a_result() {
        let job: JobStatusResponse =
            serde_json::from_str(r#"{"job_id":"j1","status":"failed","error":"x"}"#)
                .expect("decode job");
        assert!(RigOutput::from_job(&job).expect("ok").is_none());
    }
}
