use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::keys::StatusResponse;
use crate::serde_util::null_as_default as deserialize_null_as_default;

/// A compute instance template describing available GPU configurations.
#[derive(Debug, Clone, Deserialize)]
pub struct ComputeTemplate {
    /// Template identifier (e.g. "a100-80gb", "h100-sxm").
    pub id: String,

    /// Human-readable name.
    #[serde(default)]
    pub name: Option<String>,

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

    /// Price per hour in USD.
    #[serde(default)]
    pub price_per_hour_usd: Option<f64>,

    /// Available zones.
    #[serde(default)]
    pub zones: Option<Vec<String>>,
}

/// Response from listing compute templates.
#[derive(Debug, Clone, Deserialize)]
pub struct TemplatesResponse {
    /// Available compute templates.
    pub templates: Vec<ComputeTemplate>,
}

/// Request body for provisioning a compute instance.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ProvisionRequest {
    /// Template ID to provision.
    pub template: String,

    /// Preferred zone (e.g. "us-central1-a").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,

    /// Use spot/preemptible pricing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot: Option<bool>,

    /// Auto-teardown after N minutes of inactivity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_teardown_minutes: Option<i32>,

    /// SSH public key for access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_public_key: Option<String>,
}

/// Response from provisioning a compute instance.
#[derive(Debug, Clone, Deserialize)]
pub struct ProvisionResponse {
    /// Instance identifier.
    pub instance_id: String,

    /// Current instance status.
    pub status: String,

    /// Template that was provisioned.
    #[serde(default)]
    pub template: Option<String>,

    /// Zone the instance was placed in.
    #[serde(default)]
    pub zone: Option<String>,

    /// SSH connection address.
    #[serde(default)]
    pub ssh_address: Option<String>,

    /// Estimated price per hour.
    #[serde(default)]
    pub price_per_hour_usd: Option<f64>,
}

/// A running compute instance.
#[derive(Debug, Clone, Deserialize)]
pub struct ComputeInstance {
    /// Instance identifier.
    pub id: String,

    /// Current status (e.g. "running", "provisioning", "stopped").
    pub status: String,

    /// Template used.
    #[serde(default)]
    pub template: Option<String>,

    /// Zone.
    #[serde(default)]
    pub zone: Option<String>,

    /// SSH connection address.
    #[serde(default)]
    pub ssh_address: Option<String>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Price per hour.
    #[serde(default)]
    pub price_per_hour_usd: Option<f64>,

    /// Auto-teardown setting in minutes.
    #[serde(default)]
    pub auto_teardown_minutes: Option<i32>,
}

/// Detailed compute instance info with GPU, cost, and uptime details.
#[derive(Debug, Clone, Deserialize)]
pub struct ComputeInstanceInfo {
    /// Unique instance identifier.
    pub instance_id: String,

    /// Template that was used.
    pub template: String,

    /// Current instance status ("provisioning", "running", "stopping", "terminated", "failed").
    pub status: String,

    /// Live GCE instance status.
    #[serde(default)]
    pub gcp_status: Option<String>,

    /// GCP zone.
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
#[derive(Debug, Clone, Deserialize)]
pub struct InstancesResponse {
    /// Running compute instances.
    pub instances: Vec<ComputeInstance>,
}

/// Response from getting a single compute instance.
#[derive(Debug, Clone, Deserialize)]
pub struct InstanceResponse {
    /// The compute instance details.
    pub instance: ComputeInstance,
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
    /// SSH public key to add.
    pub ssh_public_key: String,
}

/// Request for querying compute billing from BigQuery.
#[derive(Debug, Clone, Serialize, Default)]
pub struct BillingRequest {
    /// Filter by instance ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,

    /// Start date for billing period (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,

    /// End date for billing period (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
}

/// A single billing line item from BigQuery.
#[derive(Debug, Clone, Deserialize)]
pub struct BillingEntry {
    /// Instance identifier.
    pub instance_id: String,

    /// Instance name.
    #[serde(default)]
    pub instance_name: Option<String>,

    /// Total cost in USD.
    pub cost_usd: f64,

    /// Usage duration in hours.
    #[serde(default)]
    pub usage_hours: Option<f64>,

    /// SKU description (e.g. "N1 Predefined Instance Core").
    #[serde(default)]
    pub sku_description: Option<String>,

    /// Billing period start.
    #[serde(default)]
    pub start_time: Option<String>,

    /// Billing period end.
    #[serde(default)]
    pub end_time: Option<String>,
}

/// Response from billing query.
#[derive(Debug, Clone, Deserialize)]
pub struct BillingResponse {
    /// Individual billing entries.
    pub entries: Vec<BillingEntry>,

    /// Total cost across all entries.
    pub total_cost_usd: f64,
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

impl Client {
    /// Lists available compute templates (GPU configurations and pricing).
    pub async fn compute_templates(&self) -> Result<TemplatesResponse> {
        let (resp, _meta) = self
            .get_json::<TemplatesResponse>("/qai/v1/compute/templates")
            .await?;
        Ok(resp)
    }

    /// Provisions a new GPU compute instance.
    pub async fn compute_provision(&self, req: &ProvisionRequest) -> Result<ProvisionResponse> {
        let (resp, _meta) = self
            .post_json::<ProvisionRequest, ProvisionResponse>("/qai/v1/compute/provision", req)
            .await?;
        Ok(resp)
    }

    /// Lists all compute instances for the account.
    pub async fn compute_instances(&self) -> Result<InstancesResponse> {
        let (resp, _meta) = self
            .get_json::<InstancesResponse>("/qai/v1/compute/instances")
            .await?;
        Ok(resp)
    }

    /// Gets details for a specific compute instance.
    pub async fn compute_instance(&self, id: &str) -> Result<InstanceResponse> {
        let path = format!("/qai/v1/compute/instance/{id}");
        let (resp, _meta) = self.get_json::<InstanceResponse>(&path).await?;
        Ok(resp)
    }

    /// Deletes (tears down) a compute instance.
    pub async fn compute_delete(&self, id: &str) -> Result<DeleteResponse> {
        let path = format!("/qai/v1/compute/instance/{id}");
        let (resp, _meta) = self.delete_json::<DeleteResponse>(&path).await?;
        Ok(resp)
    }

    /// Adds an SSH public key to a running compute instance.
    pub async fn compute_ssh_key(&self, id: &str, req: &SSHKeyRequest) -> Result<StatusResponse> {
        let path = format!("/qai/v1/compute/instance/{id}/ssh-key");
        let (resp, _meta) = self
            .post_json::<SSHKeyRequest, StatusResponse>(&path, req)
            .await?;
        Ok(resp)
    }

    /// Sends a keepalive to prevent auto-teardown of a compute instance.
    pub async fn compute_keepalive(&self, id: &str) -> Result<StatusResponse> {
        let path = format!("/qai/v1/compute/instance/{id}/keepalive");
        let (resp, _meta) = self
            .post_json::<serde_json::Value, StatusResponse>(&path, &serde_json::json!({}))
            .await?;
        Ok(resp)
    }

    /// Queries compute billing from BigQuery via the QAI backend.
    ///
    /// The gateway does not serve `/qai/v1/compute/billing`; this call
    /// returns a 404. Read spend from
    /// [`Client::account_usage`](crate::Client::account_usage) instead.
    #[deprecated(
        since = "0.8.2",
        note = "the gateway retired /qai/v1/compute/billing; use account_usage instead"
    )]
    pub async fn compute_billing(&self, req: &BillingRequest) -> Result<BillingResponse> {
        let (resp, _meta) = self
            .post_json::<BillingRequest, BillingResponse>("/qai/v1/compute/billing", req)
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
    /// with an estimate and the resolved machine spec. Deploying needs
    /// per-account approval, which is refused before any spend.
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
    /// Only a `ready` deployment can be extended.
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
