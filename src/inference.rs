//! Inference against a self-deployed model.
//!
//! `POST /qai/v1/inference/{id}` proxies an OpenAI-compatible chat-completion
//! request to a Vertex endpoint stood up by
//! [`Client::compute_deploy_model`](crate::Client::compute_deploy_model). The
//! deployment must be `ready`; the caller must own it, or it must be marked
//! public. Billing is per token on top of the hourly deployment cost.
//!
//! The request and response are forwarded verbatim, so both are untyped here —
//! the shape is whatever the deployed server speaks (vLLM's OpenAI-compatible
//! surface for the catalogue models).

use crate::agent::AgentStream;
use crate::client::Client;
use crate::error::Result;

impl Client {
    /// Sends an OpenAI-compatible completion request to a deployment.
    ///
    /// `body` is forwarded as-is; it must not set `stream` — use
    /// [`Client::inference_stream`] for that.
    ///
    /// `POST /qai/v1/inference/{id}`
    pub async fn inference(
        &self,
        deployment_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let (resp, _meta) = self
            .post_json::<serde_json::Value, serde_json::Value>(
                &format!("/qai/v1/inference/{deployment_id}"),
                body,
            )
            .await?;
        Ok(resp)
    }

    /// Streams an OpenAI-compatible completion from a deployment.
    ///
    /// `stream: true` is set on the forwarded body, and the upstream SSE
    /// chunks are relayed through unchanged.
    ///
    /// `POST /qai/v1/inference/{id}`
    pub async fn inference_stream(
        &self,
        deployment_id: &str,
        body: &serde_json::Value,
    ) -> Result<AgentStream> {
        let mut body = body.clone();
        body["stream"] = serde_json::Value::Bool(true);
        let (resp, _meta) = self
            .post_stream_raw(&format!("/qai/v1/inference/{deployment_id}"), &body)
            .await?;
        Ok(AgentStream::from_response(resp))
    }
}
