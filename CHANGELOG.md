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

## 0.8.1

- Client-level routing region rides every chat call.
