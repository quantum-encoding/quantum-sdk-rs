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

### Fixed
- `AuthUser.name` / `avatar_url` read the gateway's `display_name` / `photo_url`.
- Doc links in `jobs` resolve.
- `cargo fmt` across the crate.

## 0.8.1

- Client-level routing region rides every chat call.
