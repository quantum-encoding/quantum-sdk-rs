//! Agent runtime — the gateway's own agent / environment / session surface.
//!
//! An [`RuntimeAgent`] is a reusable config (model, system prompt, tools). An
//! [`RuntimeEnvironment`] binds it to a backend: `coding-session` runs in the
//! gateway's metered container, `managed-agents` projects onto Anthropic's
//! hosted runtime and is admin-only. Starting a session returns a
//! [`RuntimeSession`] descriptor which the client then holds and passes back on
//! every session call — the event, stream, stop, and workspace routes are
//! stateless with respect to the server.
//!
//! Agent and environment records are free to create and edit; spend starts at
//! session start.

use futures_util::{Stream, StreamExt};
use reqwest::header::{AUTHORIZATION, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::agent::sse_data_payloads;
use crate::client::Client;
use crate::error::{ApiError, Error, Result};
use crate::serde_util::null_as_default;

// ── Agents ──────────────────────────────────────────────────────────────────

/// A tool reference on a runtime agent. `type` is the provider tool type
/// (e.g. `bash_20250124`); each backend maps it onto its own capability.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeTool {
    /// Provider tool type.
    #[serde(default)]
    pub r#type: String,

    /// Name the tool is exposed under.
    #[serde(default)]
    pub name: String,
}

/// Request body for creating or updating a runtime agent.
///
/// On update, an omitted `name` or `model` carries the stored value forward.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RuntimeAgentRequest {
    /// Display name for the agent.
    pub name: String,

    /// Model the agent runs on.
    pub model: String,

    /// System prompt.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub system_prompt: String,

    /// Tools the agent may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<RuntimeTool>>,
}

/// A stored runtime agent.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimeAgent {
    /// Agent identifier.
    #[serde(default)]
    pub id: String,

    /// Owning user.
    #[serde(default)]
    pub user_id: String,

    /// Display name.
    #[serde(default)]
    pub name: String,

    /// Model the agent runs on.
    #[serde(default)]
    pub model: String,

    /// System prompt.
    #[serde(default)]
    pub system_prompt: String,

    /// Tools the agent may call.
    #[serde(default, deserialize_with = "null_as_default")]
    pub tools: Vec<RuntimeTool>,

    /// Config version, bumped on every update.
    #[serde(default)]
    pub version: i64,

    /// RFC3339 creation timestamp.
    #[serde(default)]
    pub created_at: String,

    /// RFC3339 timestamp of the last update.
    #[serde(default)]
    pub updated_at: String,

    /// Backend-side id once the agent has been projected onto a backend.
    /// Empty until first instantiated.
    #[serde(default)]
    pub upstream_id: String,
}

/// Response from `GET /qai/v1/agent-runtime/agents`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimeAgentsResponse {
    /// The caller's agents.
    #[serde(default, deserialize_with = "null_as_default")]
    pub agents: Vec<RuntimeAgent>,
}

/// Response from `PUT /qai/v1/agent-runtime/agents/{id}` — the update returns
/// the new config version rather than the whole record.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimeAgentUpdateResponse {
    /// The agent that was updated.
    #[serde(default)]
    pub id: String,

    /// The config version after the update.
    #[serde(default)]
    pub version: i64,
}

// ── Environments ────────────────────────────────────────────────────────────

/// The git contract a `coding-session` environment runs under: which repo to
/// check out at which ref, the path the agent may write, and how its diff is
/// published. Exactly one of `core_repo` and `workspace_object` is required.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverlayConfig {
    /// Repository to check out.
    #[serde(default)]
    pub core_repo: String,

    /// Ref the checkout is pinned to.
    #[serde(default)]
    pub core_pinned_ref: String,

    /// Path within the checkout the agent may write.
    #[serde(default)]
    pub overlay_path: String,

    /// Prefix for the per-session branch.
    #[serde(default)]
    pub branch_prefix: String,

    /// Push the per-session branch to origin after each snapshot, so the diff
    /// survives the gateway workdir. Needs a git credential for the repo; push
    /// failures are reported, never fatal.
    #[serde(default)]
    pub push_branch: bool,

    /// Seed the session from an uploaded archive instead of a git clone — the
    /// object returned by
    /// [`Client::agent_runtime_stage_workspace`]. Makes `push_branch`
    /// meaningless (there is no origin).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workspace_object: String,
}

/// Request body for `POST /qai/v1/agent-runtime/environments`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RuntimeEnvironmentRequest {
    /// Display name for the environment.
    pub name: String,

    /// Backend: `"coding-session"` or `"managed-agents"`. Required.
    pub backend: String,

    /// Coding-session lifecycle: single-shot by default, or a long-lived
    /// multi-turn workspace. Ignored by the managed-agents backend.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    /// Coding-session container size (`s`, `m`, `l`). Ignored by the
    /// managed-agents backend; empty means medium.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,

    /// Container image override. Both backends have defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Stored credential references the session mounts (e.g. a git push
    /// token).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault_ids: Option<Vec<String>>,

    /// The coding-session git contract. Omit for managed-agents environments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay: Option<OverlayConfig>,
}

/// A stored runtime environment.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimeEnvironment {
    /// Environment identifier.
    #[serde(default)]
    pub id: String,

    /// Owning user.
    #[serde(default)]
    pub user_id: String,

    /// Display name.
    #[serde(default)]
    pub name: String,

    /// Backend this environment runs on.
    #[serde(default)]
    pub backend: String,

    /// Coding-session lifecycle mode.
    #[serde(default)]
    pub mode: String,

    /// Coding-session container size.
    #[serde(default)]
    pub tier: String,

    /// Container image.
    #[serde(default)]
    pub image: String,

    /// Stored credential references.
    #[serde(default, deserialize_with = "null_as_default")]
    pub vault_ids: Vec<String>,

    /// The coding-session git contract, when there is one.
    #[serde(default)]
    pub overlay: Option<OverlayConfig>,

    /// Backend-side environment id once provisioned.
    #[serde(default)]
    pub upstream_id: String,

    /// RFC3339 creation timestamp.
    #[serde(default)]
    pub created_at: String,
}

/// Response from `GET /qai/v1/agent-runtime/environments`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimeEnvironmentsResponse {
    /// The caller's environments.
    #[serde(default, deserialize_with = "null_as_default")]
    pub environments: Vec<RuntimeEnvironment>,
}

// ── Sessions ────────────────────────────────────────────────────────────────

/// Request body for `POST /qai/v1/agent-runtime/sessions`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct StartSessionRequest {
    /// The agent to run.
    pub agent_id: String,

    /// The environment to run it in.
    pub environment_id: String,
}

/// A running session. The client holds this descriptor and passes it back on
/// every subsequent session call — the gateway keeps no per-connection state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeSession {
    /// Session identifier.
    #[serde(default)]
    pub id: String,

    /// Owning user.
    #[serde(default)]
    pub user_id: String,

    /// The agent being run.
    #[serde(default)]
    pub agent_id: String,

    /// The environment it runs in.
    #[serde(default)]
    pub environment_id: String,

    /// Backend the session runs on.
    #[serde(default)]
    pub backend: String,

    /// Session status.
    #[serde(default)]
    pub status: String,

    /// Backend-side session id. Required for every session call.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub upstream_id: String,

    /// RFC3339 creation timestamp.
    #[serde(default)]
    pub created_at: String,
}

/// One item appended to, or emitted from, a session. Type names follow the
/// Managed Agents event vocabulary so both backends stream the same shape.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeEvent {
    /// Event type (e.g. a user message, a model delta, a tool use/result, a
    /// status change).
    #[serde(default)]
    pub r#type: String,

    /// Message role, when the event carries one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,

    /// Text payload, when the event carries one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content: String,

    /// Structured payload, when the event carries one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// RFC3339 timestamp.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub timestamp: String,

    /// 1-based sequence number, assigned only to durable structural events.
    /// Send the last one back as `since` to resume a dropped stream. Zero for
    /// ephemeral events (bash output, token deltas), which are never
    /// persisted or replayed.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub index: i64,
}

fn is_zero(value: &i64) -> bool {
    *value == 0
}

/// Request body for `POST /qai/v1/agent-runtime/sessions/events`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AppendEventRequest {
    /// The session descriptor returned by
    /// [`Client::agent_runtime_session_start`].
    pub session: RuntimeSession,

    /// The event to append.
    pub event: RuntimeEvent,
}

/// Response from the session routes that only report success.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimeOkResponse {
    /// True once the call was accepted.
    #[serde(default)]
    pub ok: bool,
}

/// Response from `POST /qai/v1/agent-runtime/workspaces`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct StageWorkspaceResponse {
    /// The staged object, to set as
    /// [`OverlayConfig::workspace_object`] on the environment that launches
    /// the session.
    #[serde(default)]
    pub workspace_object: String,
}

/// An async stream of [`RuntimeEvent`]s from a session's SSE stream.
pub struct RuntimeEventStream {
    inner: std::pin::Pin<Box<dyn Stream<Item = RuntimeEvent> + Send>>,
}

impl Stream for RuntimeEventStream {
    type Item = RuntimeEvent;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl Client {
    // ── Agents ──────────────────────────────────────────────────────────────

    /// Creates a runtime agent.
    ///
    /// `POST /qai/v1/agent-runtime/agents`
    pub async fn agent_runtime_agent_create(
        &self,
        req: &RuntimeAgentRequest,
    ) -> Result<RuntimeAgent> {
        let (resp, _meta) = self
            .post_json::<RuntimeAgentRequest, RuntimeAgent>("/qai/v1/agent-runtime/agents", req)
            .await?;
        Ok(resp)
    }

    /// Lists the caller's runtime agents. `limit` caps the page size.
    ///
    /// `GET /qai/v1/agent-runtime/agents`
    pub async fn agent_runtime_agents(&self, limit: Option<u32>) -> Result<RuntimeAgentsResponse> {
        let path = match limit {
            Some(limit) => format!("/qai/v1/agent-runtime/agents?limit={limit}"),
            None => "/qai/v1/agent-runtime/agents".to_string(),
        };
        let (resp, _meta) = self.get_json::<RuntimeAgentsResponse>(&path).await?;
        Ok(resp)
    }

    /// Reads one runtime agent.
    ///
    /// `GET /qai/v1/agent-runtime/agents/{id}`
    pub async fn agent_runtime_agent_get(&self, id: &str) -> Result<RuntimeAgent> {
        let (resp, _meta) = self
            .get_json::<RuntimeAgent>(&format!("/qai/v1/agent-runtime/agents/{id}"))
            .await?;
        Ok(resp)
    }

    /// Updates a runtime agent's config and returns the new version.
    ///
    /// `PUT /qai/v1/agent-runtime/agents/{id}`
    pub async fn agent_runtime_agent_update(
        &self,
        id: &str,
        req: &RuntimeAgentRequest,
    ) -> Result<RuntimeAgentUpdateResponse> {
        let (resp, _meta) = self
            .put_json::<RuntimeAgentRequest, RuntimeAgentUpdateResponse>(
                &format!("/qai/v1/agent-runtime/agents/{id}"),
                req,
            )
            .await?;
        Ok(resp)
    }

    /// Deletes a runtime agent.
    ///
    /// `DELETE /qai/v1/agent-runtime/agents/{id}`
    pub async fn agent_runtime_agent_delete(&self, id: &str) -> Result<()> {
        self.delete_no_content(&format!("/qai/v1/agent-runtime/agents/{id}"))
            .await
    }

    // ── Environments ────────────────────────────────────────────────────────

    /// Creates a runtime environment.
    ///
    /// `POST /qai/v1/agent-runtime/environments`
    pub async fn agent_runtime_environment_create(
        &self,
        req: &RuntimeEnvironmentRequest,
    ) -> Result<RuntimeEnvironment> {
        let (resp, _meta) = self
            .post_json::<RuntimeEnvironmentRequest, RuntimeEnvironment>(
                "/qai/v1/agent-runtime/environments",
                req,
            )
            .await?;
        Ok(resp)
    }

    /// Lists the caller's runtime environments. `limit` caps the page size.
    ///
    /// `GET /qai/v1/agent-runtime/environments`
    pub async fn agent_runtime_environments(
        &self,
        limit: Option<u32>,
    ) -> Result<RuntimeEnvironmentsResponse> {
        let path = match limit {
            Some(limit) => format!("/qai/v1/agent-runtime/environments?limit={limit}"),
            None => "/qai/v1/agent-runtime/environments".to_string(),
        };
        let (resp, _meta) = self.get_json::<RuntimeEnvironmentsResponse>(&path).await?;
        Ok(resp)
    }

    /// Deletes a runtime environment.
    ///
    /// `DELETE /qai/v1/agent-runtime/environments/{id}`
    pub async fn agent_runtime_environment_delete(&self, id: &str) -> Result<()> {
        self.delete_no_content(&format!("/qai/v1/agent-runtime/environments/{id}"))
            .await
    }

    // ── Sessions ────────────────────────────────────────────────────────────

    /// Starts a session for an agent in an environment.
    ///
    /// Sessions on a `managed-agents` environment are admin-only; the
    /// `coding-session` backend is open to any owner.
    ///
    /// `POST /qai/v1/agent-runtime/sessions`
    pub async fn agent_runtime_session_start(
        &self,
        req: &StartSessionRequest,
    ) -> Result<RuntimeSession> {
        let (resp, _meta) = self
            .post_json::<StartSessionRequest, RuntimeSession>("/qai/v1/agent-runtime/sessions", req)
            .await?;
        Ok(resp)
    }

    /// Appends an event to a running session — this is how a user turn is
    /// sent.
    ///
    /// `POST /qai/v1/agent-runtime/sessions/events`
    pub async fn agent_runtime_session_event(
        &self,
        session: &RuntimeSession,
        event: &RuntimeEvent,
    ) -> Result<RuntimeOkResponse> {
        let req = AppendEventRequest {
            session: session.clone(),
            event: event.clone(),
        };
        let (resp, _meta) = self
            .post_json::<AppendEventRequest, RuntimeOkResponse>(
                "/qai/v1/agent-runtime/sessions/events",
                &req,
            )
            .await?;
        Ok(resp)
    }

    /// Streams a session's events.
    ///
    /// The session descriptor rides the query string, so the stream is a
    /// stateless GET. Pass `since` — the [`RuntimeEvent::index`] of the last
    /// structural event seen — to replay the durable events past it before
    /// bridging to the live stream; ephemeral events are not replayed.
    ///
    /// `GET /qai/v1/agent-runtime/sessions/stream`
    pub async fn agent_runtime_session_stream(
        &self,
        session: &RuntimeSession,
        since: Option<i64>,
    ) -> Result<RuntimeEventStream> {
        let descriptor = serde_json::to_string(session)?;
        let mut path = format!(
            "/qai/v1/agent-runtime/sessions/stream?session={}",
            urlencoding::encode(&descriptor)
        );
        if let Some(since) = since {
            path.push_str(&format!("&since={since}"));
        }
        let (resp, _meta) = self.get_stream_raw(&path).await?;
        let payloads = sse_data_payloads(resp.bytes_stream());
        let events = payloads.filter_map(|payload| async move {
            serde_json::from_str::<RuntimeEvent>(&payload).ok()
        });
        Ok(RuntimeEventStream {
            inner: Box::pin(events),
        })
    }

    /// Stops a running session.
    ///
    /// `POST /qai/v1/agent-runtime/sessions/stop`
    pub async fn agent_runtime_session_stop(
        &self,
        session: &RuntimeSession,
    ) -> Result<RuntimeOkResponse> {
        let (resp, _meta) = self
            .post_json::<RuntimeSession, RuntimeOkResponse>(
                "/qai/v1/agent-runtime/sessions/stop",
                session,
            )
            .await?;
        Ok(resp)
    }

    /// Downloads a session's current server-side working tree as a `.tar.gz`.
    ///
    /// This is the whole-copy counterpart to the per-turn diff, for clients
    /// with no local checkout to apply a diff onto. `coding-session` only.
    ///
    /// `POST /qai/v1/agent-runtime/sessions/workspace`
    pub async fn agent_runtime_session_workspace(
        &self,
        session: &RuntimeSession,
    ) -> Result<Vec<u8>> {
        let (resp, _meta) = self
            .post_stream_raw("/qai/v1/agent-runtime/sessions/workspace", session)
            .await?;
        Ok(resp.bytes().await?.to_vec())
    }

    /// Stages a workspace archive for a later session launch.
    ///
    /// `archive` is a `.tar.gz` or `.zip` of the tree (150 MB cap). Set the
    /// returned object as [`OverlayConfig::workspace_object`] on the
    /// environment that launches the session.
    ///
    /// `POST /qai/v1/agent-runtime/workspaces` (multipart, field `file`)
    pub async fn agent_runtime_stage_workspace(
        &self,
        filename: &str,
        archive: Vec<u8>,
    ) -> Result<StageWorkspaceResponse> {
        let part = reqwest::multipart::Part::bytes(archive)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| {
                Error::Api(ApiError {
                    status_code: 0,
                    code: "multipart_error".into(),
                    message: e.to_string(),
                    request_id: String::new(),
                })
            })?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let (resp, _meta) = self
            .post_multipart::<StageWorkspaceResponse>("/qai/v1/agent-runtime/workspaces", form)
            .await?;
        Ok(resp)
    }

    /// Sends a DELETE that the gateway answers with `204 No Content`.
    ///
    /// The shared `delete_json` helper always decodes a JSON body, which an
    /// empty 204 has none of, so these routes need their own send.
    async fn delete_no_content(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url(), path);
        let auth = self.auth_header().clone();
        // Proxies that claim the Authorization header read X-API-Key instead,
        // so send the raw token alongside — same pairing the shared client
        // applies to every other request.
        let raw_token = auth
            .to_str()
            .unwrap_or_default()
            .strip_prefix("Bearer ")
            .unwrap_or_default()
            .to_string();

        let http = reqwest::Client::builder().build()?;
        let mut req = http.delete(&url).header(AUTHORIZATION, auth);
        if let Ok(value) = HeaderValue::from_str(&raw_token) {
            req = req.header("X-API-Key", value);
        }
        let resp = req.send().await?;

        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let request_id = resp
            .headers()
            .get("X-QAI-Request-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let body = resp.text().await.unwrap_or_default();
        Err(Error::Api(ApiError {
            status_code: status.as_u16(),
            code: status.canonical_reason().unwrap_or("Unknown").to_string(),
            message: body,
            request_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_request_omits_empty_prompt_and_tools() {
        let req = RuntimeAgentRequest {
            name: "reviewer".into(),
            model: "claude-sonnet-4-6".into(),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["name"], "reviewer");
        assert!(json.get("system_prompt").is_none());
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn session_round_trips_as_the_descriptor_the_gateway_expects() {
        let session = RuntimeSession {
            id: "s1".into(),
            user_id: "u1".into(),
            agent_id: "a1".into(),
            environment_id: "e1".into(),
            backend: "coding-session".into(),
            status: "running".into(),
            upstream_id: "sesn_123".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let encoded = serde_json::to_string(&session).expect("serialize");
        let decoded: RuntimeSession = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded.upstream_id, "sesn_123");
        assert_eq!(decoded.backend, "coding-session");
    }

    #[test]
    fn ephemeral_event_omits_its_zero_index() {
        let event = RuntimeEvent {
            r#type: "message".into(),
            role: "user".into(),
            content: "run the tests".into(),
            ..Default::default()
        };
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["type"], "message");
        assert!(json.get("index").is_none());
        assert!(json.get("data").is_none());
        assert!(json.get("timestamp").is_none());
    }

    #[test]
    fn environment_decodes_without_an_overlay() {
        let env: RuntimeEnvironment = serde_json::from_str(
            r#"{"id":"e1","user_id":"u1","name":"hosted","backend":"managed-agents",
                "vault_ids":null,"created_at":"2026-01-01T00:00:00Z"}"#,
        )
        .expect("decode");
        assert_eq!(env.backend, "managed-agents");
        assert!(env.overlay.is_none());
        assert!(env.vault_ids.is_empty());
    }
}
