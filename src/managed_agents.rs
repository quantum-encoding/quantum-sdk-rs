//! Anthropic Managed Agents passthrough.
//!
//! `/qai/v1/managed-agents/<rest>` is a thin reverse proxy onto Anthropic's
//! hosted Managed Agents REST surface (`agents`, `environments`, `sessions`,
//! `deployments`, `vaults`). The gateway injects the org's Anthropic
//! credentials and the beta headers, so the client never holds the provider
//! key, and egress goes through the gateway's SSRF-safe pool.
//!
//! **This passthrough is admin-only.** Managed Agents spend Anthropic credits
//! the gateway cannot meter inline, so the fail-closed billing rule restricts
//! it. End-user managed-agent work goes through the mirrored
//! [`agent_runtime`](crate::agent_runtime) surface, which is metered.
//!
//! Paths and query strings are forwarded verbatim after the prefix, and the
//! upstream shapes are Anthropic's rather than the gateway's — so the bodies
//! here are untyped. `..` segments are rejected.

use crate::agent::AgentStream;
use crate::client::Client;
use crate::error::Result;

impl Client {
    /// Sends a GET to the Managed Agents passthrough.
    ///
    /// `path` is everything after `/qai/v1/managed-agents/`, query string
    /// included (e.g. `"agents?limit=20"`). Admin-only.
    ///
    /// `GET /qai/v1/managed-agents/{path}`
    pub async fn managed_agents_get(&self, path: &str) -> Result<serde_json::Value> {
        let (resp, _meta) = self
            .get_json::<serde_json::Value>(&format!("/qai/v1/managed-agents/{path}"))
            .await?;
        Ok(resp)
    }

    /// Sends a POST to the Managed Agents passthrough. Admin-only.
    ///
    /// `POST /qai/v1/managed-agents/{path}`
    pub async fn managed_agents_post(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let (resp, _meta) = self
            .post_json::<serde_json::Value, serde_json::Value>(
                &format!("/qai/v1/managed-agents/{path}"),
                body,
            )
            .await?;
        Ok(resp)
    }

    /// Sends a DELETE to the Managed Agents passthrough. Admin-only.
    ///
    /// `DELETE /qai/v1/managed-agents/{path}`
    pub async fn managed_agents_delete(&self, path: &str) -> Result<serde_json::Value> {
        let (resp, _meta) = self
            .delete_json::<serde_json::Value>(&format!("/qai/v1/managed-agents/{path}"))
            .await?;
        Ok(resp)
    }

    /// Opens a Managed Agents SSE stream, e.g.
    /// `"sessions/sesn_123/events/stream"`.
    ///
    /// The gateway relays the upstream stream unbuffered. Admin-only.
    ///
    /// `GET /qai/v1/managed-agents/{path}`
    pub async fn managed_agents_stream(&self, path: &str) -> Result<AgentStream> {
        let (resp, _meta) = self
            .get_stream_raw(&format!("/qai/v1/managed-agents/{path}"))
            .await?;
        Ok(AgentStream::from_response(resp))
    }

    /// Opens a Managed Agents SSE stream that is started by a POST body.
    ///
    /// Admin-only.
    ///
    /// `POST /qai/v1/managed-agents/{path}`
    pub async fn managed_agents_post_stream(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<AgentStream> {
        let (resp, _meta) = self
            .post_stream_raw(&format!("/qai/v1/managed-agents/{path}"), body)
            .await?;
        Ok(AgentStream::from_response(resp))
    }
}
