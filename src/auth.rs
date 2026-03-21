//! Authentication — sign in via OAuth providers.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;

/// User information returned after authentication.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthUser {
    /// User identifier.
    pub id: String,

    /// Display name.
    #[serde(default)]
    pub name: Option<String>,

    /// Email address.
    #[serde(default)]
    pub email: Option<String>,

    /// Avatar URL.
    #[serde(default)]
    pub avatar_url: Option<String>,
}

/// Response from authentication endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    /// API token for subsequent requests.
    pub token: String,

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
}
