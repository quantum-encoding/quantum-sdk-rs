//! Authentication — sign in via OAuth providers, verify keys, end sessions.
//!
//! Every sign-in answers with the same [`AuthResponse`]: a session token
//! (the bearer for later calls), when it expires, the account's default API
//! key, and the user. An app that signs a person in never has to hold a
//! developer key — the token is the credential.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// User information returned after authentication.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthUser {
    /// User identifier.
    pub id: String,

    /// Display name (`display_name` on the wire).
    #[serde(default, alias = "display_name")]
    pub name: Option<String>,

    /// Email address.
    #[serde(default)]
    pub email: Option<String>,

    /// Avatar URL (`photo_url` on the wire).
    #[serde(default, alias = "photo_url")]
    pub avatar_url: Option<String>,

    /// Credit balance in ticks (10¹⁰ ticks per USD).
    #[serde(default)]
    pub credit_ticks: Option<i64>,

    /// Account role (e.g. `"user"`, `"admin"`).
    #[serde(default)]
    pub role: Option<String>,
}

/// Response from authentication endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    /// Session token: the bearer for subsequent requests.
    pub token: String,

    /// When the session token expires (RFC 3339).
    #[serde(default)]
    pub expires_at: Option<String>,

    /// The account's default API key, for clients that persist a key
    /// rather than a session.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Email address of the signed-in account.
    #[serde(default)]
    pub email: Option<String>,

    /// Credit balance in USD at sign-in.
    #[serde(default)]
    pub credit_usd: Option<f64>,

    /// Authenticated user information.
    pub user: AuthUser,
}

/// Request body for Apple Sign-In.
#[derive(Debug, Clone, Serialize)]
pub struct AuthAppleRequest {
    /// The Apple identity token (JWT from Sign in with Apple).
    pub id_token: String,

    /// Optional display name (only provided on first sign-in).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The raw nonce the sign-in was started with; its SHA-256 is checked
    /// against the token's claim for replay protection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,

    /// Per-device key bucket: each device gets its own default key. Empty
    /// means the account's shared default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,

    /// The authorization code from Sign in with Apple, when the app has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<String>,
}

/// Request body for Google Sign-In.
#[derive(Debug, Clone, Serialize)]
pub struct AuthGoogleRequest {
    /// The Google ID token (JWT) from the OAuth flow.
    pub id_token: String,

    /// The OAuth client ID the token was issued for; it must be one the
    /// gateway recognises, and the token's audience is checked against it.
    pub client_id: String,

    /// Per-device key bucket (see [`AuthAppleRequest::device_id`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

/// Request body for Firebase sign-in (any Firebase Auth provider).
#[derive(Debug, Clone, Serialize)]
pub struct AuthFirebaseRequest {
    /// The Firebase ID token.
    pub id_token: String,

    /// Per-device key bucket (see [`AuthAppleRequest::device_id`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

/// Request body for key verification.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VerifyKeyRequest {
    /// The `qai_k_` key to resolve. Empty verifies the calling credential.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Who a key belongs to.
#[derive(Debug, Clone, Deserialize)]
pub struct VerifyKeyResponse {
    /// Always true on a 200; an unknown key is a 401.
    pub verified: bool,

    /// Owner's user id.
    pub user_id: String,

    /// Owner's Apple subject, when they signed in with Apple.
    #[serde(default)]
    pub apple_sub: Option<String>,

    /// Owner's email.
    #[serde(default)]
    pub email: Option<String>,

    /// When the key was created (RFC 3339).
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Outcome of revoking the calling session.
#[derive(Debug, Clone, Deserialize)]
pub struct RevokeSessionResponse {
    /// `"revoked"`.
    pub status: String,
}

impl Client {
    /// Authenticate with Apple Sign-In.
    ///
    /// The `id_token` is the JWT received from the Sign in with Apple flow.
    /// On first sign-in, pass the user's `name` so the account is created
    /// with a display name.
    pub async fn auth_apple(&self, req: &AuthAppleRequest) -> Result<AuthResponse> {
        let (resp, _meta) = self
            .post_json::<AuthAppleRequest, AuthResponse>("/qai/v1/auth/apple", req)
            .await?;
        Ok(resp)
    }

    /// Authenticate with Google Sign-In.
    ///
    /// The `id_token` is the JWT from Google's OAuth flow and `client_id`
    /// the OAuth client it was issued for. Construct the client with any
    /// placeholder key: the call needs none, and the response's `token`
    /// becomes the credential for everything after.
    pub async fn auth_google(&self, req: &AuthGoogleRequest) -> Result<AuthResponse> {
        let (resp, _meta) = self
            .post_json::<AuthGoogleRequest, AuthResponse>("/qai/v1/auth/google", req)
            .await?;
        Ok(resp)
    }

    /// Authenticate with a Firebase ID token (any Firebase Auth provider).
    pub async fn auth_firebase(&self, req: &AuthFirebaseRequest) -> Result<AuthResponse> {
        let (resp, _meta) = self
            .post_json::<AuthFirebaseRequest, AuthResponse>("/qai/v1/auth/firebase", req)
            .await?;
        Ok(resp)
    }

    /// Resolve a `qai_k_` key to its owner. For services that accept a
    /// customer's key and need to know whose it is; with no key in the
    /// request, the calling credential is the one verified.
    pub async fn verify_key(&self, req: &VerifyKeyRequest) -> Result<VerifyKeyResponse> {
        let (resp, _meta) = self
            .post_json::<VerifyKeyRequest, VerifyKeyResponse>("/qai/v1/auth/verify-key", req)
            .await?;
        Ok(resp)
    }

    /// Sign out: revoke the session token this client was built with.
    pub async fn revoke_session(&self) -> Result<RevokeSessionResponse> {
        let (resp, _meta) = self
            .delete_json::<RevokeSessionResponse>("/qai/v1/auth/session")
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sign_in_reads_the_gateways_user_shape() {
        // The gateway spells the user's fields display_name / photo_url;
        // the SDK's names stay stable and read either spelling.
        let body = r#"{"token":"qai_s_x","session_token":"qai_s_x","expires_at":"2026-09-06T00:00:00Z",
            "api_key":"qai_k_y","email":"a@b.c","credit_usd":1.5,
            "user":{"id":"u1","email":"a@b.c","display_name":"Ada","photo_url":"https://p/x.png","credit_ticks":15000000000,"role":"user"}}"#;
        let r: AuthResponse = serde_json::from_str(body).unwrap();
        assert_eq!(r.token, "qai_s_x");
        assert_eq!(r.api_key.as_deref(), Some("qai_k_y"));
        assert_eq!(r.user.name.as_deref(), Some("Ada"));
        assert_eq!(r.user.avatar_url.as_deref(), Some("https://p/x.png"));
        assert_eq!(r.user.credit_ticks, Some(15_000_000_000));
    }

    #[test]
    fn a_google_sign_in_sends_only_what_it_has() {
        let req = AuthGoogleRequest {
            id_token: "t".into(),
            client_id: "c".into(),
            device_id: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, r#"{"id_token":"t","client_id":"c"}"#);
    }
}
