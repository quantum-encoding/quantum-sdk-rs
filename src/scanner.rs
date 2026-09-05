//! Code scanner — structural analysis of a codebase.
//!
//! The scanner parses a source tree (a GitHub URL, an OpenAPI spec, or an
//! uploaded tar.gz) into a [`CodebaseGraph`] of modules, types, fields,
//! functions, and call edges. Scans are persisted per-user and can then be
//! diffed against each other, verified against a [`Blueprint`], queried a type
//! at a time for agent grounding, or rendered as an SVG graph.
//!
//! [`Client::scanner_audit`] is a separate, LLM-backed pass: it returns a job
//! id immediately and analyses each file asynchronously — poll it with
//! [`Client::get_job`](crate::Client::get_job).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::{ApiError, Error, Result};
use crate::serde_util::null_as_default;

// ── Code graph ──────────────────────────────────────────────────────────────

/// A source file or module in a scanned codebase.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeModule {
    /// Path relative to the scan root.
    #[serde(default)]
    pub path: String,

    /// Detected language (`rust`, `go`, `typescript`, `python`, `swift`,
    /// `kotlin`).
    #[serde(default)]
    pub language: String,

    /// Line count.
    #[serde(default)]
    pub lines: i64,
}

/// A field within a [`CodeType`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeField {
    /// Field name as declared.
    #[serde(default)]
    pub name: String,

    /// Declared type, as source text.
    #[serde(default)]
    pub r#type: String,

    /// Whether the field is optional / nullable.
    #[serde(default)]
    pub optional: bool,

    /// Serialised name from a JSON tag / serde rename, when the source
    /// declares one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub json_tag: String,
}

/// A struct, class, interface, enum, or data class.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeType {
    /// Type name.
    #[serde(default)]
    pub name: String,

    /// Declaration kind (`struct`, `class`, `interface`, `enum`,
    /// `data_class`).
    #[serde(default)]
    pub kind: String,

    /// File the type is declared in.
    #[serde(default)]
    pub file: String,

    /// First line of the declaration.
    #[serde(default)]
    pub line_start: i64,

    /// Last line of the declaration.
    #[serde(default)]
    pub line_end: i64,

    /// Declared visibility (`public`, `private`, `internal`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub visibility: String,

    /// The type's fields.
    #[serde(default, deserialize_with = "null_as_default")]
    pub fields: Vec<CodeField>,

    /// Names of methods attached to the type.
    #[serde(default, deserialize_with = "null_as_default")]
    pub methods: Vec<String>,

    /// Raw source, present only when the scan requested `include_source`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

/// A standalone function or a method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeFunction {
    /// Function name.
    #[serde(default)]
    pub name: String,

    /// File the function is declared in.
    #[serde(default)]
    pub file: String,

    /// First line of the declaration.
    #[serde(default)]
    pub line_start: i64,

    /// Last line of the declaration.
    #[serde(default)]
    pub line_end: i64,

    /// Parameter list, as source text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub params: String,

    /// Return type, as source text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub return_type: String,

    /// Type this function is a method on, when it is one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub receiver: String,

    /// Declared visibility (`public`, `private`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub visibility: String,

    /// Whether the function is asynchronous.
    #[serde(default)]
    pub is_async: bool,

    /// Raw source, present only when the scan requested `include_source`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

/// One function calling another, extracted when the scan requested
/// `include_call_graph`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallEdge {
    /// Caller, qualified as `file::function`.
    #[serde(default)]
    pub from: String,

    /// Name of the function being called.
    #[serde(default)]
    pub to: String,

    /// Line number of the call site.
    #[serde(default)]
    pub call_line: i64,
}

/// The full structural representation of a codebase.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodebaseGraph {
    /// Scanned modules.
    #[serde(default, deserialize_with = "null_as_default")]
    pub modules: Vec<CodeModule>,

    /// Declared types.
    #[serde(default, deserialize_with = "null_as_default")]
    pub types: Vec<CodeType>,

    /// Declared functions.
    #[serde(default, deserialize_with = "null_as_default")]
    pub functions: Vec<CodeFunction>,

    /// Call edges — empty unless the scan requested the call graph.
    #[serde(default, deserialize_with = "null_as_default")]
    pub call_edges: Vec<CallEdge>,
}

/// Counts summarising what a scan found.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanStats {
    /// Files parsed.
    #[serde(default)]
    pub files: i64,
    /// Types found.
    #[serde(default)]
    pub types: i64,
    /// Fields across all types.
    #[serde(default)]
    pub fields: i64,
    /// Functions found.
    #[serde(default)]
    pub functions: i64,
    /// Call edges extracted.
    #[serde(default)]
    pub call_edges: i64,
    /// Modules found.
    #[serde(default)]
    pub modules: i64,
}

// ── Scan ────────────────────────────────────────────────────────────────────

/// Request body for `POST /qai/v1/scanner/scan`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScanRequest {
    /// What to scan: a `github://owner/repo` or `https://github.com/...` URL,
    /// an OpenAPI spec URL, or a directory on the gateway's own filesystem
    /// under `/workspace` or `/tmp` (any other local path is 403
    /// `forbidden`). Required.
    pub source: String,

    /// Git branch to check out. Defaults to `main`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Restrict parsing to these languages. Empty scans every supported
    /// language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<String>>,

    /// Include raw source text on each type and function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_source: Option<bool>,

    /// Extract call edges between functions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_call_graph: Option<bool>,

    /// Label for this scan. Defaults to the source string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A completed scan and its graph.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScanResult {
    /// Scan identifier, used by the diff / verify / type-query routes.
    #[serde(default)]
    pub scan_id: String,

    /// Label of the scan.
    #[serde(default)]
    pub name: String,

    /// The source that was scanned.
    #[serde(default)]
    pub source: String,

    /// Summary counts.
    #[serde(default)]
    pub stats: ScanStats,

    /// The parsed graph. Listing endpoints return scans with an empty graph —
    /// fetch the scan by id for the full structure.
    #[serde(default)]
    pub graph: CodebaseGraph,

    /// RFC3339 creation timestamp.
    #[serde(default)]
    pub created_at: String,
}

/// Response from `GET /qai/v1/scanner/scans`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScanListResponse {
    /// The caller's scans, newest first (capped at 100 server-side).
    #[serde(default, deserialize_with = "null_as_default")]
    pub scans: Vec<ScanResult>,

    /// Gateway request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Response from `DELETE /qai/v1/scanner/scans/{id}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScanDeleteResponse {
    /// True once the scan is gone.
    #[serde(default)]
    pub deleted: bool,

    /// The scan that was deleted.
    #[serde(default)]
    pub scan_id: String,

    /// Gateway request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ── Type queries ────────────────────────────────────────────────────────────

/// A lightweight type entry from `GET /qai/v1/scanner/scans/{id}/types`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScanTypeSummary {
    /// Type name.
    #[serde(default)]
    pub name: String,

    /// Declaration kind.
    #[serde(default)]
    pub kind: String,

    /// File the type is declared in.
    #[serde(default)]
    pub file: String,

    /// Number of fields on the type.
    #[serde(default)]
    pub field_count: i64,

    /// Declared visibility.
    #[serde(default)]
    pub visibility: String,
}

/// Response from `GET /qai/v1/scanner/scans/{id}/types`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScanTypeListResponse {
    /// Every type in the scan, names and kinds only.
    #[serde(default, deserialize_with = "null_as_default")]
    pub types: Vec<ScanTypeSummary>,

    /// Number of entries in `types`.
    #[serde(default)]
    pub count: i64,

    /// The scan queried.
    #[serde(default)]
    pub scan_id: String,

    /// Label of the scan.
    #[serde(default)]
    pub scan_name: String,

    /// Gateway request identifier.
    #[serde(default)]
    pub request_id: String,
}

/// Response from `GET /qai/v1/scanner/scans/{id}/types/{name}` — one type with
/// everything an agent needs to write against it.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScanTypeDetail {
    /// The type itself, with its fields.
    #[serde(default, rename = "type")]
    pub code_type: CodeType,

    /// Methods attached to the type.
    #[serde(default, deserialize_with = "null_as_default")]
    pub methods: Vec<CodeFunction>,

    /// Types referenced by the type's field declarations.
    #[serde(default, deserialize_with = "null_as_default")]
    pub references: Vec<CodeType>,

    /// The scan queried.
    #[serde(default)]
    pub scan_id: String,

    /// Label of the scan.
    #[serde(default)]
    pub scan_name: String,

    /// Gateway request identifier.
    #[serde(default)]
    pub request_id: String,
}

// ── Diff ────────────────────────────────────────────────────────────────────

/// Request body for `POST /qai/v1/scanner/diff`.
///
/// Each side is given either by scan id or as an inline graph. The gateway
/// resolves the id form against the caller's own scans.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DiffRequest {
    /// Scan id of the reference codebase.
    #[serde(rename = "base", skip_serializing_if = "Option::is_none")]
    pub base_scan_id: Option<String>,

    /// Scan id of the codebase being compared.
    #[serde(rename = "target", skip_serializing_if = "Option::is_none")]
    pub target_scan_id: Option<String>,

    /// Inline reference graph, instead of `base`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_graph: Option<CodebaseGraph>,

    /// Inline comparison graph, instead of `target`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_graph: Option<CodebaseGraph>,

    /// Label for the reference side in the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_name: Option<String>,

    /// Label for the comparison side in the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_name: Option<String>,
}

/// A type present in both codebases under different casing conventions.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConventionDiff {
    /// Name as spelled in the reference codebase.
    #[serde(default)]
    pub base_name: String,

    /// Name as spelled in the compared codebase.
    #[serde(default)]
    pub target_name: String,
}

/// Response from `POST /qai/v1/scanner/diff`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DiffResult {
    /// Label of the reference side.
    #[serde(default)]
    pub base: String,

    /// Label of the compared side.
    #[serde(default)]
    pub target: String,

    /// Types in the reference but not the target — real gaps.
    #[serde(default, deserialize_with = "null_as_default")]
    pub missing_types: Vec<String>,

    /// Types in the target but not the reference.
    #[serde(default, deserialize_with = "null_as_default")]
    pub extra_types: Vec<String>,

    /// Types that exist on both sides but differ only in casing — not gaps.
    #[serde(default, deserialize_with = "null_as_default")]
    pub convention_diffs: Vec<ConventionDiff>,

    /// Type name → field names missing from the target.
    #[serde(default, deserialize_with = "null_as_default")]
    pub missing_fields: HashMap<String, Vec<String>>,

    /// Functions in the reference but not the target.
    #[serde(default, deserialize_with = "null_as_default")]
    pub missing_functions: Vec<String>,

    /// Fraction of the reference surface present in the target, 0.0–1.0.
    #[serde(default)]
    pub completion: f64,

    /// Total count of missing types, fields, and functions.
    #[serde(default)]
    pub total_gaps: i64,
}

// ── Verify ──────────────────────────────────────────────────────────────────

/// An expected type in a [`Blueprint`].
#[derive(Debug, Clone, Serialize, Default)]
pub struct BlueprintType {
    /// Expected type name.
    pub name: String,

    /// Expected declaration kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Expected fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<CodeField>>,
}

/// An expected function in a [`Blueprint`].
#[derive(Debug, Clone, Serialize, Default)]
pub struct BlueprintFunction {
    /// Expected function name.
    pub name: String,

    /// Expected parameter list, as source text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<String>,

    /// Expected return type, as source text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
}

/// An expected file in a [`Blueprint`].
#[derive(Debug, Clone, Serialize, Default)]
pub struct BlueprintModule {
    /// Expected path, relative to the scan root.
    pub path: String,
}

/// The structure a codebase is expected to have.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Blueprint {
    /// Label for the blueprint.
    pub name: String,

    /// Expected types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<BlueprintType>>,

    /// Expected functions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<BlueprintFunction>>,

    /// Expected modules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<BlueprintModule>>,
}

/// Request body for `POST /qai/v1/scanner/verify`.
///
/// Provide either `source` (scanned fresh) or `scan_id` (an existing scan).
#[derive(Debug, Clone, Serialize, Default)]
pub struct VerifyRequest {
    /// The expected structure.
    pub blueprint: Blueprint,

    /// Codebase to verify — a GitHub URL, or a directory on the gateway's own
    /// filesystem under `/workspace` or `/tmp` (any other local path is 403
    /// `forbidden`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Git branch for `source`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Existing scan to verify against, instead of `source`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_id: Option<String>,
}

/// Whether one expected module exists and is complete.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FileStatus {
    /// Expected path.
    #[serde(default)]
    pub path: String,

    /// Whether the file was found.
    #[serde(default)]
    pub exists: bool,

    /// Whether every symbol the blueprint expects in it was found.
    #[serde(default)]
    pub complete: bool,
}

/// Response from `POST /qai/v1/scanner/verify`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct VerifyResult {
    /// True when nothing the blueprint expects is missing.
    #[serde(default)]
    pub passed: bool,

    /// Fraction of the blueprint present, 0.0–1.0.
    #[serde(default)]
    pub completion: f64,

    /// Expected types that were not found.
    #[serde(default, deserialize_with = "null_as_default")]
    pub missing_types: Vec<String>,

    /// Type name → expected field names that were not found.
    #[serde(default, deserialize_with = "null_as_default")]
    pub missing_fields: HashMap<String, Vec<String>>,

    /// Expected functions that were not found.
    #[serde(default, deserialize_with = "null_as_default")]
    pub missing_functions: Vec<String>,

    /// Expected modules that were not found.
    #[serde(default, deserialize_with = "null_as_default")]
    pub missing_modules: Vec<String>,

    /// Per-module existence and completeness.
    #[serde(default, deserialize_with = "null_as_default")]
    pub file_status: Vec<FileStatus>,
}

// ── Audit ───────────────────────────────────────────────────────────────────

/// Response from `POST /qai/v1/scanner/audit` — the audit runs asynchronously,
/// so this is the accepted job, not the findings.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditJobResponse {
    /// Job id to poll with
    /// [`Client::get_job`](crate::Client::get_job).
    #[serde(default)]
    pub job_id: String,

    /// Audit profile that was applied.
    #[serde(default)]
    pub profile: String,

    /// Model the analysis runs on.
    #[serde(default)]
    pub model: String,

    /// Number of source files that survived filtering and will be analysed.
    #[serde(default)]
    pub files_to_analyze: i64,

    /// Pre-flight cost estimate, pre-formatted for display.
    #[serde(default)]
    pub estimated_cost: String,
}

/// One file's findings from a completed audit job.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditFileResult {
    /// Path relative to the uploaded root.
    #[serde(default)]
    pub path: String,

    /// Detected language.
    #[serde(default)]
    pub language: String,

    /// Input tokens spent on this file.
    #[serde(default)]
    pub tokens_in: i64,

    /// Output tokens produced for this file.
    #[serde(default)]
    pub tokens_out: i64,

    /// The model's findings for this file.
    #[serde(default)]
    pub findings: String,

    /// Why this file could not be analysed, when it could not be.
    #[serde(default)]
    pub error: String,
}

/// The manifest a completed audit job produces.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditResult {
    /// Audit profile that was applied.
    #[serde(default)]
    pub profile: String,

    /// Model the analysis ran on.
    #[serde(default)]
    pub model: String,

    /// Files successfully analysed.
    #[serde(default)]
    pub files_analyzed: i64,

    /// Files that errored.
    #[serde(default)]
    pub files_errored: i64,

    /// Total input tokens across all files.
    #[serde(default)]
    pub total_tokens_in: i64,

    /// Total output tokens across all files.
    #[serde(default)]
    pub total_tokens_out: i64,

    /// Wall-clock duration of the audit.
    #[serde(default)]
    pub duration_seconds: f64,

    /// Actual cost of the audit in USD.
    #[serde(default)]
    pub cost_usd: f64,

    /// Per-file findings.
    #[serde(default, deserialize_with = "null_as_default")]
    pub files: Vec<AuditFileResult>,
}

// ── Vulnerabilities ─────────────────────────────────────────────────────────

/// Options for `POST /qai/v1/scanner/vulnerabilities`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VulnerabilityScanOptions {
    /// Also produce a threat model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threat_model: Option<bool>,

    /// Also cross-reference dependencies against known CVEs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cve_check: Option<bool>,
}

/// Request body for `POST /qai/v1/scanner/vulnerabilities`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VulnerabilityScanRequest {
    /// GitHub URL to scan. Local paths are rejected — the handler only accepts
    /// `https://github.com/owner/repo`.
    pub source: String,

    /// Label for the scan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Detector options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<VulnerabilityScanOptions>,
}

// ── Client methods ──────────────────────────────────────────────────────────

impl Client {
    /// Scans a codebase or an OpenAPI spec into a structural graph.
    ///
    /// `POST /qai/v1/scanner/scan`
    pub async fn scanner_scan(&self, req: &ScanRequest) -> Result<ScanResult> {
        let (resp, _meta) = self
            .post_json::<ScanRequest, ScanResult>("/qai/v1/scanner/scan", req)
            .await?;
        Ok(resp)
    }

    /// Scans an uploaded source archive.
    ///
    /// `archive` is a tar.gz of the tree (50 MB cap). `languages` restricts
    /// parsing when non-empty.
    ///
    /// `POST /qai/v1/scanner/upload` (multipart: `file`, `name`, `languages`)
    pub async fn scanner_upload(
        &self,
        name: &str,
        archive: Vec<u8>,
        languages: &[String],
    ) -> Result<ScanResult> {
        let part = reqwest::multipart::Part::bytes(archive)
            .file_name(format!("{name}.tar.gz"))
            .mime_str("application/gzip")
            .map_err(multipart_error)?;
        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("name", name.to_string());
        if !languages.is_empty() {
            form = form.text("languages", languages.join(","));
        }
        let (resp, _meta) = self
            .post_multipart::<ScanResult>("/qai/v1/scanner/upload", form)
            .await?;
        Ok(resp)
    }

    /// Compares two codebases structurally and reports the gaps.
    ///
    /// `POST /qai/v1/scanner/diff`
    pub async fn scanner_diff(&self, req: &DiffRequest) -> Result<DiffResult> {
        let (resp, _meta) = self
            .post_json::<DiffRequest, DiffResult>("/qai/v1/scanner/diff", req)
            .await?;
        Ok(resp)
    }

    /// Verifies a codebase against a blueprint of the structure it should have.
    ///
    /// `POST /qai/v1/scanner/verify`
    pub async fn scanner_verify(&self, req: &VerifyRequest) -> Result<VerifyResult> {
        let (resp, _meta) = self
            .post_json::<VerifyRequest, VerifyResult>("/qai/v1/scanner/verify", req)
            .await?;
        Ok(resp)
    }

    /// Starts an LLM code audit over an uploaded source archive.
    ///
    /// Returns as soon as the job is accepted; the analysis runs in the
    /// background. `profile` defaults to `security-redteam` and `model` to the
    /// gateway's default audit model when either is `None`.
    ///
    /// `POST /qai/v1/scanner/audit` (multipart: `file`, `model`, `profile`)
    pub async fn scanner_audit(
        &self,
        archive: Vec<u8>,
        profile: Option<&str>,
        model: Option<&str>,
    ) -> Result<AuditJobResponse> {
        let part = reqwest::multipart::Part::bytes(archive)
            .file_name("audit.tar.gz".to_string())
            .mime_str("application/gzip")
            .map_err(multipart_error)?;
        let mut form = reqwest::multipart::Form::new().part("file", part);
        if let Some(profile) = profile {
            form = form.text("profile", profile.to_string());
        }
        if let Some(model) = model {
            form = form.text("model", model.to_string());
        }
        let (resp, _meta) = self
            .post_multipart::<AuditJobResponse>("/qai/v1/scanner/audit", form)
            .await?;
        Ok(resp)
    }

    /// Runs the security detector over a public GitHub repository.
    ///
    /// The findings come straight from the detector, so they are returned
    /// untyped.
    ///
    /// `POST /qai/v1/scanner/vulnerabilities`
    pub async fn scanner_vulnerabilities(
        &self,
        req: &VulnerabilityScanRequest,
    ) -> Result<serde_json::Value> {
        let (resp, _meta) = self
            .post_json::<VulnerabilityScanRequest, serde_json::Value>(
                "/qai/v1/scanner/vulnerabilities",
                req,
            )
            .await?;
        Ok(resp)
    }

    /// Lists the caller's saved scans, newest first.
    ///
    /// `GET /qai/v1/scanner/scans`
    pub async fn scanner_scans(&self) -> Result<ScanListResponse> {
        let (resp, _meta) = self
            .get_json::<ScanListResponse>("/qai/v1/scanner/scans")
            .await?;
        Ok(resp)
    }

    /// Fetches one saved scan with its full graph.
    ///
    /// `GET /qai/v1/scanner/scans/{id}`
    pub async fn scanner_scan_get(&self, scan_id: &str) -> Result<ScanResult> {
        let (resp, _meta) = self
            .get_json::<ScanResult>(&format!("/qai/v1/scanner/scans/{scan_id}"))
            .await?;
        Ok(resp)
    }

    /// Deletes a saved scan.
    ///
    /// `DELETE /qai/v1/scanner/scans/{id}`
    pub async fn scanner_scan_delete(&self, scan_id: &str) -> Result<ScanDeleteResponse> {
        let (resp, _meta) = self
            .delete_json::<ScanDeleteResponse>(&format!("/qai/v1/scanner/scans/{scan_id}"))
            .await?;
        Ok(resp)
    }

    /// Lists every type in a scan by name and kind — the discovery step before
    /// [`Client::scanner_type`].
    ///
    /// `GET /qai/v1/scanner/scans/{id}/types`
    pub async fn scanner_types(&self, scan_id: &str) -> Result<ScanTypeListResponse> {
        let (resp, _meta) = self
            .get_json::<ScanTypeListResponse>(&format!("/qai/v1/scanner/scans/{scan_id}/types"))
            .await?;
        Ok(resp)
    }

    /// Fetches one type from a scan with its fields, methods, and the types it
    /// references. The name match is case-insensitive.
    ///
    /// `GET /qai/v1/scanner/scans/{id}/types/{name}`
    pub async fn scanner_type(&self, scan_id: &str, type_name: &str) -> Result<ScanTypeDetail> {
        let encoded = urlencoding::encode(type_name);
        let (resp, _meta) = self
            .get_json::<ScanTypeDetail>(&format!("/qai/v1/scanner/scans/{scan_id}/types/{encoded}"))
            .await?;
        Ok(resp)
    }

    /// Renders a scan's graph as SVG. Returns the raw document.
    ///
    /// `GET /qai/v1/scanner/scans/{id}/graph.svg`
    pub async fn scanner_graph_svg(&self, scan_id: &str) -> Result<String> {
        let (resp, _meta) = self
            .get_stream_raw(&format!("/qai/v1/scanner/scans/{scan_id}/graph.svg"))
            .await?;
        Ok(resp.text().await?)
    }
}

/// Wraps a multipart construction failure in the SDK's error envelope.
fn multipart_error(e: reqwest::Error) -> Error {
    Error::Api(ApiError {
        status_code: 0,
        code: "multipart_error".into(),
        message: e.to_string(),
        request_id: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_request_renames_scan_ids_to_base_and_target() {
        let req = DiffRequest {
            base_scan_id: Some("scan_a".into()),
            target_scan_id: Some("scan_b".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["base"], "scan_a");
        assert_eq!(json["target"], "scan_b");
        assert!(json.get("base_scan_id").is_none());
        assert!(json.get("base_graph").is_none());
    }

    #[test]
    fn diff_result_decodes_missing_field_map() {
        let result: DiffResult = serde_json::from_str(
            r#"{"base":"rust","target":"go","missing_types":["ChatUsage"],
                "extra_types":null,"missing_fields":{"ChatRequest":["region"]},
                "completion":0.75,"total_gaps":2}"#,
        )
        .expect("decode");
        assert_eq!(result.missing_types, vec!["ChatUsage"]);
        assert!(result.extra_types.is_empty());
        assert_eq!(
            result.missing_fields.get("ChatRequest"),
            Some(&vec!["region".to_string()])
        );
        assert_eq!(result.total_gaps, 2);
    }

    #[test]
    fn type_detail_reads_the_reserved_type_key() {
        let detail: ScanTypeDetail = serde_json::from_str(
            r#"{"type":{"name":"ChatRequest","kind":"struct","file":"src/chat.rs",
                        "fields":[{"name":"model","type":"String","optional":false}]},
                "methods":null,"references":null,"scan_id":"s1","scan_name":"rust"}"#,
        )
        .expect("decode");
        assert_eq!(detail.code_type.name, "ChatRequest");
        assert_eq!(detail.code_type.fields[0].r#type, "String");
        assert!(detail.methods.is_empty());
    }

    #[test]
    fn scan_request_omits_unset_options() {
        let req = ScanRequest {
            source: "https://github.com/quantum-encoding/quantum-sdk-rs".into(),
            include_call_graph: Some(true),
            ..Default::default()
        };
        let json = serde_json::to_value(&req).expect("serialize");
        assert_eq!(json["include_call_graph"], true);
        assert!(json.get("branch").is_none());
        assert!(json.get("languages").is_none());
    }
}
