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
- RAG collections were wrong on the wire and could not have worked: `collections_search`
  now posts to `/rag/collections/search` with `max_chunks`; `collections_get` returns
  `CollectionDetail` (`{collection, documents}`); `collections_delete` returns
  `DeleteCollectionResponse`; `collections_create` takes a request struct;
  `CollectionUploadResult` is an alias of `CollectionDocument`; `Collection` and
  `CollectionDocument` fields match the gateway.

### Deprecated
- `compute_billing`, `scrape`, `screenshot`: their routes no longer exist on the gateway.

### Fixed
- `AuthUser.name` / `avatar_url` read the gateway's `display_name` / `photo_url`.
- Doc links in `jobs` resolve.
- `cargo fmt` across the crate.

## 0.8.1

- Client-level routing region rides every chat call.
