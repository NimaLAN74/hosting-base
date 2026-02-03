//! Models for Phase 4: controls, requirements, evidence (COMP-AI-MULTIFRAMEWORK-SUPPORT).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

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
    pub due_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create or update remediation (assignee, due_date, status, notes).
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertRemediationRequest {
    pub assigned_to: Option<String>,
    pub due_date: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}
