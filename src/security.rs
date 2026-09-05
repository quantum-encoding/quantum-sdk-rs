use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Result;
use crate::serde_util::null_as_default;

// ---------------------------------------------------------------------------
// Scan requests
// ---------------------------------------------------------------------------

/// Request body for scanning a URL for prompt injection.
#[derive(Debug, Clone, Serialize)]
pub struct SecurityScanUrlRequest {
    /// URL to scan.
    pub url: String,
}

/// Request body for scanning raw HTML content.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SecurityScanHtmlRequest {
    /// Raw HTML to scan.
    pub html: String,

    /// Rendered visible text (for structural analysis).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_text: Option<String>,

    /// Source URL (for context).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Request body for `POST /qai/v1/security/scan-code`.
///
/// Two modes: set `code` (with `filename` so the language can be detected) to
/// scan one file, or set `url` to clone a GitHub repository and scan a tree.
/// One of the two is required.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SecurityScanCodeRequest {
    /// Source of the single file to scan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Filename for `code`; the language is detected from its extension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Language override, when the filename does not settle it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// GitHub repository URL to clone and scan instead of `code`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Branch to check out for `url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Subdirectory of the checkout to scan. Must be relative and stay inside
    /// the repository root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// One issue found by the code security scan.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CodeScanFinding {
    /// Issue category.
    #[serde(default)]
    pub category: String,

    /// Severity (`critical`, `high`, `medium`, `low`).
    #[serde(default)]
    pub severity: String,

    /// The rule that matched.
    #[serde(default)]
    pub pattern: String,

    /// File the issue is in.
    #[serde(default)]
    pub file: String,

    /// Line the issue is on.
    #[serde(default)]
    pub line: i64,

    /// The offending line, truncated.
    #[serde(default)]
    pub content: String,

    /// What is wrong.
    #[serde(default)]
    pub description: String,

    /// Suggested fix, when the rule carries one.
    #[serde(default)]
    pub fix: String,

    /// Whether this matches a known machine-generated code pattern.
    #[serde(default)]
    pub ai_pattern: bool,
}

/// Response from `POST /qai/v1/security/scan-code`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CodeScanReport {
    /// What was scanned — the filename in single-file mode, the scanned
    /// directory otherwise.
    #[serde(default)]
    pub path: String,

    /// Number of files scanned.
    #[serde(default)]
    pub files_scanned: i64,

    /// Everything found.
    #[serde(default, deserialize_with = "null_as_default")]
    pub findings: Vec<CodeScanFinding>,

    /// Finding counts keyed by severity.
    #[serde(default, deserialize_with = "null_as_default")]
    pub summary: std::collections::HashMap<String, i64>,

    /// Finding counts keyed by category.
    #[serde(default, deserialize_with = "null_as_default")]
    pub categories: std::collections::HashMap<String, i64>,

    /// Security score from 0 to 100; higher is better.
    #[serde(default)]
    pub score: f64,
}

/// Request body for reporting a suspicious URL.
#[derive(Debug, Clone, Serialize)]
pub struct SecurityReportRequest {
    /// URL to report.
    pub url: String,

    /// Description of the suspected threat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Category of the suspected threat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response from a security scan.
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityScanResponse {
    /// Full threat assessment.
    pub assessment: SecurityAssessment,

    /// Request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Threat assessment for a scanned page.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SecurityAssessment {
    /// Source URL.
    #[serde(default)]
    pub url: String,

    /// Overall threat level: "none", "low", "medium", "high", "critical".
    #[serde(default)]
    pub threat_level: String,

    /// Numeric threat score (0.0 - 100.0).
    #[serde(default)]
    pub threat_score: f64,

    /// Individual findings.
    #[serde(default, deserialize_with = "null_as_default")]
    pub findings: Vec<SecurityFinding>,

    /// Length of hidden text content detected.
    #[serde(default)]
    pub hidden_text_length: i32,

    /// Length of visible text content.
    #[serde(default)]
    pub visible_text_length: i32,

    /// Ratio of hidden to total content.
    #[serde(default)]
    pub hidden_ratio: f64,

    /// Human-readable summary.
    #[serde(default)]
    pub summary: String,
}

/// A single detected injection pattern.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SecurityFinding {
    /// Category: "instruction_override", "role_impersonation", "data_exfiltration",
    /// "hidden_text", "hidden_comment", "model_targeting", "encoded_payload",
    /// "structural_anomaly", "meta_injection", "safety_override".
    #[serde(default)]
    pub category: String,

    /// Pattern that matched.
    #[serde(default)]
    pub pattern: String,

    /// Offending content (truncated).
    #[serde(default)]
    pub content: String,

    /// Location in the page.
    #[serde(default)]
    pub location: String,

    /// Threat level for this finding.
    #[serde(default)]
    pub threat: String,

    /// Detection confidence (0.0 - 1.0).
    #[serde(default)]
    pub confidence: f64,

    /// Human-readable description.
    #[serde(default)]
    pub description: String,
}

/// Response from checking a URL against the registry.
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityCheckResponse {
    /// URL that was checked.
    #[serde(default)]
    pub url: String,

    /// Whether the URL is blocked.
    #[serde(default)]
    pub blocked: bool,

    /// Threat level (if blocked).
    #[serde(default)]
    pub threat_level: Option<String>,

    /// Threat score (if blocked).
    #[serde(default)]
    pub threat_score: Option<f64>,

    /// Detection categories (if blocked).
    #[serde(default)]
    pub categories: Option<Vec<String>>,

    /// First seen timestamp.
    #[serde(default)]
    pub first_seen: Option<String>,

    /// Last seen timestamp.
    #[serde(default)]
    pub last_seen: Option<String>,

    /// Number of reports.
    #[serde(default)]
    pub report_count: Option<i32>,

    /// Registry status: "confirmed", "suspected".
    #[serde(default)]
    pub status: Option<String>,

    /// Human-readable message.
    #[serde(default)]
    pub message: Option<String>,
}

/// Response from the blocklist feed.
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityBlocklistResponse {
    /// Blocklist entries.
    #[serde(default, deserialize_with = "null_as_default")]
    pub entries: Vec<SecurityBlocklistEntry>,

    /// Total count.
    #[serde(default)]
    pub count: i32,

    /// Filter status used.
    #[serde(default)]
    pub status: String,
}

/// A single blocklist entry.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SecurityBlocklistEntry {
    /// Entry identifier.
    #[serde(default)]
    pub id: Option<String>,

    /// Blocked URL.
    #[serde(default)]
    pub url: String,

    /// Registry status.
    #[serde(default)]
    pub status: String,

    /// Threat level.
    #[serde(default)]
    pub threat_level: String,

    /// Threat score.
    #[serde(default)]
    pub threat_score: f64,

    /// Detection categories.
    #[serde(default, deserialize_with = "null_as_default")]
    pub categories: Vec<String>,

    /// Number of findings.
    #[serde(default)]
    pub findings_count: i32,

    /// Hidden content ratio.
    #[serde(default)]
    pub hidden_ratio: f64,

    /// First seen timestamp.
    #[serde(default)]
    pub first_seen: Option<String>,

    /// Summary.
    #[serde(default)]
    pub summary: String,
}

/// Response from reporting a URL.
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityReportResponse {
    /// URL that was reported.
    #[serde(default)]
    pub url: String,

    /// Report status: "existing" or "suspected".
    #[serde(default)]
    pub status: String,

    /// Message.
    #[serde(default)]
    pub message: String,

    /// Threat level (if already in registry).
    #[serde(default)]
    pub threat_level: Option<String>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Scan a URL for prompt injection attacks.
    pub async fn security_scan_url(&self, url: &str) -> Result<SecurityScanResponse> {
        let req = SecurityScanUrlRequest {
            url: url.to_string(),
        };
        let (resp, _) = self.post_json("/qai/v1/security/scan-url", &req).await?;
        Ok(resp)
    }

    /// Scan source code for security issues — one file inline, or a GitHub
    /// repository the gateway clones.
    ///
    /// `POST /qai/v1/security/scan-code`
    pub async fn security_scan_code(
        &self,
        req: &SecurityScanCodeRequest,
    ) -> Result<CodeScanReport> {
        let (resp, _) = self.post_json("/qai/v1/security/scan-code", req).await?;
        Ok(resp)
    }

    /// Scan raw HTML content for prompt injection.
    pub async fn security_scan_html(
        &self,
        req: &SecurityScanHtmlRequest,
    ) -> Result<SecurityScanResponse> {
        let (resp, _) = self.post_json("/qai/v1/security/scan-html", req).await?;
        Ok(resp)
    }

    /// Check a URL against the injection registry.
    pub async fn security_check(&self, url: &str) -> Result<SecurityCheckResponse> {
        let encoded = urlencoding::encode(url);
        let (resp, _) = self
            .get_json(&format!("/qai/v1/security/check?url={}", encoded))
            .await?;
        Ok(resp)
    }

    /// Get the injection blocklist feed.
    pub async fn security_blocklist(
        &self,
        status: Option<&str>,
    ) -> Result<SecurityBlocklistResponse> {
        let path = match status {
            Some(s) => format!("/qai/v1/security/blocklist?status={}", s),
            None => "/qai/v1/security/blocklist".into(),
        };
        let (resp, _) = self.get_json(&path).await?;
        Ok(resp)
    }

    /// Report a suspicious URL.
    pub async fn security_report(
        &self,
        req: &SecurityReportRequest,
    ) -> Result<SecurityReportResponse> {
        let (resp, _) = self.post_json("/qai/v1/security/report", req).await?;
        Ok(resp)
    }
}
