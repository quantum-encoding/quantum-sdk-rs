use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};

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

/// Response from the contact form endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ContactResponse {
    /// Status message (e.g. "ok", "sent").
    pub status: String,

    /// Optional detail message.
    #[serde(default)]
    pub message: Option<String>,
}

impl Client {
    /// Sends a contact form message.
    ///
    /// This endpoint does not require authentication. A separate HTTP client
    /// is used to avoid sending API key headers.
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
            let status_code = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::error::Error::Api(crate::error::ApiError {
                status_code,
                code: "contact_error".to_string(),
                message: body,
                request_id: String::new(),
            }));
        }

        let result: StatusResponse = resp.json().await?;
        Ok(result)
    }
}
