//! GPU compute rentals and model deployments.
//!
//! Every write on this surface (provision, SSH key, keepalive, deploy,
//! extend) is behind per-account compute approval: an unapproved account gets
//! 403 `compute_not_approved` before anything is priced or charged.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::keys::StatusResponse;
use crate::serde_util::null_as_default as deserialize_null_as_default;

/// A compute instance template describing available GPU configurations.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ComputeTemplate {
    /// Template identifier (e.g. "a100-80gb", "h100-sxm").
    pub id: String,

    /// Human-readable name.
    #[serde(default)]
    pub name: Option<String>,

    /// What the template is for.
    #[serde(default)]
    pub description: String,

    /// `"cpu"` or `"gpu"`.
    #[serde(default)]
    pub category: String,

    /// GCE machine type.
    #[serde(default)]
    pub machine_type: String,

    /// GPU type description.
    #[serde(default)]
    pub gpu: Option<String>,

    /// Number of GPUs.
    #[serde(default)]
    pub gpu_count: Option<i32>,

    /// VRAM per GPU in GB.
    #[serde(default)]
    pub vram_gb: Option<i32>,

    /// CPU cores.
    #[serde(default)]
    pub vcpus: Option<i32>,

    /// RAM in GB.
    #[serde(default)]
    pub ram_gb: Option<i32>,

    /// Boot disk size in GB.
    #[serde(default)]
    pub disk_size_gb: i32,

    /// The on-demand rate actually billed, in USD per hour. Refreshed from
    /// live pricing when the gateway has it configured.
    #[serde(default)]
    pub hourly_usd: f64,

    /// The spot rate actually billed when provisioning with `spot: true`,
    /// in USD per hour. Zero when the template has no spot pricing.
    #[serde(default)]
    pub spot_hourly_usd: f64,

    /// The static catalogue price copied once at gateway start. Live pricing
    /// updates `hourly_usd`, not this field, so read `hourly_usd` for what a
    /// provision will charge.
    #[serde(default)]
    pub price_per_hour_usd: Option<f64>,

    /// Whether `spot: true` is accepted for this template.
    #[serde(default)]
    pub spot_allowed: bool,

    /// Whether provisioning needs the explicit `confirm` flag (see
    /// [`Client::compute_provision`]).
    #[serde(default)]
    pub requires_approval: bool,

    /// Minimum balance required to provision, in USD. Zero means one hour
    /// at the template rate.
    #[serde(default)]
    pub min_deposit_usd: f64,

    /// Typical boot time in seconds.
    #[serde(default)]
    pub boot_time_secs: i32,

    /// Available zones.
    #[serde(default)]
    pub zones: Option<Vec<String>>,
}

/// Response from listing compute templates.
#[derive(Debug, Clone, Deserialize)]
pub struct TemplatesResponse {
    /// Available compute templates.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub templates: Vec<ComputeTemplate>,
}

/// Request body for provisioning a compute instance.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ProvisionRequest {
    /// Template ID to provision.
    pub template: String,

    /// Preferred zone (e.g. "us-central1-a"). Must be one of the template's
    /// zones; defaults to its first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,

    /// Use spot/preemptible pricing. Refused with 400 when the template has
    /// no spot allowance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot: Option<bool>,

    /// Auto-teardown after N minutes of inactivity. Values at or below zero
    /// become 30; values above 1440 (24 hours) become 1440.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_teardown_minutes: Option<i32>,

    /// SSH public key for access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_public_key: Option<String>,
}

/// Response from provisioning a compute instance (`201 Created`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProvisionResponse {
    /// Instance identifier.
    #[serde(default)]
    pub instance_id: String,

    /// Status at acceptance — `"provisioning"`.
    #[serde(default)]
    pub status: String,

    /// Zone the instance was placed in.
    #[serde(default)]
    pub zone: String,

    /// GCE machine type.
    #[serde(default)]
    pub machine_type: String,

    /// GPU accelerator type.
    #[serde(default)]
    pub gpu_type: String,

    /// Hourly rate the instance bills at, in USD.
    #[serde(default)]
    pub hourly_usd: f64,

    /// Amount charged so far — the first hour, deducted before the VM
    /// exists.
    #[serde(default)]
    pub cost_usd: f64,

    /// Public IP. Always absent at acceptance; poll
    /// [`Client::compute_instance`] for it.
    #[serde(default)]
    pub external_ip: Option<String>,

    /// Expected boot time in seconds.
    #[serde(default)]
    pub estimated_boot_secs: i32,
}

/// A compute instance, as returned by [`Client::compute_instance`] and, with
/// fewer fields filled in, by [`Client::compute_instances`].
///
/// The list omits `gcp_status`, `machine_type`, `spot`, `ssh_username` and
/// `error_message`; those decode to their defaults there.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ComputeInstanceInfo {
    /// Unique instance identifier.
    #[serde(default)]
    pub instance_id: String,

    /// Template that was used.
    #[serde(default)]
    pub template: String,

    /// Current instance status ("provisioning", "running", "stopping", "terminated", "failed").
    #[serde(default)]
    pub status: String,

    /// Live GCE instance status. Empty unless the instance is running.
    #[serde(default)]
    pub gcp_status: Option<String>,

    /// GCP zone.
    #[serde(default)]
    pub zone: String,

    /// GCE machine type.
    #[serde(default)]
    pub machine_type: Option<String>,

    /// Public IP address (available once running).
    #[serde(default)]
    pub external_ip: Option<String>,

    /// GPU accelerator type.
    #[serde(default)]
    pub gpu_type: Option<String>,

    /// Number of GPUs.
    #[serde(default)]
    pub gpu_count: Option<i32>,

    /// Whether this is a spot/preemptible instance.
    #[serde(default)]
    pub spot: bool,

    /// Hourly rate in USD.
    #[serde(default)]
    pub hourly_usd: f64,

    /// Total cost so far in USD.
    #[serde(default)]
    pub cost_usd: f64,

    /// Total uptime in minutes.
    #[serde(default)]
    pub uptime_minutes: i32,

    /// Inactivity timeout in minutes.
    #[serde(default)]
    pub auto_teardown_minutes: i32,

    /// SSH username for the instance.
    #[serde(default)]
    pub ssh_username: Option<String>,

    /// ISO 8601 timestamp of last activity.
    #[serde(default)]
    pub last_active_at: Option<String>,

    /// ISO 8601 creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// ISO 8601 termination timestamp (if terminated).
    #[serde(default)]
    pub terminated_at: Option<String>,

    /// Error message if the instance failed.
    #[serde(default)]
    pub error_message: Option<String>,
}

/// Response from listing compute instances.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct InstancesResponse {
    /// The caller's instances, terminated ones included.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub instances: Vec<ComputeInstanceInfo>,
}

/// Response from deleting a compute instance.
#[derive(Debug, Clone, Deserialize)]
pub struct DeleteResponse {
    /// Status message.
    pub status: String,

    /// Instance that was deleted.
    #[serde(default)]
    pub instance_id: Option<String>,
}

/// Request body for adding an SSH key to an instance.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SSHKeyRequest {
    /// SSH public key to add. Required.
    pub public_key: String,

    /// Login user the key is installed for. Defaults to `cosmic`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

// ── Model deployments ───────────────────────────────────────────────────────

/// A tested Model Garden deploy configuration from the catalogue.
///
/// Passing [`KnownModel::id`] as [`DeployModelRequest::model`] fills in the
/// machine spec and region server-side, so a caller need not repeat them.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct KnownModel {
    /// Short catalogue id.
    #[serde(default)]
    pub id: String,

    /// Display name.
    #[serde(default)]
    pub name: String,

    /// Model publisher.
    #[serde(default)]
    pub publisher: String,

    /// Full `publishers/<x>/models/<y>@<variant>` path.
    #[serde(default)]
    pub model_path: String,

    /// Machine type the model is verified on.
    #[serde(default)]
    pub machine_type: String,

    /// Accelerator type the model is verified on.
    #[serde(default)]
    pub accelerator_type: String,

    /// Number of accelerators.
    #[serde(default)]
    pub accelerator_count: i32,

    /// Regions the configuration is known to deploy in.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub regions: Vec<String>,

    /// Serving container image override, when the model needs a specific one.
    #[serde(default)]
    pub container_image: String,

    /// VRAM the configuration provides, in GB.
    #[serde(default)]
    pub vram_gb: i32,

    /// What the model is for.
    #[serde(default)]
    pub description: String,

    /// Parameter count, as displayed (e.g. `"120B (12B active)"`).
    #[serde(default)]
    pub parameters: String,

    /// Hourly price, enriched from the live billing catalogue at response
    /// time.
    #[serde(default)]
    pub price_per_hour_usd: f64,
}

/// Response from `GET /qai/v1/compute/catalog`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ComputeCatalogResponse {
    /// Curated, tested configurations with live pricing.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub verified_models: Vec<KnownModel>,

    /// Models discovered dynamically from Model Garden. Absent when the
    /// catalogue fetch failed — the verified list is still returned.
    #[serde(default)]
    pub discovered_models: Option<Vec<serde_json::Value>>,

    /// When the dynamic catalogue was fetched, RFC3339.
    #[serde(default)]
    pub cached_at: Option<String>,

    /// When the cached dynamic catalogue goes stale, RFC3339.
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Request body for `POST /qai/v1/compute/deploy-model`.
///
/// The endpoint is two-phase: leave `confirmed` unset for a cost estimate that
/// bills nothing, then resend the same request with `confirmed: true` to
/// actually provision.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DeployModelRequest {
    /// A catalogue [`KnownModel::id`], or a full Model Garden model path.
    /// Required.
    pub model: String,

    /// Machine type. Filled in from the catalogue when `model` is a known id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_type: Option<String>,

    /// Accelerator type. Filled in from the catalogue when `model` is a known
    /// id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accelerator_type: Option<String>,

    /// Number of accelerators. Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accelerator_count: Option<i32>,

    /// Deploy region. Defaults to the catalogue's first known-good region, or
    /// `us-east1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// How long to hold the deployment. Raised to the 2-hour minimum when
    /// lower.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_hours: Option<i32>,

    /// Auto-scaling minimum. Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i32>,

    /// Auto-scaling maximum. Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i32>,

    /// Let other authenticated users call this deployment's inference
    /// endpoint. They are billed per token; the hourly cost stays with the
    /// owner.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,

    /// Set to `true` to provision. Unset returns an estimate and bills
    /// nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed: Option<bool>,
}

/// The estimate returned when a deploy request is not confirmed.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeployModelEstimate {
    /// Hourly price including margin.
    #[serde(default)]
    pub cost_per_hour_usd: f64,

    /// Total for the requested duration.
    #[serde(default)]
    pub total_estimate_usd: f64,

    /// The same total in ticks.
    #[serde(default)]
    pub total_ticks: i64,

    /// Duration the estimate covers, after the 2-hour minimum is applied.
    #[serde(default)]
    pub duration_hours: i32,

    /// Display name resolved for the model.
    #[serde(default)]
    pub model_display_name: String,

    /// Resolved full model path.
    #[serde(default)]
    pub model: String,

    /// Resolved machine type.
    #[serde(default)]
    pub machine_type: String,

    /// Resolved accelerator type.
    #[serde(default)]
    pub accelerator_type: String,

    /// Resolved accelerator count.
    #[serde(default)]
    pub accelerator_count: i32,

    /// Resolved region.
    #[serde(default)]
    pub region: String,

    /// How to proceed — `"resubmit with confirmed:true to deploy"`.
    #[serde(default)]
    pub note: String,
}

/// The acceptance returned when a deploy request is confirmed. Provisioning
/// runs asynchronously; poll [`Client::compute_deployment`] until the status
/// reaches `ready`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeployModelAccepted {
    /// The deployment to poll.
    #[serde(default)]
    pub deployment_id: String,

    /// Status at acceptance — `"provisioning"`.
    #[serde(default)]
    pub status: String,

    /// Display name resolved for the model.
    #[serde(default)]
    pub model_display_name: String,

    /// Hourly price including margin.
    #[serde(default)]
    pub cost_per_hour_usd: f64,

    /// Amount deducted up front, refunded if provisioning fails.
    #[serde(default)]
    pub total_cost_usd: f64,

    /// RFC3339 time the deployment is torn down.
    #[serde(default)]
    pub expires_at: String,

    /// Provider long-running operation backing the provision.
    #[serde(default)]
    pub operation: String,

    /// Where to poll for status.
    #[serde(default)]
    pub note: String,
}

/// A model deployment record.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModelDeployment {
    /// Deployment identifier.
    #[serde(default)]
    pub id: String,

    /// Owning user.
    #[serde(default)]
    pub user_id: String,

    /// Full model path deployed.
    #[serde(default)]
    pub model: String,

    /// Display name for the model.
    #[serde(default)]
    pub model_display_name: String,

    /// Machine type.
    #[serde(default)]
    pub machine_type: String,

    /// Accelerator type.
    #[serde(default)]
    pub accelerator_type: String,

    /// Number of accelerators.
    #[serde(default)]
    pub accelerator_count: i32,

    /// Deploy region.
    #[serde(default)]
    pub region: String,

    /// Hours the deployment was booked for.
    #[serde(default)]
    pub duration_hours: i32,

    /// Lifecycle status (`provisioning`, `deploying`, `ready`, `terminated`,
    /// `failed`).
    #[serde(default)]
    pub status: String,

    /// Provider long-running operation name.
    #[serde(default)]
    pub vertex_operation: String,

    /// Endpoint URL once the deployment is serving.
    #[serde(default)]
    pub endpoint_url: String,

    /// Provider endpoint id.
    #[serde(default)]
    pub endpoint_id: String,

    /// Provider model id on the endpoint.
    #[serde(default)]
    pub model_id: String,

    /// Failure reason, when the deployment failed.
    #[serde(default, rename = "error")]
    pub error_message: String,

    /// Hourly price including margin.
    #[serde(default)]
    pub cost_per_hour_usd: f64,

    /// Total charged in ticks.
    #[serde(default)]
    pub total_cost_ticks: i64,

    /// Margin applied over the raw hardware price, as a percentage.
    #[serde(default)]
    pub margin_pct: f64,

    /// Auto-scaling minimum.
    #[serde(default)]
    pub min_replicas: i32,

    /// Auto-scaling maximum.
    #[serde(default)]
    pub max_replicas: i32,

    /// Whether other authenticated users may run inference against it.
    #[serde(default)]
    pub public: bool,

    /// Partner the spend is attributed to, or `"direct"`.
    #[serde(default)]
    pub consumer: String,

    /// RFC3339 creation timestamp.
    #[serde(default)]
    pub created_at: String,

    /// RFC3339 time the deployment became ready.
    #[serde(default)]
    pub ready_at: Option<String>,

    /// RFC3339 teardown time.
    #[serde(default)]
    pub expires_at: String,

    /// RFC3339 time the deployment was torn down.
    #[serde(default)]
    pub terminated_at: Option<String>,
}

/// Response from `GET /qai/v1/compute/deployments`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeploymentsResponse {
    /// The caller's deployments.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub deployments: Vec<ModelDeployment>,
}

/// Request body for `POST /qai/v1/compute/deployments/{id}/extend`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ExtendDeploymentRequest {
    /// Hours to add. Values at or below zero become 1.
    pub hours: i32,
}

/// Response from `POST /qai/v1/compute/deployments/{id}/extend`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtendDeploymentResponse {
    /// The deployment that was extended.
    #[serde(default)]
    pub deployment_id: String,

    /// RFC3339 teardown time after the extension.
    #[serde(default)]
    pub new_expiry: String,

    /// Hours actually added.
    #[serde(default)]
    pub extended_hours: i32,

    /// Amount charged for the extension.
    #[serde(default)]
    pub cost_usd: f64,
}

/// Response from `DELETE /qai/v1/compute/deployments/{id}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeploymentDeleteResponse {
    /// Status after teardown — `"terminated"`.
    #[serde(default)]
    pub status: String,
}

/// The provision route, with the confirmation flag the high-cost templates
/// require.
fn provision_path(confirm: bool) -> &'static str {
    if confirm {
        "/qai/v1/compute/provision?confirm=yes"
    } else {
        "/qai/v1/compute/provision"
    }
}

impl Client {
    /// Lists available compute templates (GPU configurations and pricing).
    ///
    /// `GET /qai/v1/compute/templates`
    pub async fn compute_templates(&self) -> Result<TemplatesResponse> {
        let (resp, _meta) = self
            .get_json::<TemplatesResponse>("/qai/v1/compute/templates")
            .await?;
        Ok(resp)
    }

    /// Provisions a new GPU compute instance.
    ///
    /// Requires per-account compute approval (403 `compute_not_approved`
    /// otherwise). One hour at the template's billed rate (`hourly_usd`, or
    /// `spot_hourly_usd` with `spot: true`) is deducted before the VM
    /// exists; the balance must cover that hour, or the template's
    /// `min_deposit_usd` when it is higher (402 `insufficient_funds`).
    /// `auto_teardown_minutes` is clamped to 30..=1440.
    ///
    /// Templates flagged `requires_approval` (the largest multi-GPU
    /// machines) are refused with 400 `confirmation_required` unless
    /// `confirm` is `true`, which sends `?confirm=yes`. Pass `false` for
    /// every other template; the flag is ignored there.
    ///
    /// `POST /qai/v1/compute/provision`
    pub async fn compute_provision(
        &self,
        req: &ProvisionRequest,
        confirm: bool,
    ) -> Result<ProvisionResponse> {
        let (resp, _meta) = self
            .post_json::<ProvisionRequest, ProvisionResponse>(provision_path(confirm), req)
            .await?;
        Ok(resp)
    }

    /// Lists the caller's compute instances, terminated ones included.
    ///
    /// `GET /qai/v1/compute/instances`
    pub async fn compute_instances(&self) -> Result<InstancesResponse> {
        let (resp, _meta) = self
            .get_json::<InstancesResponse>("/qai/v1/compute/instances")
            .await?;
        Ok(resp)
    }

    /// Gets details for one compute instance, including its live GCE
    /// status and public IP when running. 404 for an unknown id, 403 for
    /// someone else's.
    ///
    /// `GET /qai/v1/compute/instance/{id}`
    pub async fn compute_instance(&self, id: &str) -> Result<ComputeInstanceInfo> {
        let path = format!("/qai/v1/compute/instance/{id}");
        let (resp, _meta) = self.get_json::<ComputeInstanceInfo>(&path).await?;
        Ok(resp)
    }

    /// Deletes (tears down) a compute instance.
    ///
    /// `DELETE /qai/v1/compute/instance/{id}`
    pub async fn compute_delete(&self, id: &str) -> Result<DeleteResponse> {
        let path = format!("/qai/v1/compute/instance/{id}");
        let (resp, _meta) = self.delete_json::<DeleteResponse>(&path).await?;
        Ok(resp)
    }

    /// Adds an SSH public key to a running compute instance (409
    /// `not_running` otherwise).
    ///
    /// `POST /qai/v1/compute/instance/{id}/ssh-key`
    pub async fn compute_ssh_key(&self, id: &str, req: &SSHKeyRequest) -> Result<StatusResponse> {
        let path = format!("/qai/v1/compute/instance/{id}/ssh-key");
        let (resp, _meta) = self
            .post_json::<SSHKeyRequest, StatusResponse>(&path, req)
            .await?;
        Ok(resp)
    }

    /// Sends a keepalive to prevent auto-teardown of a compute instance.
    /// Refused with 402 `balance_zero` when the balance is exhausted.
    ///
    /// `POST /qai/v1/compute/instance/{id}/keepalive`
    pub async fn compute_keepalive(&self, id: &str) -> Result<StatusResponse> {
        let path = format!("/qai/v1/compute/instance/{id}/keepalive");
        let (resp, _meta) = self
            .post_json::<serde_json::Value, StatusResponse>(&path, &serde_json::json!({}))
            .await?;
        Ok(resp)
    }

    // ── Model deployments ───────────────────────────────────────────────────

    /// Lists deployable models: curated configurations with live pricing, plus
    /// whatever the dynamic Model Garden catalogue reports.
    ///
    /// `GET /qai/v1/compute/catalog`
    pub async fn compute_catalog(&self) -> Result<ComputeCatalogResponse> {
        let (resp, _meta) = self
            .get_json::<ComputeCatalogResponse>("/qai/v1/compute/catalog")
            .await?;
        Ok(resp)
    }

    /// Prices a model deployment without billing for it.
    ///
    /// Sends the request with `confirmed` forced off, so the gateway answers
    /// with an estimate and the resolved machine spec. Even the estimate
    /// needs per-account compute approval: an unapproved account gets 403
    /// `compute_not_approved` before anything is priced.
    ///
    /// `POST /qai/v1/compute/deploy-model`
    pub async fn compute_deploy_model_estimate(
        &self,
        req: &DeployModelRequest,
    ) -> Result<DeployModelEstimate> {
        let mut req = req.clone();
        req.confirmed = None;
        let (resp, _meta) = self
            .post_json::<DeployModelRequest, DeployModelEstimate>(
                "/qai/v1/compute/deploy-model",
                &req,
            )
            .await?;
        Ok(resp)
    }

    /// Provisions a model deployment, deducting the full duration up front.
    ///
    /// Sends the request with `confirmed` forced on. Provisioning is
    /// asynchronous — poll [`Client::compute_deployment`] until the status is
    /// `ready`, then call it through
    /// [`Client::inference`](crate::Client::inference). A failed provision is
    /// refunded.
    ///
    /// `POST /qai/v1/compute/deploy-model`
    pub async fn compute_deploy_model(
        &self,
        req: &DeployModelRequest,
    ) -> Result<DeployModelAccepted> {
        let mut req = req.clone();
        req.confirmed = Some(true);
        let (resp, _meta) = self
            .post_json::<DeployModelRequest, DeployModelAccepted>(
                "/qai/v1/compute/deploy-model",
                &req,
            )
            .await?;
        Ok(resp)
    }

    /// Lists the caller's model deployments.
    ///
    /// `GET /qai/v1/compute/deployments`
    pub async fn compute_deployments(&self) -> Result<DeploymentsResponse> {
        let (resp, _meta) = self
            .get_json::<DeploymentsResponse>("/qai/v1/compute/deployments")
            .await?;
        Ok(resp)
    }

    /// Reads one model deployment, including its endpoint URL once ready.
    ///
    /// `GET /qai/v1/compute/deployments/{id}`
    pub async fn compute_deployment(&self, id: &str) -> Result<ModelDeployment> {
        let (resp, _meta) = self
            .get_json::<ModelDeployment>(&format!("/qai/v1/compute/deployments/{id}"))
            .await?;
        Ok(resp)
    }

    /// Extends a ready deployment's lifetime, billing for the extra hours.
    ///
    /// Requires compute approval (403 `compute_not_approved`). Only a
    /// `ready` deployment can be extended (400 `invalid_state`). `hours` at
    /// or below zero becomes 1; the extension is refused with 402
    /// `insufficient_funds` when the balance does not cover it.
    ///
    /// `POST /qai/v1/compute/deployments/{id}/extend`
    pub async fn compute_deployment_extend(
        &self,
        id: &str,
        hours: i32,
    ) -> Result<ExtendDeploymentResponse> {
        let req = ExtendDeploymentRequest { hours };
        let (resp, _meta) = self
            .post_json::<ExtendDeploymentRequest, ExtendDeploymentResponse>(
                &format!("/qai/v1/compute/deployments/{id}/extend"),
                &req,
            )
            .await?;
        Ok(resp)
    }

    /// Tears a model deployment down early. The remaining hours are not
    /// refunded.
    ///
    /// `DELETE /qai/v1/compute/deployments/{id}`
    pub async fn compute_deployment_delete(&self, id: &str) -> Result<DeploymentDeleteResponse> {
        let (resp, _meta) = self
            .delete_json::<DeploymentDeleteResponse>(&format!("/qai/v1/compute/deployments/{id}"))
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod instance_tests {
    use super::*;

    #[test]
    fn template_exposes_the_billed_rate_beside_the_catalogue_price() {
        let resp: TemplatesResponse = serde_json::from_str(
            r#"{"templates":[{"id":"h100-8x","label":"8x H100","description":"training",
                "category":"gpu","machine_type":"a3-highgpu-8g","vcpus":208,"memory_gb":1872,
                "gpu_type":"nvidia-h100-80gb","gpu_count":8,"vram_gb":640,"disk_size_gb":500,
                "hourly_usd":98.5,"spot_hourly_usd":41.2,"spot_allowed":true,"boot_time_secs":120,
                "available_zones":["us-central1-a"],"use_cases":["training"],"preinstalled":["cuda"],
                "name":"8x H100","gpu":"nvidia-h100-80gb","ram_gb":1872,"price_per_hour_usd":88.0,
                "zones":["us-central1-a"],"min_deposit_usd":200,"requires_approval":true}]}"#,
        )
        .expect("decode");
        let t = &resp.templates[0];
        assert_eq!(t.hourly_usd, 98.5);
        assert_eq!(t.spot_hourly_usd, 41.2);
        assert_eq!(t.price_per_hour_usd, Some(88.0));
        assert!(t.requires_approval);
        assert_eq!(t.min_deposit_usd, 200.0);
        assert_eq!(t.zones.as_deref(), Some(&["us-central1-a".to_string()][..]));
    }

    #[test]
    fn confirm_flag_rides_the_query_string() {
        assert_eq!(
            provision_path(true),
            "/qai/v1/compute/provision?confirm=yes"
        );
        assert_eq!(provision_path(false), "/qai/v1/compute/provision");
    }

    #[test]
    fn provision_response_decodes_the_handler_shape() {
        let resp: ProvisionResponse = serde_json::from_str(
            r#"{"instance_id":"i1","status":"provisioning","zone":"us-central1-a",
                "machine_type":"g2-standard-4","gpu_type":"nvidia-l4","hourly_usd":1.25,
                "cost_usd":1.25,"external_ip":null,"estimated_boot_secs":60}"#,
        )
        .expect("decode");
        assert_eq!(resp.instance_id, "i1");
        assert_eq!(resp.hourly_usd, 1.25);
        assert!(resp.external_ip.is_none());
        assert_eq!(resp.estimated_boot_secs, 60);
    }

    #[test]
    fn instance_list_decodes_the_handler_entries() {
        let resp: InstancesResponse = serde_json::from_str(
            r#"{"instances":[{"instance_id":"i1","template":"l4-1x","status":"running",
                "zone":"us-central1-a","external_ip":"34.1.2.3","gpu_type":"nvidia-l4",
                "gpu_count":1,"hourly_usd":1.25,"cost_usd":2.5,"uptime_minutes":95,
                "auto_teardown_minutes":30,"last_active_at":"2026-01-01T01:00:00Z",
                "created_at":"2026-01-01T00:00:00Z"}]}"#,
        )
        .expect("decode");
        let inst = &resp.instances[0];
        assert_eq!(inst.instance_id, "i1");
        assert_eq!(inst.external_ip.as_deref(), Some("34.1.2.3"));
        assert_eq!(inst.hourly_usd, 1.25);
        assert!(inst.machine_type.is_none());
        assert!(inst.terminated_at.is_none());
    }

    #[test]
    fn instance_list_decodes_empty() {
        let resp: InstancesResponse = serde_json::from_str(r#"{"instances":[]}"#).expect("decode");
        assert!(resp.instances.is_empty());
    }

    #[test]
    fn single_instance_decodes_flat() {
        let inst: ComputeInstanceInfo = serde_json::from_str(
            r#"{"instance_id":"i1","template":"l4-1x","status":"running","gcp_status":"RUNNING",
                "zone":"us-central1-a","machine_type":"g2-standard-4","external_ip":"34.1.2.3",
                "gpu_type":"nvidia-l4","gpu_count":1,"spot":false,"hourly_usd":1.25,"cost_usd":2.5,
                "uptime_minutes":95,"auto_teardown_minutes":30,"ssh_username":"cosmic",
                "last_active_at":"2026-01-01T01:00:00Z","created_at":"2026-01-01T00:00:00Z",
                "error_message":"","terminated_at":"2026-01-01T02:00:00Z"}"#,
        )
        .expect("decode");
        assert_eq!(inst.gcp_status.as_deref(), Some("RUNNING"));
        assert_eq!(inst.ssh_username.as_deref(), Some("cosmic"));
        assert_eq!(inst.terminated_at.as_deref(), Some("2026-01-01T02:00:00Z"));
    }

    #[test]
    fn ssh_key_request_sends_public_key() {
        let req = SSHKeyRequest {
            public_key: "ssh-ed25519 AAAA".into(),
            username: None,
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["public_key"], "ssh-ed25519 AAAA");
        assert!(json.get("username").is_none());
        assert!(json.get("ssh_public_key").is_none());
    }

    #[test]
    fn provision_request_clamp_fields_serialise() {
        let req = ProvisionRequest {
            template: "l4-1x".into(),
            spot: Some(true),
            auto_teardown_minutes: Some(60),
            ssh_public_key: Some("ssh-ed25519 AAAA".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["template"], "l4-1x");
        assert_eq!(json["spot"], true);
        assert_eq!(json["auto_teardown_minutes"], 60);
        assert_eq!(json["ssh_public_key"], "ssh-ed25519 AAAA");
        assert!(json.get("zone").is_none());
    }
}

#[cfg(test)]
mod deployment_tests {
    use super::*;

    #[test]
    fn a_known_model_id_carries_no_machine_spec() {
        let req = DeployModelRequest {
            model: "nemotron-3-super-120b".into(),
            duration_hours: Some(4),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["model"], "nemotron-3-super-120b");
        assert_eq!(json["duration_hours"], 4);
        assert!(json.get("machine_type").is_none());
        assert!(json.get("accelerator_type").is_none());
        assert!(json.get("confirmed").is_none());
    }

    #[test]
    fn deployment_maps_the_error_key_to_error_message() {
        let deployment: ModelDeployment = serde_json::from_str(
            r#"{"id":"d1","user_id":"u1","model":"publishers/x/models/y",
                "status":"failed","error":"quota exhausted","cost_per_hour_usd":12.5}"#,
        )
        .expect("decode");
        assert_eq!(deployment.error_message, "quota exhausted");
        assert_eq!(deployment.status, "failed");
        assert!(deployment.ready_at.is_none());
    }

    #[test]
    fn catalog_decodes_without_the_dynamic_half() {
        let catalog: ComputeCatalogResponse = serde_json::from_str(
            r#"{"verified_models":[{"id":"m1","name":"M1","machine_type":"a4-highgpu-8g",
                                    "regions":null,"price_per_hour_usd":30.0}]}"#,
        )
        .expect("decode");
        assert_eq!(catalog.verified_models.len(), 1);
        assert!(catalog.verified_models[0].regions.is_empty());
        assert!(catalog.discovered_models.is_none());
    }
}
