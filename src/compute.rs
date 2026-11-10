//! GPU compute rentals.
//!
//! Every write on this surface (provision, SSH key, keepalive) is behind
//! per-account compute approval: an unapproved account gets 403
//! `compute_not_approved` before anything is priced or charged.

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
