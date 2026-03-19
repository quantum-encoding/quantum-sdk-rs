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
}
