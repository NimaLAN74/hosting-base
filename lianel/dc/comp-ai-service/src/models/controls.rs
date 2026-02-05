//! Models for Phase 4: controls, requirements, evidence (COMP-AI-MULTIFRAMEWORK-SUPPORT).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Control {
    pub id: i64,
    pub internal_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Requirement {
    pub id: i64,
    pub framework_id: i64,
    pub code: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FrameworkRow {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub version: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ControlWithRequirements {
    pub id: i64,
    pub internal_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub requirements: Vec<RequirementRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RequirementRef {
    pub code: String,
    pub title: Option<String>,
    pub framework_slug: String,
}

/// Requirement as returned by GET /api/v1/requirements (Phase 5 audit view).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RequirementListItem {
    pub id: i64,
    pub framework_slug: String,
    pub code: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EvidenceItem {
    pub id: i64,
    pub control_id: i64,
    pub r#type: String,
    pub source: Option<String>,
    pub description: Option<String>,
    pub link_url: Option<String>,
    pub collected_at: DateTime<Utc>,
    pub created_by: Option<String>,
    /// Phase B: path under evidence storage (uploaded file).
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    /// Extracted text for AI analysis (set after upload or analyse).
    pub extracted_text: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEvidenceRequest {
    pub control_id: i64,
    pub r#type: String,
    pub source: Option<String>,
    pub description: Option<String>,
    pub link_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateEvidenceResponse {
    pub id: i64,
    pub control_id: i64,
    pub collected_at: DateTime<Utc>,
}

/// Request to collect evidence from GitHub (last commit or branch protection).
#[derive(Debug, Deserialize, ToSchema)]
pub struct GitHubEvidenceRequest {
    pub control_id: i64,
    pub owner: String,
    pub repo: String,
    /// "last_commit" or "branch_protection"
    pub evidence_type: String,
}

/// One control with requirements and evidence for audit export (Phase 5).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ControlExportEntry {
    pub control: ControlWithRequirements,
    pub evidence: Vec<EvidenceItem>,
}

/// Full audit export payload (Phase 5).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditExport {
    pub exported_at: DateTime<Utc>,
    pub controls: Vec<ControlExportEntry>,
}

/// Remediation task for a control (Phase 5 – gaps workflow).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RemediationTask {
    pub id: i64,
    pub control_id: i64,
    pub assigned_to: Option<String>,
    #[serde(with = "crate::utils::serde_eu_date::option")]
    pub due_date: Option<NaiveDate>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create or update remediation (assignee, due_date, status, notes).
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertRemediationRequest {
    pub assigned_to: Option<String>,
    #[schema(example = "31/12/2026")]
    pub due_date: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

/// Automated test per control (Phase 6B).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ControlTest {
    pub id: i64,
    pub control_id: i64,
    pub name: String,
    pub test_type: String,
    pub schedule: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_result: Option<String>,
    pub last_details: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Record a test run result (POST body).
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordTestResultRequest {
    pub result: String,
    pub details: Option<String>,
}

/// Optional context for AI remediation suggestion (Phase 6C).
#[derive(Debug, Deserialize, ToSchema)]
pub struct RemediationSuggestRequest {
    pub context: Option<String>,
}

/// AI-generated remediation suggestion (Phase 6C).
#[derive(Debug, Serialize, ToSchema)]
pub struct RemediationSuggestResponse {
    pub suggestion: String,
    pub model_used: String,
}

/// Request for AI gap/risk analysis (Phase 7.2). Optional framework filter.
#[derive(Debug, Deserialize, ToSchema)]
pub struct GapAnalysisRequest {
    pub framework: Option<String>,
}

/// AI-generated gap analysis summary (Phase 7.2).
#[derive(Debug, Serialize, ToSchema)]
pub struct GapAnalysisResponse {
    pub summary: String,
    pub model_used: String,
}

/// Response for document analyse (Phase B).
#[derive(Debug, Serialize, ToSchema)]
pub struct EvidenceAnalyzeResponse {
    pub summary: String,
    pub suggested_control_ids: Option<Vec<i64>>,
    pub gaps: Option<Vec<String>>,
    pub model_used: String,
}
