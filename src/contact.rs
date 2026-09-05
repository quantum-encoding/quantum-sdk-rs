use reqwest::header::CONTENT_TYPE;
use serde::Serialize;

use crate::client::Client;
use crate::error::Result;
use crate::keys::StatusResponse;

/// Request body for the public contact form.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ContactRequest {
    /// Sender name.
    pub name: String,

    /// Sender email address.
    pub email: String,

    /// Message subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Message body.
    pub message: String,
}

impl Client {
    /// Sends a contact form message.
    ///
    /// The route is unauthenticated, so the request goes out on a bare HTTP
    /// client with no credential headers. Validation failures arrive as
    /// typed [`ApiError`](crate::ApiError)s (`invalid_email`,
    /// `field_too_long`, `missing_fields`).
    pub async fn contact(&self, req: &ContactRequest) -> Result<StatusResponse> {
        let url = format!("{}/qai/v1/contact", self.base_url());

        let http = reqwest::Client::new();
        let resp = http
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .json(req)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(crate::client::parse_api_error(resp, "").await);
        }

        let result: StatusResponse = resp.json().await?;
        Ok(result)
    }
}
