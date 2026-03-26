use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::keys::StatusResponse;

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
    pub async fn compute_billing(&self, req: &BillingRequest) -> Result<BillingResponse> {
        let (resp, _meta) = self
            .post_json::<BillingRequest, BillingResponse>("/qai/v1/compute/billing", req)
            .await?;
        Ok(resp)
    }
}
