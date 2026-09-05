//! Cross-app licences.
//!
//! Licences are minted by the fulfilment paths (Stripe webhook, in-app
//! purchase verification) and carry an Ed25519-signed JWT that a client caches
//! locally and verifies offline. These routes are the read / maintain surface:
//!
//! - [`Client::licenses_mine`] lists the caller's licences, each with a freshly
//!   signed JWT (a call renews the offline-validity window).
//! - [`Client::license_revocations`] is a public, id-only feed a client polls
//!   to drop refunded or disputed licences from its cache.
//! - [`Client::license_public_key`] is the public JWKS-style verification key.
//!
//! The revocations and public-key routes do not require authentication.

use serde::Deserialize;

use crate::client::Client;
use crate::error::Result;
use crate::serde_util::null_as_default;

/// One licence held by the caller.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct License {
    /// Licence identifier.
    #[serde(default)]
    pub id: String,

    /// App the licence unlocks.
    #[serde(default)]
    pub app: String,

    /// SKU purchased.
    #[serde(default)]
    pub sku: String,

    /// Fulfilment source (e.g. `"stripe"`, `"app_store"`, `"google_play"`).
    #[serde(default)]
    pub source: String,

    /// Provider-side transaction id the licence was minted from.
    #[serde(default)]
    pub source_transaction: String,

    /// RFC3339 issue timestamp of the underlying licence row (not of the JWT,
    /// which is re-signed on every read).
    #[serde(default)]
    pub issued_at: String,

    /// RFC3339 expiry.
    #[serde(default)]
    pub expires_at: String,

    /// `"active"` or `"revoked"`.
    #[serde(default)]
    pub status: String,

    /// The signed licence JWT. Empty for revoked licences — they stay in the
    /// list so a client can tell "receipt landed, access withdrawn" apart from
    /// "receipt never arrived".
    #[serde(default)]
    pub license_key: String,
}

/// Response from `GET /qai/v1/licenses/mine`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LicensesResponse {
    /// The caller's licences.
    #[serde(default, deserialize_with = "null_as_default")]
    pub licenses: Vec<License>,
}

/// Response from `GET /qai/v1/licenses/revocations`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LicenseRevocationsResponse {
    /// Licence ids revoked strictly after the requested `since`.
    #[serde(default, deserialize_with = "null_as_default")]
    pub revoked_ids: Vec<String>,

    /// The `since` bound the server actually applied, RFC3339.
    #[serde(default)]
    pub since: String,

    /// Server time the feed was generated, RFC3339. Use it as the next poll's
    /// `since`.
    #[serde(default)]
    pub as_of: String,
}

/// One JWKS entry carrying the licence verification key.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LicenseJwk {
    /// Key type — `"OKP"` for the Ed25519 licence key.
    #[serde(default)]
    pub kty: String,

    /// Curve — `"Ed25519"`.
    #[serde(default)]
    pub crv: String,

    /// Signing algorithm — `"EdDSA"`.
    #[serde(default)]
    pub alg: String,

    /// Key use — `"sig"`.
    #[serde(default, rename = "use")]
    pub key_use: String,

    /// Key id, matching the `kid` header of issued licence JWTs.
    #[serde(default)]
    pub kid: String,

    /// The base64url (unpadded) Ed25519 public key.
    #[serde(default)]
    pub x: String,
}

/// Response from `GET /qai/v1/licenses/public-key`.
///
/// Rotation appends a new entry while keeping the old one, so a client must
/// select by `kid` rather than assuming a single key.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LicensePublicKeyResponse {
    /// The active verification keys.
    #[serde(default, deserialize_with = "null_as_default")]
    pub keys: Vec<LicenseJwk>,
}

impl Client {
    /// Lists the caller's licences, each active one carrying a freshly signed
    /// JWT.
    ///
    /// Pass `app` to filter to a single app, or `None` for all apps.
    ///
    /// `GET /qai/v1/licenses/mine`
    pub async fn licenses_mine(&self, app: Option<&str>) -> Result<LicensesResponse> {
        let path = match app {
            Some(app) => format!("/qai/v1/licenses/mine?app={}", urlencoding::encode(app)),
            None => "/qai/v1/licenses/mine".to_string(),
        };
        let (resp, _meta) = self.get_json::<LicensesResponse>(&path).await?;
        Ok(resp)
    }

    /// Fetches licence ids revoked since a timestamp.
    ///
    /// `since` is RFC3339; omitting it defaults to 30 days ago server-side.
    ///
    /// `GET /qai/v1/licenses/revocations`
    pub async fn license_revocations(
        &self,
        since: Option<&str>,
    ) -> Result<LicenseRevocationsResponse> {
        let path = match since {
            Some(since) => format!(
                "/qai/v1/licenses/revocations?since={}",
                urlencoding::encode(since)
            ),
            None => "/qai/v1/licenses/revocations".to_string(),
        };
        let (resp, _meta) = self.get_json::<LicenseRevocationsResponse>(&path).await?;
        Ok(resp)
    }

    /// Fetches the public key(s) that verify licence JWTs.
    ///
    /// `GET /qai/v1/licenses/public-key`
    pub async fn license_public_key(&self) -> Result<LicensePublicKeyResponse> {
        let (resp, _meta) = self
            .get_json::<LicensePublicKeyResponse>("/qai/v1/licenses/public-key")
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoked_license_carries_no_key() {
        let resp: LicensesResponse = serde_json::from_str(
            r#"{"licenses":[
                {"id":"lic_1","app":"kitchenshare","sku":"pro","source":"stripe",
                 "source_transaction":"pi_1","issued_at":"2026-01-01T00:00:00Z",
                 "expires_at":"2027-01-01T00:00:00Z","status":"revoked"}]}"#,
        )
        .expect("decode");
        assert_eq!(resp.licenses.len(), 1);
        assert_eq!(resp.licenses[0].status, "revoked");
        assert!(resp.licenses[0].license_key.is_empty());
    }

    #[test]
    fn jwks_use_field_maps_from_reserved_name() {
        let resp: LicensePublicKeyResponse = serde_json::from_str(
            r#"{"keys":[{"kty":"OKP","crv":"Ed25519","alg":"EdDSA","use":"sig",
                        "kid":"k1","x":"AAAA"}]}"#,
        )
        .expect("decode");
        assert_eq!(resp.keys[0].key_use, "sig");
        assert_eq!(resp.keys[0].kid, "k1");
    }

    #[test]
    fn revocations_decode_null_list() {
        let resp: LicenseRevocationsResponse = serde_json::from_str(
            r#"{"revoked_ids":null,"since":"2026-01-01T00:00:00Z","as_of":"2026-02-01T00:00:00Z"}"#,
        )
        .expect("decode");
        assert!(resp.revoked_ids.is_empty());
        assert_eq!(resp.as_of, "2026-02-01T00:00:00Z");
    }
}
