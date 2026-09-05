# Changelog

## 0.9.0 — unreleased

Parity with the gateway as of September 2026.

### Added
- Sign-in: `auth_google`, `auth_firebase` beside `auth_apple`; `AuthResponse` now carries
  `expires_at`, `api_key`, `email`, `credit_usd`, and the user's `credit_ticks` and `role`.
  `AuthAppleRequest` gains `nonce`, `device_id`, `authorization_code`.
- `verify_key` (resolve a `qai_k_` key to its owner) and `revoke_session` (sign out).
- Keys: `list_device_keys`, `rotate_key`, `key_usage`, `create_ephemeral_key`,
  `create_partner_key`.
- Credits: `lifetime_plans`, `lifetime_purchase`.
- Account: `account_delete` (sends the confirmation phrase for you), `account_deletion_status`.
- New surfaces: media sessions, files (multipart upload), Gemini context caches, licences
  (`mine`, `revocations`, `public-key`), the code scanner (scan, upload, diff, verify, audit,
  vulnerabilities, scans and their types and graph), the agent runtime (agents, environments,
  sessions with an SSE stream, workspaces), the managed-agents passthrough, Cloud Run (SSE)
  and dedicated inference endpoints (buffered and streaming).
- Audio: `audio/stt/realtime-token`. Realtime: the ElevenLabs WebSocket proxy with
  `send_elevenlabs_audio` / `send_json`. Compute: catalog, deploy-model (estimate and
  confirmed), deployments list/get/extend/delete. Security: `scan-code`.

### Changed (breaking)
- **Retries.** A POST is replayed only on 429, never on 502/503/504. The gateway does not
  dedupe on `Idempotency-Key` for chat, session chat or any media route (they bill through a
  reserve→settle rail that never reads the header), and key-minting and Stripe checkout routes
  have no dedupe at all, so a replay after a 5xx that masked a completed operation ran — and
  charged for — it twice. Passing an explicit key to `post_json_with_idempotency` /
  `post_json_empty_with_idempotency` opts that request into 5xx replay; the doc lists the routes
  that honour the key and warns that the gateway's dedupe cache ignores the request body. GETs
  still replay on all four statuses. `Retry-After` is honoured on 429 (clamped to 30 s); the
  0.5 s / 1 s / 2 s backoff applies only when the header is absent.
- `Client::new` returns `Result<Client>` and no longer panics on a key that cannot be an HTTP
  header value (a trailing newline read from a file, say); it fails with the builder's
  `invalid_api_key` error.
- The SDK never writes to stderr. The `[sdk] …` retry notices are gone, and a JSON decode
  failure on a 2xx body is `Error::Json` carrying serde's position only — the body, which on
  sign-in and key-minting routes opens with a live credential, is not echoed anywhere.
- `get_pricing` returns `HashMap<String, PricingInfo>` keyed by model id; `PricingInfo` is now an
  alias of `PricingEntry` with the fields the gateway sends. It previously expected an array and
  always failed to decode.
- `CreditTier` has the wire shape: `tier`, `label`, `margin_percent`, `description`,
  `requirements` (`name` / `min_balance` / `discount_percent` / `extra` never carried data).
- `DevProgramApplyRequest::expected_usd` is `expected_monthly_usd`, the name the gateway reads;
  the old field was silently dropped server-side.
- `StreamEvent` gains `citations` (the gateway's `citations` event, previously discarded) and
  `session` (the event that opens a session stream). The failure types `invalid_request` and
  `rate_limit` now fill `error` like `error` does; `StreamEvent::is_error()` covers all three.
- `ChatMessage.content_blocks`: a malformed block array is a decode error, not `None`.
- `chat_session` always sends `stream: false`; `chat_session_stream` is the streaming form and
  returns `SessionChatStream { session_id, events }`. A request with `stream: Some(true)` used
  to be sent buffered, billed, and then fail to decode the SSE body.
- `ErrorCode` gains `KeyRotated`, `AccountDeleted`, `AppScopeMismatch`,
  `ProviderFeatureDisallowed`, `FileMimeUnsupported`, `NotFound`, `PermissionDenied`,
  `InvalidRequest`, and generic `AuthenticationError`, `InvalidState`, `ProviderError`,
  `RateLimited`. `typed_code()` folds the gateway's legacy lowercase `type` strings
  (`invalid_request`, `authentication_error`, `forbidden`, `not_found`, `provider_error`,
  `invalid_state`, …) onto those variants instead of `Unknown`.
- Streaming requests (`chat_stream`, `chat_session_stream`, every SSE helper) now send
  `X-API-Key` and the builder's extra headers exactly like buffered requests, from one shared
  no-timeout client built at construction time. Behind a proxy that consumed `Authorization`,
  `chat()` worked and `chat_stream()` got 401.
- RAG collections were wrong on the wire and could not have worked: `collections_search`
  now posts to `/rag/collections/search` with `max_chunks`; `collections_get` returns
  `CollectionDetail` (`{collection, documents}`); `collections_delete` returns
  `DeleteCollectionResponse`; `collections_create` takes a request struct;
  `CollectionUploadResult` is an alias of `CollectionDocument`; `Collection` and
  `CollectionDocument` fields match the gateway.

### Deprecated
- `compute_billing`, `scrape`, `screenshot`: their routes no longer exist on the gateway.

### Fixed
- Docs corrected to what the gateway does: the semantic-cache header is `X-QAI-Cache` and a hit
  is free and unmetered; the streaming `usage` event's `output_tokens` excludes reasoning (the
  envelope's includes it); `X-QAI-Balance-After` is sent by media routes only; `is_tool_use()`
  is false when a provider reports `max_tokens` / `content_filter` / `error` beside tool calls;
  `summarize_strategy` distinguishes only `plan_and_tools`; ephemeral and partner keys need the
  `internal` tier; Google sign-in accepts a token minted for any recognised client id;
  `revoke_session` is for session tokens only; an out-of-range usage `limit` is ignored, not
  clamped. README examples compile as doctests; the stale "v0.4.0 / scanner" line is gone.
- `AuthUser.name` / `avatar_url` read the gateway's `display_name` / `photo_url`.
- Doc links in `jobs` resolve.
- `cargo fmt` across the crate.

### Media, jobs and batch

**Added**
- `video_twin` (`POST /qai/v1/video/twin-video`) with `TwinVideoRequest`: render a video of a
  trained avatar look from a script + `voice_id` or supplied `audio_base64`.
- `DigitalTwinCreateRequest` / `DigitalTwinCreateResponse` for the twin *creation* route
  (`name` + `video_url` → `group_id`, `consent_url`, `avatar_id`).
- `JobStreamEvent::is_stream_timeout()`: tells the stream's own 10-minute deadline event apart
  from a job failure (the job keeps running).
- `Voice` gains `category`, `model` (the TTS model id to pass to `speak`) and `description`;
  `VoicesResponse` gains `request_id`. `SharedVoice` gains `descriptive`,
  `usage_character_count`, `live_moderation_enabled`. `AddVoiceFromLibraryResponse` gains
  `status`. `BatchSubmitResponse` gains `jobs`, `batch_id`, `pricing`. `AlignRequest`,
  `IsolateVoiceRequest` and `RemixVoiceRequest` gain `filename`.
- Unit tests decode a fixture in every touched response type's handler shape and assert the
  exact keys every rebuilt request serialises.

**Changed (breaking)**
- **Requests the gateway rejected or ignored now match the handler**:
  - `RemixVoiceRequest`: `voice` / `model` / `output_format` were never read (the $0.30 remix
    ran with no attributes). Replaced by the gateway's knobs `gender`, `accent`, `style`,
    `pacing`, `audio_quality`, `prompt_strength`, `script`, `filename`.
  - `batch_submit_jsonl` sends the JSONL text as the raw request body (`application/x-ndjson`)
    instead of `{"jsonl": …}`, which the gateway always answered with 400.
  - `VideoStudioRequest` is `{avatar_id, script, voice_id}` (all required); `StudioClip` and the
    `clips` / `title` / `dimension` / `aspect_ratio` fields, which the route never read, are gone.
  - `VideoTranslateRequest` is `{video_url, output_language, source_language?, title?}`;
    `target_language` and `video_base64` had no counterpart on the gateway (every call was 400).
  - `video_digital_twin` **creates** a twin (`DigitalTwinCreateRequest` → synchronous
    `DigitalTwinCreateResponse`); rendering a twin video is the new `video_twin`. The old
    `DigitalTwinRequest {avatar_id, script}` is removed — it was sent to the wrong route.
  - `clone_voice(&CloneVoiceRequest)` sends the JSON body the route decodes
    (`name`, `description?`, `audio_samples: [base64]`); the multipart form was always 400.
    `CloneVoiceFile` is removed. `CloneVoiceResponse.status` → `request_id`.
  - `create_finetune(&MusicFinetuneCreateRequest)` sends JSON `{name, description?, samples}`
    and decodes the 201 `{id, status}` into `FinetuneInfo`; the multipart form was always 400.
  - `voice_library` sends the search text as `q` (the gateway ignored `query`, returning an
    unfiltered page).
  - `SpeechToSpeechRequest` is `{voice_id, audio_base64}` (both required); `StarfishTTSRequest`
    makes `voice_id` required. Their `voice` / `model` / `output_format` fields were never read.
  - `IsolateVoiceRequest.output_format` and `VoiceDesignRequest.output_format` are removed
    (not read; provider default format is returned).
  - `ElevenMusicRequest.model` is optional (gateway default `music_v1`); `edit_reference_id` /
    `edit_instruction` are removed (edits are not supported on this route).
- **Response types that could not decode now match the wire**:
  - `BatchJobInfo` is the gateway's `internal/batch.Job`: `id` (was `job_id`), `output` /
    `output_gcs` (was `result`), `type`, `priority`, `prompt`, `created_by`, `tokens`; no
    `cost_ticks`. Statuses are `queued|running|paused|complete|failed|cancelled`.
  - `FinetuneInfo` is `{id, status, model_id?}` (was `finetune_id`, `name`, …).
  - `SharedVoicesResponse.next_cursor` → `last_sort_id` (pass as `cursor`).
  - `Avatar.name` reads `avatar_name`, `Avatar.preview_url` reads `preview_image_url`, and
    `Avatar.avatar_type` reads `type`; `VideoTemplate.thumbnail_url` reads
    `thumbnail_image_url` (was `preview_url`); `HeyGenVoice.name` reads `display_name` and
    `preview_url` reads `preview_audio`. These fields are plain `String`s now and the `extra`
    maps are gone. `AvatarsResponse`, `VideoTemplatesResponse`, `HeyGenVoicesResponse` gain
    `request_id`.
  - `voice_design` returns `VoiceDesignResponse` (`previews[{generated_voice_id, audio_base64,
    format}]`); `resp.audio_base64` on the old generic type was always `None`.
  - `AlignResponse.segments` and `AlignmentSegment` are removed (never on the wire).
  - `JobStatusResponse.status` vocabulary is `pending|running|completed|failed`.
  - Every list the gateway can serialise as `null` decodes to an empty `Vec`.
- **One type per wire shape** (dead duplicates removed):
  - `JobAcceptedResponse` is the single 202 envelope (gains optional `created_at`);
    `JobCreateResponse` and `video::JobResponse` are aliases of it (the old `JobResponse.extra`
    / `cost_ticks` are gone — the envelope never carried a cost).
  - `JobListResponse { jobs: Vec<JobStatusResponse>, request_id }` is the list type;
    `ListJobsResponse` is an alias, `JobListEntry` aliases `JobStatusResponse`; `JobSummary`
    is removed.
  - `BatchJsonlResponse` aliases `BatchSubmitResponse`.
  - `HeyGenAvatarsResponse` / `HeyGenTemplatesResponse` (raw `Vec<Value>`) are removed in
    favour of the typed `AvatarsResponse` / `VideoTemplatesResponse`.
  - `MusicAdvancedRequest` / `MusicAdvancedClip` / `MusicAdvancedResponse`,
    `MusicFinetuneInfo` / `MusicFinetuneListResponse` are removed (`ElevenMusic*`,
    `FinetuneInfo`, `ListFinetunesResponse` are the live names).
  - `AudioResponse` (generic, `extra`-map) is removed: `dialogue`, `speech_to_speech`,
    `isolate_voice`, `remix_voice`, `dub`, `voice_design`, `starfish_tts` return their typed
    responses (`DialogueResponse`, `SpeechToSpeechResponse`, `IsolateVoiceResponse`,
    `RemixVoiceResponse`, `DubResponse`, `VoiceDesignResponse`, `StarfishTTSResponse`).
- `poll_job` returns `Err(ApiError { code: "poll_timeout", status_code: 0 })` when
  `max_attempts` runs out instead of `Ok` with a synthetic `"timeout"` status. (`mesh` 3D
  helpers inherit this.)
- `DialogueRequest::from_turns` returns `Result`: an `invalid_request` error (status 0, raised
  locally) when a speaker has no voice on any turn or two turns give the same speaker different
  voices, instead of sending a script the gateway bills with an unmapped speaker.
- `VoiceLibraryQuery.query` serialises as `q`.

**Fixed (docs)**
- `vision`: `image_url` is fetched server-side by current gateways (400 on fetch failure);
  older gateways pasted it into the prompt as text. `image_base64` documented as the reliable
  path. The `vision_*` methods only set the default profile; a request `profile` overrides it.
- `batch`: results are read via `batch_jobs` / `batch_job`, not the Jobs API; the list is the
  caller's slice of the newest 100 batch jobs across all users and `batch_job` scans the newest
  200; `job_ids` can be shorter than the input (skipped jobs have no per-index error).
- `list_jobs` returns the newest 50. `stream_job` documents the 10-minute timeout event.
- `video_template_generate`: submit still answers 400 (empty `variables`) and 402 (balance
  preflight); only HeyGen-side validation is deferred to the job.
- `GeneratedVideo.base64` is always bytes; the 202 envelope never carries `cost_ticks`.
- `avatar`: the `voice_id`-must-be-omitted rule surfaces as a 502 `provider_error`; the
  post-`final` 410 is HeyGen's documented status passed through as `provider_error`.
- `audio_stt_realtime_token`: single-use is ElevenLabs behaviour; the gateway bills flat at mint.

### Agents, compute, RAG and mesh

**Added**
- `agent_step` (`POST /qai/v1/agent`): one non-streaming model turn with tool-call
  passthrough. New types `AgentMessage` (with `user`/`system`/`assistant`/`tool_result`
  constructors), `AgentToolDef`, `AgentToolUse`, `AgentResponse` (`text()`, `to_message()`,
  `cost_ticks` from the response header), `AgentContentPart`, `AgentUsage`.
- `RuntimeAgentUpdate` for `agent_runtime_agent_update`: `None` keeps the stored value; the
  SDK reads the agent and merges before the PUT, since the route wipes an omitted
  `system_prompt` / `tools`.
- `RigOutput` (+ `RigOutput::from_job`) — the typed `result` of a `3d/rig` job, with
  `BasicAnimations` on the real `*_url` keys (walk/run only; there are no idle animations).
- `MissionRetryResponse`: `mission_retry_task` now returns the retried task's `result` and
  `model` instead of discarding them.
- `ComputeTemplate` carries `hourly_usd` / `spot_hourly_usd` (the rates actually billed),
  `spot_allowed`, `requires_approval`, `min_deposit_usd`, `machine_type`, `category`,
  `description`, `disk_size_gb`, `boot_time_secs`.
- `SSHKeyRequest.username` (optional; defaults to `cosmic` server-side).
- `RetextureRequest` gains `image_style_url`, `enable_original_uv`, `remove_lighting`,
  `target_formats`; `RigRequest` gains `texture_image_url`.

**Changed (breaking)**
- `agent_run` is gone. It posted `{task, conductor_model, workers…}` to `/qai/v1/agent`,
  which requires `model` + `messages` and returns one JSON document, so every call was a
  400. `AgentRequest` is now that route's real body; `AgentWorker` / `AgentWorkerConfig`
  are removed. Server-side orchestration is `cloudrun` or `mission_run`.
- `compute_provision(req, confirm: bool)`: `confirm` sends `?confirm=yes`, which the
  `requires_approval` templates demand. Docs now state the approval gate, the upfront hour,
  the `min_deposit_usd` floor and the 30..=1440 `auto_teardown_minutes` clamp.
- `compute_instances` / `compute_instance` decode the gateway's actual shapes:
  `ComputeInstanceInfo` (`instance_id`, `external_ip`, `hourly_usd`, flat GET) is the one
  instance type; `ComputeInstance` and `InstanceResponse` are removed. Previously any
  non-empty list, and every single-instance GET, failed to decode.
- `ProvisionResponse` matches the handler (`machine_type`, `gpu_type`, `hourly_usd`,
  `cost_usd`, `external_ip`, `estimated_boot_secs`); the never-present `template`,
  `ssh_address`, `price_per_hour_usd` are gone.
- `SSHKeyRequest` sends `public_key` (was `ssh_public_key`, always 400).
- `RetextureRequest.prompt` → `text_style_prompt` (the old key was unknown to Meshy, so
  every retexture job failed).
- `BasicAnimations` fields renamed to the wire keys (`walking_glb_url`, …,
  `running_armature_glb_url`); the idle fields never existed.
- `SurrealRagProviderInfo.chunk_count` → `chunks: i64` (the wire key).
- `agent_runtime_agent_update` takes `&RuntimeAgentUpdate` instead of `&RuntimeAgentRequest`.
- `mission_retry_task` returns `MissionRetryResponse`.
- Dead request fields removed: `MissionRequest.auto_plan` / `worker_model`,
  `MissionPlanUpdate.system_prompt`, `VulnerabilityScanOptions.severity_threshold`.
- `compute_billing` and its `BillingRequest` / `BillingEntry` / `BillingResponse` removed
  (the route does not exist).
- `Generate3DRequest` alias removed (3D generation is `generate_3d` in `jobs`).
- Streams: `AgentStream` now yields a final `error` event with `transport: true` when the
  connection drops mid-run instead of ending as if complete. `RuntimeEventStream` yields an
  `unknown` event carrying the raw payload for a non-object `data:` line and an `error`
  event for a transport failure, instead of silently dropping them.

**Fixed**
- `rag_search`, `surreal_rag_search`, `surreal_rag_providers` decode the `null` the gateway
  sends for an empty list; `SurrealRagResult.title` / `heading` / `source_file` are
  optional on the wire and default to empty.
- Query values in `mission_list` and `security_blocklist` are URL-encoded.
- The 204 DELETE routes (`agent_runtime_agent_delete`, `agent_runtime_environment_delete`)
  go through the shared HTTP client: same credentials, extra headers, timeout and error
  parsing as every other call.
- Docs corrected to the gateway: cloudrun / mission `workspace_path` is relative to a
  per-user server root (absolute and `..` rejected); scanner `scan` / `verify` local
  sources are fenced to `/workspace` and `/tmp`; `security_blocklist` is admin-only;
  `mission_cancel` charges ~$0.02/min for a running mission; cloudrun `tier` only feeds
  the budget guard and the final charge is at the conductor rate; the deploy estimate and
  extend routes need compute approval; `limit` outside 1..=500 becomes 50; the session
  routes still check ownership server-side; `OverlayConfig` exactly-one rule is not
  enforced at creation; collection `provider` is a label only; upload `status` is always
  `indexed` and `chunks` never set; `licenses_mine` omits a licence whose re-sign fails;
  `security_scan_code.url` accepts any `https://` git URL; codegen-only mission fields.


## 0.8.1

- Client-level routing region rides every chat call.
