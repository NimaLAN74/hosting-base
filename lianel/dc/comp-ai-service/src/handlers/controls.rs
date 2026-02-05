//! Phase 4: Controls and evidence API handlers.

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use std::sync::Arc;
use sqlx::PgPool;
use crate::config::AppConfig;
use crate::auth::extract_user;
use crate::models::{
    Control, ControlWithRequirements, EvidenceItem, CreateEvidenceRequest, CreateEvidenceResponse,
    GitHubEvidenceRequest, ControlExportEntry, AuditExport, RemediationTask, UpsertRemediationRequest,
    RequirementListItem, ControlTest, RecordTestResultRequest,
    RemediationSuggestRequest, RemediationSuggestResponse,
    GapAnalysisRequest, GapAnalysisResponse, EvidenceAnalyzeResponse, EvidenceReviewResponse,
    ScanDocumentsRequest, ScanDocumentsResponse,
    ControlPolicyMappingResponse, ControlPolicyMappingEntry, PolicyRef, SystemDescriptionResponse,
    PatchControlRequest, BulkExternalIdRequest, BulkExternalIdResponse, OktaEvidenceRequest, AwsEvidenceRequest,
    M365EvidenceRequest, M365EvidenceResponse, DlpEvidenceRequest,
    DriveEvidenceRequest, DriveEvidenceResponse,
    SharepointEvidenceRequest, SharepointEvidenceResponse,
};
use crate::db::queries::{
    list_controls, list_controls_filtered, get_control_with_requirements, list_evidence, create_evidence, get_evidence_by_id,
    list_controls_without_evidence, list_remediation_tasks, get_remediation_by_control_id,
    upsert_remediation, list_requirements, update_control_external_id,
    list_control_tests, list_all_control_tests, record_control_test_result,
};
use crate::inference;
use crate::utils::{format_eu_date, parse_eu_date};
use crate::integrations::github::{fetch_last_commit_evidence, fetch_branch_protection_evidence};
use crate::integrations::okta::{
    fetch_okta_org_summary, fetch_okta_users_snapshot, fetch_okta_groups_snapshot,
};
use crate::integrations::aws::fetch_aws_iam_summary;
use crate::integrations::m365::fetch_m365_email_list;
use crate::integrations::drive::fetch_drive_file_list;
use crate::integrations::sharepoint::fetch_sharepoint_file_list;
use chrono::Utc;
use axum_extra::extract::Multipart;
use uuid::Uuid;
use crate::handlers::comp_ai::ResponseCache;

type AppState = (Arc<AppConfig>, PgPool, ResponseCache);

#[utoipa::path(
    get,
    path = "/api/v1/requirements",
    tag = "controls",
    params(("framework" = Option<String>, Query, description = "Filter by framework slug (e.g. soc2, iso27001)")),
    responses(
        (status = 200, description = "List of framework requirements", body = Vec<RequirementListItem>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_requirements(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Query(q): Query<RequirementsQuery>,
) -> Result<Json<Vec<RequirementListItem>>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let framework = q.framework.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let requirements = list_requirements(pool, framework)
        .await
        .map_err(|e| {
            tracing::error!("list_requirements failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list requirements"})),
            )
        })?;
    Ok(Json(requirements))
}

#[derive(Debug, serde::Deserialize)]
pub struct RequirementsQuery {
    pub framework: Option<String>,
}

/// C5: Optional category filter for GET /api/v1/controls (e.g. operational, administrative).
#[derive(Debug, serde::Deserialize)]
pub struct ControlsQuery {
    pub category: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/controls",
    tag = "controls",
    params(("category" = Option<String>, Query, description = "C5: Filter by category (e.g. operational, administrative)")),
    responses(
        (status = 200, description = "List of controls", body = Vec<Control>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_controls(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Query(q): Query<ControlsQuery>,
) -> Result<Json<Vec<Control>>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let controls = list_controls_filtered(pool, q.category.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("list_controls_filtered failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list controls"})),
            )
        })?;

    Ok(Json(controls))
}

#[utoipa::path(
    get,
    path = "/api/v1/controls/{id}",
    tag = "controls",
    params(("id" = i64, Path, description = "Control ID")),
    responses(
        (status = 200, description = "Control with requirement mappings", body = ControlWithRequirements),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Control not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_control(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ControlWithRequirements>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let control = get_control_with_requirements(pool, id)
        .await
        .map_err(|e| {
            tracing::error!("get_control_with_requirements failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get control"})),
            )
        })?;

    let Some(c) = control else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Control not found"})),
        ));
    };

    Ok(Json(c))
}

#[utoipa::path(
    patch,
    path = "/api/v1/controls/{id}",
    tag = "controls",
    params(("id" = i64, Path, description = "Control ID")),
    request_body = PatchControlRequest,
    responses(
        (status = 200, description = "Control updated (external_id)", body = ControlWithRequirements),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Control not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn patch_control(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<PatchControlRequest>,
) -> Result<Json<ControlWithRequirements>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let external_id = body.external_id.as_deref();
    let updated = update_control_external_id(pool, id, external_id)
        .await
        .map_err(|e| {
            tracing::error!("update_control_external_id failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to update control"})),
            )
        })?;

    let Some(_) = updated else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Control not found"})),
        ));
    };

    let control = get_control_with_requirements(pool, id)
        .await
        .map_err(|e| {
            tracing::error!("get_control_with_requirements failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get control"})),
            )
        })?;

    let Some(c) = control else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Control not found"})),
        ));
    };

    Ok(Json(c))
}

/// G8: Bulk set external_id on controls by internal_id (align with Vanta or other control sets).
#[utoipa::path(
    post,
    path = "/api/v1/controls/bulk-external-id",
    tag = "controls",
    request_body = BulkExternalIdRequest,
    responses(
        (status = 200, description = "Bulk update result", body = BulkExternalIdResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_bulk_external_id(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<BulkExternalIdRequest>,
) -> Result<Json<BulkExternalIdResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let controls = list_controls(pool)
        .await
        .map_err(|e| {
            tracing::error!("list_controls (bulk external_id) failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list controls"})),
            )
        })?;

    let by_internal_id: std::collections::HashMap<&str, i64> = controls
        .iter()
        .map(|c| (c.internal_id.as_str(), c.id))
        .collect();

    let mut updated = 0u32;
    let mut not_found = Vec::new();
    for item in &body.updates {
        let internal_id = item.internal_id.trim();
        if internal_id.is_empty() {
            continue;
        }
        match by_internal_id.get(internal_id) {
            Some(&id) => {
                let ext = item.external_id.as_deref();
                if let Ok(Some(_)) = update_control_external_id(pool, id, ext).await {
                    updated += 1;
                }
            }
            None => not_found.push(item.internal_id.clone()),
        }
    }

    Ok(Json(BulkExternalIdResponse { updated, not_found }))
}

#[derive(Debug, serde::Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    /// Filter audit export by framework slug (e.g. soc2, iso27001). Only controls mapped to this framework and only those requirement codes are included.
    pub framework: Option<String>,
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/controls/export",
    tag = "controls",
    params(
        ("format" = Option<String>, Query, description = "csv for CSV export, omit for JSON"),
        ("framework" = Option<String>, Query, description = "Filter by framework slug (e.g. soc2, iso27001) for audit view")
    ),
    responses(
        (status = 200, description = "Audit export (controls + requirements + evidence) as JSON or CSV"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_controls_export(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Query(q): Query<ExportQuery>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let controls = list_controls(pool)
        .await
        .map_err(|e| {
            tracing::error!("list_controls (export) failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list controls"})),
            )
        })?;

    let mut entries = Vec::with_capacity(controls.len());
    for c in &controls {
        let control_detail = get_control_with_requirements(pool, c.id)
            .await
            .map_err(|e| {
                tracing::error!("get_control_with_requirements (export) failed: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to build export"})),
                )
            })?;
        let evidence = list_evidence(pool, Some(c.id), 500, 0)
            .await
            .map_err(|e| {
                tracing::error!("list_evidence (export) failed: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to build export"})),
                )
            })?;
        let control_with_req = control_detail.unwrap_or_else(|| ControlWithRequirements {
            id: c.id,
            internal_id: c.internal_id.clone(),
            name: c.name.clone(),
            description: c.description.clone(),
            category: c.category.clone(),
            external_id: c.external_id.clone(),
            created_at: c.created_at,
            requirements: vec![],
        });
        entries.push(ControlExportEntry {
            control: control_with_req,
            evidence,
        });
    }

    // Phase 5: filter by framework if requested (audit view per framework)
    let framework_slug = q.framework.as_deref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
    let entries: Vec<ControlExportEntry> = if let Some(ref slug) = framework_slug {
        entries
            .into_iter()
            .map(|mut e| {
                e.control.requirements.retain(|r| r.framework_slug.eq_ignore_ascii_case(slug));
                e
            })
            .filter(|e| !e.control.requirements.is_empty())
            .collect()
    } else {
        entries
    };

    let export = AuditExport {
        exported_at: Utc::now(),
        controls: entries,
    };

    let want_csv = q
        .format
        .as_deref()
        .map(|s| s.trim().eq_ignore_ascii_case("csv"))
        .unwrap_or(false);

    if want_csv {
        let mut csv = String::from("control_id,internal_id,name,description,requirement_codes,evidence_count,evidence_types\n");
        for e in &export.controls {
            let req_codes: Vec<String> = e
                .control
                .requirements
                .iter()
                .map(|r| format!("{}:{}", r.framework_slug, r.code))
                .collect();
            let req_codes = req_codes.join("|");
            let ev_count = e.evidence.len();
            let ev_types: Vec<&str> = e.evidence.iter().map(|ev| ev.r#type.as_str()).collect();
            let ev_types = ev_types.join("|");
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                e.control.id,
                escape_csv(&e.control.internal_id),
                escape_csv(&e.control.name),
                escape_csv(e.control.description.as_deref().unwrap_or("")),
                escape_csv(&req_codes),
                ev_count,
                escape_csv(&ev_types),
            ));
        }
        let filename_date = format_eu_date(export.exported_at.date_naive()).replace('/', "-");
        let filename = format!("comp-ai-audit-export-{}.csv", filename_date);
        let body = axum::body::Body::from(csv);
        let res = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            )
            .body(body)
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to build response"})),
                )
            })?;
        Ok(res)
    } else {
        Ok(Json(export).into_response())
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/controls/gaps",
    tag = "controls",
    responses(
        (status = 200, description = "Controls with no evidence (gaps)", body = Vec<Control>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_controls_gaps(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<Vec<Control>>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let gaps = list_controls_without_evidence(pool)
        .await
        .map_err(|e| {
            tracing::error!("list_controls_without_evidence failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list gaps"})),
            )
        })?;

    Ok(Json(gaps))
}

// --- Phase 7.2: AI gap/risk analysis ---

#[utoipa::path(
    post,
    path = "/api/v1/analysis/gaps",
    tag = "controls",
    request_body = GapAnalysisRequest,
    responses(
        (status = 200, description = "AI-generated gap analysis summary", body = GapAnalysisResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "AI model not available"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_analysis_gaps(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<GapAnalysisRequest>,
) -> Result<Json<GapAnalysisResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let gaps = list_controls_without_evidence(pool)
        .await
        .map_err(|e| {
            tracing::error!("list_controls_without_evidence failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list gaps"})),
            )
        })?;

    let framework_hint = body
        .framework
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!(" (focus on framework: {})", s))
        .unwrap_or_else(|| "".to_string());

    let gap_list: Vec<String> = gaps
        .iter()
        .map(|c| {
            format!(
                "- {} [{}]: {}",
                c.internal_id,
                c.name,
                c.description.as_deref().unwrap_or("(no description)")
            )
        })
        .collect();

    let system_prompt = format!(
        "You are a compliance and risk expert. The following controls have no evidence yet (gaps).{}\n\n\
         List of controls with no evidence ({} total):\n{}\n\n\
         Provide a short risk analysis: (1) suggest a prioritisation order (which gaps to address first and why), \
         (2) typical evidence types to collect for each, and (3) suggested order of work. Be concise and actionable.",
        framework_hint,
        gaps.len(),
        gap_list.join("\n")
    );

    let user_prompt = "Summarise prioritisation, evidence types, and suggested order of work for closing these gaps.";

    let (summary, model_used) = if let (Some(ref url), Some(ref model)) =
        (&config.comp_ai_ollama_url, &config.comp_ai_ollama_model)
    {
        match inference::generate(
            url,
            model,
            user_prompt,
            config.comp_ai_max_tokens,
            config.comp_ai_temperature,
            Some(&system_prompt),
        )
        .await
        {
            Ok((text, _)) => (text.trim().to_string(), model.clone()),
            Err(e) => {
                tracing::error!("Ollama gap analysis failed: {}", e);
                if config.comp_ai_ollama_fallback_to_mock {
                    (
                        format!(
                            "Prioritise closing {} gap(s) by control criticality and audit timeline. \
                             Collect evidence types: policy docs, screenshots, logs, or attestations as required by each control.",
                            gaps.len()
                        ),
                        "mock-model (Ollama unavailable)".to_string(),
                    )
                } else {
                    return Err((
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(serde_json::json!({
                            "error": "AI model unavailable",
                            "detail": e.to_string()
                        })),
                    ));
                }
            }
        }
    } else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "AI model not configured",
                "detail": "Set COMP_AI_OLLAMA_URL and COMP_AI_OLLAMA_MODEL for gap analysis"
            })),
        ));
    };

    Ok(Json(GapAnalysisResponse {
        summary: summary,
        model_used,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct RemediationQuery {
    pub control_id: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/remediation",
    tag = "controls",
    params(("control_id" = Option<i64>, Query, description = "Filter by control ID")),
    responses(
        (status = 200, description = "List of remediation tasks", body = Vec<RemediationTask>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_remediation(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Query(q): Query<RemediationQuery>,
) -> Result<Json<Vec<RemediationTask>>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let tasks = list_remediation_tasks(pool, q.control_id)
        .await
        .map_err(|e| {
            tracing::error!("list_remediation_tasks failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list remediation tasks"})),
            )
        })?;

    Ok(Json(tasks))
}

#[utoipa::path(
    get,
    path = "/api/v1/controls/{id}/remediation",
    tag = "controls",
    params(("id" = i64, Path, description = "Control ID")),
    responses(
        (status = 200, description = "Remediation task for control, or null if none", body = Option<RemediationTask>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_control_remediation(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Option<RemediationTask>>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let task = get_remediation_by_control_id(pool, id)
        .await
        .map_err(|e| {
            tracing::error!("get_remediation_by_control_id failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get remediation"})),
            )
        })?;

    Ok(Json(task))
}

#[utoipa::path(
    put,
    path = "/api/v1/controls/{id}/remediation",
    tag = "controls",
    params(("id" = i64, Path, description = "Control ID")),
    request_body = UpsertRemediationRequest,
    responses(
        (status = 200, description = "Remediation task created or updated", body = RemediationTask),
        (status = 400, description = "Invalid request (e.g. invalid due_date or status)"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn put_control_remediation(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpsertRemediationRequest>,
) -> Result<Json<RemediationTask>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let due_date = body
        .due_date
        .as_deref()
        .and_then(parse_eu_date);
    let status = body
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let valid_statuses = ["open", "in_progress", "done"];
    if let Some(s) = status {
        if !valid_statuses.contains(&s) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid status",
                    "detail": "Use open, in_progress, or done"
                })),
            ));
        }
    }

    let task = upsert_remediation(
        pool,
        id,
        body.assigned_to.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        due_date,
        status,
        body.notes.as_deref().map(str::trim).filter(|s| !s.is_empty()),
    )
    .await
    .map_err(|e| {
        tracing::error!("upsert_remediation failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save remediation task"})),
        )
    })?;

    Ok(Json(task))
}

// --- Phase 6C: AI remediation suggestion ---

#[utoipa::path(
    post,
    path = "/api/v1/controls/{id}/remediation/suggest",
    tag = "controls",
    params(("id" = i64, Path, description = "Control ID")),
    request_body = RemediationSuggestRequest,
    responses(
        (status = 200, description = "AI-generated remediation suggestion", body = RemediationSuggestResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Control not found"),
        (status = 503, description = "AI model not available (Ollama not configured or failed)"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_remediation_suggest(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<RemediationSuggestRequest>,
) -> Result<Json<RemediationSuggestResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let control = get_control_with_requirements(pool, id)
        .await
        .map_err(|e| {
            tracing::error!("get_control_with_requirements failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get control"})),
            )
        })?;

    let Some(c) = control else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Control not found"})),
        ));
    };

    let remediation = get_remediation_by_control_id(pool, id)
        .await
        .map_err(|e| {
            tracing::error!("get_remediation_by_control_id failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get remediation"})),
            )
        })?;

    let req_codes: Vec<String> = c
        .requirements
        .iter()
        .map(|r| format!("{} ({})", r.code, r.framework_slug))
        .collect();
    let system_prompt = format!(
        "You are a compliance and security expert. Suggest concrete remediation steps for the following control.\n\n\
         Control: {} [{}]\nDescription: {}\n\
         Mapped framework requirements: {}\n\
         Current remediation (if any): {}",
        c.name,
        c.internal_id,
        c.description.as_deref().unwrap_or("(none)"),
        req_codes.join(", "),
        remediation
            .as_ref()
            .map(|t| {
                format!(
                    "status={}, assigned_to={}, due_date={}, notes={}",
                    t.status,
                    t.assigned_to.as_deref().unwrap_or("(unset)"),
                    t.due_date
                        .map(|d| format_eu_date(d))
                        .unwrap_or_else(|| "(unset)".to_string()),
                    t.notes.as_deref().unwrap_or("(none)")
                )
            })
            .unwrap_or_else(|| "(no remediation task yet)".to_string()),
    );

    let user_prompt = match body.context.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(ctx) => format!(
            "Suggest concrete remediation steps (assignee, due date, and actionable steps). Additional context: {}",
            ctx
        ),
        None => "Suggest concrete remediation steps (assignee, due date, and actionable steps).".to_string(),
    };

    let (suggestion, model_used) = if let (Some(ref url), Some(ref model)) =
        (&config.comp_ai_ollama_url, &config.comp_ai_ollama_model)
    {
        match inference::generate(
            url,
            model,
            &user_prompt,
            config.comp_ai_max_tokens,
            config.comp_ai_temperature,
            Some(&system_prompt),
        )
        .await
        {
            Ok((text, _)) => (text.trim().to_string(), model.clone()),
            Err(e) => {
                tracing::error!("Ollama remediation suggest failed: {}", e);
                if config.comp_ai_ollama_fallback_to_mock {
                    (
                        format!(
                            "1. Assign an owner for this control.\n2. Set a due date (e.g. 30 days).\n3. Implement evidence collection for: {}.",
                            req_codes.join(", ")
                        ),
                        "mock-model (Ollama unavailable)".to_string(),
                    )
                } else {
                    return Err((
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(serde_json::json!({
                            "error": "AI model unavailable",
                            "detail": e.to_string()
                        })),
                    ));
                }
            }
        }
    } else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "AI model not configured",
                "detail": "Set COMP_AI_OLLAMA_URL and COMP_AI_OLLAMA_MODEL for remediation suggestions"
            })),
        ));
    };

    Ok(Json(RemediationSuggestResponse {
        suggestion,
        model_used,
    }))
}

// --- Phase 6B: control tests ---

#[utoipa::path(
    get,
    path = "/api/v1/controls/{id}/tests",
    tag = "controls",
    params(("id" = i64, Path, description = "Control ID")),
    responses(
        (status = 200, description = "List of automated tests for the control", body = Vec<ControlTest>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_control_tests(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ControlTest>>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let tests = list_control_tests(pool, id)
        .await
        .map_err(|e| {
            tracing::error!("list_control_tests failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list control tests"})),
            )
        })?;

    Ok(Json(tests))
}

#[derive(Debug, serde::Deserialize)]
pub struct TestsQuery {
    pub control_id: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tests",
    tag = "controls",
    params(("control_id" = Option<i64>, Query, description = "Filter by control ID")),
    responses(
        (status = 200, description = "List of all control tests", body = Vec<ControlTest>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_tests(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Query(q): Query<TestsQuery>,
) -> Result<Json<Vec<ControlTest>>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let tests = list_all_control_tests(pool, q.control_id)
        .await
        .map_err(|e| {
            tracing::error!("list_all_control_tests failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list tests"})),
            )
        })?;

    Ok(Json(tests))
}

#[utoipa::path(
    post,
    path = "/api/v1/controls/{id}/tests/{test_id}/result",
    tag = "controls",
    params(("id" = i64, Path, description = "Control ID"), ("test_id" = i64, Path, description = "Test ID")),
    request_body = RecordTestResultRequest,
    responses(
        (status = 200, description = "Test result recorded", body = ControlTest),
        (status = 400, description = "Invalid result (use pass, fail, or skipped)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Test not found or not linked to control"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_test_result(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path((id, test_id)): Path<(i64, i64)>,
    Json(body): Json<RecordTestResultRequest>,
) -> Result<Json<ControlTest>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let result = body.result.trim().to_lowercase();
    if !["pass", "fail", "skipped"].contains(&result.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid result",
                "detail": "Use pass, fail, or skipped"
            })),
        ));
    }

    let tests = list_control_tests(pool, id).await.map_err(|e| {
        tracing::error!("list_control_tests failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to list control tests"})),
        )
    })?;
    if !tests.iter().any(|t| t.id == test_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Test not found or not linked to this control"})),
        ));
    }

    let updated = record_control_test_result(
        pool,
        test_id,
        &result,
        body.details.as_deref().map(str::trim).filter(|s| !s.is_empty()),
    )
    .await
    .map_err(|e| {
        tracing::error!("record_control_test_result failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to record test result"})),
        )
    })?;

    Ok(Json(updated))
}

#[derive(Debug, serde::Deserialize)]
pub struct EvidenceQuery {
    pub control_id: Option<i64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/evidence",
    tag = "controls",
    params(
        ("control_id" = Option<i64>, Query, description = "Filter by control ID"),
        ("limit" = Option<u32>, Query, description = "Limit (default 50)"),
        ("offset" = Option<u32>, Query, description = "Offset (default 0)"),
    ),
    responses(
        (status = 200, description = "List of evidence", body = Vec<EvidenceItem>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Query(q): Query<EvidenceQuery>,
) -> Result<Json<Vec<EvidenceItem>>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);

    let evidence = list_evidence(pool, q.control_id, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!("list_evidence failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list evidence"})),
            )
        })?;

    Ok(Json(evidence))
}

/// Policy/document evidence types included in control–policy mapping (G6).
const POLICY_EVIDENCE_TYPES: &[&str] = &["policy", "document"];

#[utoipa::path(
    get,
    path = "/api/v1/controls/policy-mapping",
    tag = "controls",
    responses(
        (status = 200, description = "Control–policy mapping", body = ControlPolicyMappingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_control_policy_mapping(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ControlPolicyMappingResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let controls = list_controls(pool)
        .await
        .map_err(|e| {
            tracing::error!("list_controls failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list controls"})),
            )
        })?;

    let all_evidence = list_evidence(pool, None, 1000, 0)
        .await
        .map_err(|e| {
            tracing::error!("list_evidence failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to list evidence"})),
            )
        })?;

    let policy_evidence: Vec<_> = all_evidence
        .into_iter()
        .filter(|e| POLICY_EVIDENCE_TYPES.contains(&e.r#type.as_str()))
        .collect();

    let mut by_control: std::collections::HashMap<i64, Vec<PolicyRef>> = std::collections::HashMap::new();
    for ev in policy_evidence {
        let ref_ = PolicyRef {
            id: ev.id,
            r#type: ev.r#type,
            source: ev.source,
            link_url: ev.link_url,
            file_name: ev.file_name,
        };
        by_control.entry(ev.control_id).or_default().push(ref_);
    }

    let mapping: Vec<ControlPolicyMappingEntry> = controls
        .into_iter()
        .map(|c| {
            let policies = by_control.remove(&c.id).unwrap_or_default();
            ControlPolicyMappingEntry {
                control_id: c.id,
                internal_id: c.internal_id,
                name: c.name,
                policies,
            }
        })
        .collect();

    Ok(Json(ControlPolicyMappingResponse { mapping }))
}

#[utoipa::path(
    get,
    path = "/api/v1/system-description",
    tag = "controls",
    responses(
        (status = 200, description = "SOC 2 System Description template", body = SystemDescriptionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_system_description(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<SystemDescriptionResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (config, _pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let template = r#"# System Description (SOC 2)

**Organisation:** {{organisation_name}}
**System / service in scope:** {{system_name}}
**As of date:** {{as_of_date}}

## 1. Overview

This document describes the system and the control environment in scope for the service organisation’s SOC 2 examination. It is intended to support auditors and stakeholders in understanding the boundaries of the system and the controls that apply.

## 2. System boundaries

- **In scope:** {{system_name}} and supporting processes (access, change management, risk assessment, monitoring).
- **Out of scope:** (Specify any excluded subsystems or third-party services.)

## 3. Control environment

The organisation maintains controls aligned to SOC 2 Trust Services Criteria (Security, Availability, Processing Integrity, Confidentiality, Privacy as applicable). Controls are documented and evidence is collected and reviewed. Control–policy mapping is available via the compliance platform (e.g. GET /api/v1/controls/policy-mapping).

## 4. Placeholders

Replace the placeholders above with your organisation name, system name, and the date of the description. You may extend this template with additional sections (e.g. infrastructure, data flows, third parties) as required by your auditor.
"#.to_string();

    let placeholders = vec![
        "{{organisation_name}}".to_string(),
        "{{system_name}}".to_string(),
        "{{as_of_date}}".to_string(),
    ];

    Ok(Json(SystemDescriptionResponse { template, placeholders }))
}

#[utoipa::path(
    post,
    path = "/api/v1/evidence",
    tag = "controls",
    request_body = CreateEvidenceRequest,
    responses(
        (status = 201, description = "Evidence created", body = CreateEvidenceResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<CreateEvidenceRequest>,
) -> Result<(StatusCode, Json<CreateEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let type_trim = body.r#type.trim();
    if type_trim.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "type is required"})),
        ));
    }

    let id = create_evidence(
        pool,
        body.control_id,
        type_trim,
        body.source.as_deref(),
        body.description.as_deref(),
        body.link_url.as_deref(),
        Some(user.sub.as_str()),
        None,
        None,
        None,
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!("create_evidence failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to create evidence"})),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateEvidenceResponse {
            id,
            control_id: body.control_id,
            collected_at: Utc::now(),
        }),
    ))
}

/// Extract plain text from file bytes for AI analysis (Phase B). Supports txt and PDF.
fn extract_text_from_file(content_type: &str, bytes: &[u8]) -> Option<String> {
    let ct = content_type.to_lowercase();
    let ct = ct.split(';').next().unwrap_or(&ct).trim();
    if ct == "text/plain" || ct.ends_with("text") {
        return std::str::from_utf8(bytes).ok().map(String::from);
    }
    if ct == "application/pdf" {
        return pdf_extract::extract_text_from_mem(bytes).ok().filter(|s: &String| !s.is_empty());
    }
    // Fallback: try utf-8
    std::str::from_utf8(bytes).ok().map(String::from)
}

#[utoipa::path(
    post,
    path = "/api/v1/evidence/upload",
    tag = "controls",
    responses(
        (status = 201, description = "Evidence created from upload", body = CreateEvidenceResponse),
        (status = 400, description = "Invalid request or COMP_AI_EVIDENCE_STORAGE_PATH not set"),
        (status = 401, description = "Unauthorized"),
        (status = 413, description = "File too large"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_evidence_upload(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<CreateEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let storage_path = match &config.comp_ai_evidence_storage_path {
        Some(p) => p.clone(),
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "Upload not configured: set COMP_AI_EVIDENCE_STORAGE_PATH"})),
            ));
        }
    };
    let max_bytes = config.comp_ai_evidence_max_file_bytes;

    let mut control_id: Option<i64> = None;
    let mut evidence_type: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::warn!("multipart next_field: {}", e);
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid multipart"})))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == "control_id" {
            if let Ok(data) = field.bytes().await {
                if let Ok(s) = std::str::from_utf8(&data) {
                    control_id = s.trim().parse().ok();
                }
            }
        } else if name == "type" {
            if let Ok(data) = field.bytes().await {
                evidence_type = std::str::from_utf8(&data).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
            }
        } else if name == "file" {
            file_name = field.file_name().map(String::from);
            content_type = field.content_type().map(|c| c.to_string());
            if let Ok(data) = field.bytes().await {
                if data.len() as u64 > max_bytes {
                    return Err((
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(serde_json::json!({"error": "File too large", "max_bytes": max_bytes})),
                    ));
                }
                file_bytes = Some(data.to_vec());
            }
        }
    }

    let control_id = control_id.ok_or((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "control_id required"})),
    ))?;
    let evidence_type = evidence_type.unwrap_or_else(|| "document".to_string());
    let (file_bytes, file_name, content_type) = match file_bytes {
        Some(b) => (b, file_name.unwrap_or_else(|| "upload".to_string()), content_type.unwrap_or_else(|| "application/octet-stream".to_string())),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "file field required"})),
            ));
        }
    };

    let allowed = ["application/pdf", "text/plain", "text/csv", "application/octet-stream"];
    let ct_lower = content_type.to_lowercase();
    let ct_base = ct_lower.split(';').next().unwrap_or(&ct_lower).trim();
    if !allowed.contains(&ct_base) && !ct_base.ends_with("text") {
        tracing::info!("Upload content-type {} not in allowed list; storing anyway", ct_base);
    }

    let extracted_text = extract_text_from_file(&content_type, &file_bytes);
    let safe_name: String = file_name.chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' }).collect();
    let rel_path = format!("{}_{}", Uuid::new_v4().simple(), safe_name);
    let _ = std::fs::create_dir_all(&storage_path);
    let full_path = std::path::Path::new(&storage_path).join(&rel_path);
    if std::fs::write(&full_path, &file_bytes).is_err() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save file"})),
        ));
    }

    let id = create_evidence(
        pool,
        control_id,
        &evidence_type,
        Some(&file_name),
        None,
        None,
        Some(user.sub.as_str()),
        Some(&rel_path),
        Some(&file_name),
        Some(&content_type),
        extracted_text.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("create_evidence (upload) failed: {}", e);
        let _ = std::fs::remove_file(&full_path);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to create evidence"})),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateEvidenceResponse {
            id,
            control_id,
            collected_at: Utc::now(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/evidence/{id}/analyze",
    tag = "controls",
    params(("id" = i64, Path, description = "Evidence ID")),
    responses(
        (status = 200, description = "AI analysis summary", body = EvidenceAnalyzeResponse),
        (status = 400, description = "No extracted text for this evidence"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Evidence not found"),
        (status = 503, description = "Ollama not available")
    )
)]
pub async fn post_evidence_analyze(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<EvidenceAnalyzeResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let evidence = get_evidence_by_id(pool, id)
        .await
        .map_err(|e| {
            tracing::error!("get_evidence_by_id failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to load evidence"})))
        })?
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Evidence not found"}))))?;

    let text = evidence.extracted_text.as_deref().filter(|s: &&str| !s.trim().is_empty()).ok_or((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "No extracted text for this evidence; upload a document with text (PDF or plain text) first"})),
    ))?;

    let controls = list_controls(pool).await.map_err(|e| {
        tracing::error!("list_controls failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to load controls"})))
    })?;
    let control_summary: String = controls
        .iter()
        .take(50)
        .map(|c| format!("- id={} {} ({})", c.id, c.name, c.internal_id))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt_prefix = format!(
        "You are a compliance analyst. Below is extracted text from a document (evidence). \
         Suggest which controls (from the list) this evidence might support, and any gaps.\n\n\
         Controls:\n{}\n\nDocument text (excerpt):",
        control_summary
    );
    let excerpt: String = text.chars().take(8000).collect();
    let user_prompt = format!("Summarise: (1) which controls this evidence supports and why; (2) any gaps or missing evidence to note. Document excerpt:\n\n{}", excerpt);

    let (base_url, model) = match (config.comp_ai_ollama_url.as_deref(), config.comp_ai_ollama_model.as_deref()) {
        (Some(u), Some(m)) => (u, m),
        _ => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "Ollama not configured; set COMP_AI_OLLAMA_URL and COMP_AI_OLLAMA_MODEL"})),
            ));
        }
    };

    let result = inference::generate(
        base_url,
        model,
        &user_prompt,
        config.comp_ai_max_tokens,
        config.comp_ai_temperature,
        Some(&prompt_prefix),
    )
    .await;

    let (summary, model_used) = match result {
        Ok((s, _)) => (s, model.to_string()),
        Err(e) => {
            tracing::error!("Ollama generate failed: {}", e);
            if config.comp_ai_ollama_fallback_to_mock {
                let mock = format!("[Mock] This document could support controls related to its content. Review controls list and map manually. Error: {}", e);
                return Ok(Json(EvidenceAnalyzeResponse {
                    summary: mock,
                    suggested_control_ids: None,
                    gaps: None,
                    model_used: "mock".to_string(),
                }));
            }
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "AI analysis failed", "detail": e.to_string()})),
            ));
        }
    };

    Ok(Json(EvidenceAnalyzeResponse {
        summary,
        suggested_control_ids: None,
        gaps: None,
        model_used,
    }))
}

/// G9: Review all evidence for a control – AI flags gaps and suggests fixes.
#[utoipa::path(
    post,
    path = "/api/v1/controls/{id}/evidence/review",
    tag = "controls",
    params(("id" = i64, Path, description = "Control ID")),
    responses(
        (status = 200, description = "AI review of evidence for this control", body = EvidenceReviewResponse),
        (status = 404, description = "Control not found"),
        (status = 503, description = "Ollama not available")
    )
)]
pub async fn post_control_evidence_review(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<EvidenceReviewResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let control = get_control_with_requirements(pool, id)
        .await
        .map_err(|e| {
            tracing::error!("get_control_with_requirements failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to get control"})))
        })?
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Control not found"}))))?;

    let evidence_list = list_evidence(pool, Some(id), 500, 0)
        .await
        .map_err(|e| {
            tracing::error!("list_evidence failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to list evidence"})))
        })?;

    let req_codes: Vec<String> = control
        .requirements
        .iter()
        .map(|r| format!("{} ({})", r.code, r.framework_slug))
        .collect();
    let evidence_context: String = evidence_list
        .iter()
        .filter_map(|e| {
            let text = e.extracted_text.as_deref().filter(|s| !s.trim().is_empty())?;
            let excerpt: String = text.chars().take(2000).collect();
            Some(format!(
                "- Evidence id={} type={}: {}",
                e.id,
                e.r#type,
                if excerpt.len() < text.chars().count() {
                    format!("{}...", excerpt)
                } else {
                    excerpt
                }
            ))
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let evidence_context = if evidence_context.is_empty() {
        "(No evidence with extracted text yet. Upload documents and use Analyse to add content.)".to_string()
    } else {
        evidence_context
    };

    let system_prompt = format!(
        "You are a compliance analyst. Review all evidence collected for the following control.\n\n\
         Control: {} [{}]\nDescription: {}\n\
         Mapped requirements: {}\n\n\
         Evidence collected (with extracted text):\n{}\n\n\
         Provide: (1) A short summary of how well the evidence supports the control. \
         (2) List any gaps or missing evidence (as bullet points). (3) Suggest concrete fixes or additional evidence to collect (as bullet points). \
         Be concise and actionable.",
        control.name,
        control.internal_id,
        control.description.as_deref().unwrap_or("(none)"),
        req_codes.join(", "),
        evidence_context
    );

    let (base_url, model) = match (config.comp_ai_ollama_url.as_deref(), config.comp_ai_ollama_model.as_deref()) {
        (Some(u), Some(m)) => (u, m),
        _ => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "Ollama not configured; set COMP_AI_OLLAMA_URL and COMP_AI_OLLAMA_MODEL"})),
            ));
        }
    };

    let result = inference::generate(
        base_url,
        model,
        "Summarise support level, list gaps, and suggest fixes.",
        config.comp_ai_max_tokens,
        config.comp_ai_temperature,
        Some(&system_prompt),
    )
    .await;

    let (summary, model_used) = match result {
        Ok((s, _)) => (s.trim().to_string(), model.to_string()),
        Err(e) => {
            tracing::error!("Ollama evidence review failed: {}", e);
            if config.comp_ai_ollama_fallback_to_mock {
                return Ok(Json(EvidenceReviewResponse {
                    summary: format!(
                        "[Mock] Review evidence for control {} manually. Gaps and suggested fixes: ensure all requirements {} are covered by evidence. Error: {}",
                        control.internal_id,
                        req_codes.join(", "),
                        e
                    ),
                    gaps: None,
                    suggested_fixes: None,
                    model_used: "mock".to_string(),
                }));
            }
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "AI review failed", "detail": e.to_string()})),
            ));
        }
    };

    Ok(Json(EvidenceReviewResponse {
        summary,
        gaps: None,
        suggested_fixes: None,
        model_used,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/integrations/github/evidence",
    tag = "controls",
    request_body = GitHubEvidenceRequest,
    responses(
        (status = 201, description = "Evidence collected from GitHub and linked to control", body = CreateEvidenceResponse),
        (status = 400, description = "Invalid request or evidence_type"),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "GitHub integration not configured or API error"),
    )
)]
pub async fn post_github_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<GitHubEvidenceRequest>,
) -> Result<(StatusCode, Json<CreateEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let token = config
        .github_token
        .as_deref()
        .filter(|t| !t.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "GitHub integration not configured",
                "detail": "Set GITHUB_TOKEN to collect evidence from GitHub"
            })),
        ))?;

    let evidence_type_trim = body.evidence_type.trim().to_lowercase();
    let evidence = match evidence_type_trim.as_str() {
        "last_commit" => fetch_last_commit_evidence(token, &body.owner, &body.repo)
            .await
            .map_err(|e| {
                tracing::error!("GitHub last_commit failed: {}", e);
                (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({
                        "error": "Failed to fetch GitHub repo/commit",
                        "detail": e.to_string()
                    })),
                )
            })?,
        "branch_protection" => fetch_branch_protection_evidence(token, &body.owner, &body.repo)
            .await
            .map_err(|e| {
                tracing::error!("GitHub branch_protection failed: {}", e);
                (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({
                        "error": "Failed to fetch GitHub branch protection",
                        "detail": e.to_string()
                    })),
                )
            })?,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid evidence_type",
                    "detail": "Use 'last_commit' or 'branch_protection'"
                })),
            ));
        }
    };

    let id = create_evidence(
        pool,
        body.control_id,
        &evidence.evidence_type,
        Some(&evidence.source),
        Some(&evidence.description),
        evidence.link_url.as_deref(),
        Some(user.sub.as_str()),
        None,
        None,
        None,
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!("create_evidence (GitHub) failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save evidence"})),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateEvidenceResponse {
            id,
            control_id: body.control_id,
            collected_at: Utc::now(),
        }),
    ))
}

/// G3: Collect evidence from Okta IdP and link to control.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/okta/evidence",
    tag = "controls",
    request_body = OktaEvidenceRequest,
    responses(
        (status = 201, description = "Evidence collected from Okta and linked to control", body = CreateEvidenceResponse),
        (status = 400, description = "Invalid request or evidence_type"),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Okta integration not configured or API error"),
    )
)]
pub async fn post_okta_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<OktaEvidenceRequest>,
) -> Result<(StatusCode, Json<CreateEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let domain = config
        .okta_domain
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Okta integration not configured",
                "detail": "Set OKTA_DOMAIN and OKTA_API_TOKEN to collect evidence from Okta"
            })),
        ))?;
    let token = config
        .okta_api_token
        .as_deref()
        .filter(|t| !t.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Okta integration not configured",
                "detail": "Set OKTA_API_TOKEN"
            })),
        ))?;

    let evidence_type_trim = body.evidence_type.trim().to_lowercase();
    let evidence = match evidence_type_trim.as_str() {
        "org_summary" => fetch_okta_org_summary(domain, token)
            .await
            .map_err(|e| {
                tracing::error!("Okta org_summary failed: {}", e);
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "Failed to fetch Okta org", "detail": e.to_string()})))
            })?,
        "users_snapshot" => fetch_okta_users_snapshot(domain, token)
            .await
            .map_err(|e| {
                tracing::error!("Okta users_snapshot failed: {}", e);
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "Failed to fetch Okta users", "detail": e.to_string()})))
            })?,
        "groups_snapshot" => fetch_okta_groups_snapshot(domain, token)
            .await
            .map_err(|e| {
                tracing::error!("Okta groups_snapshot failed: {}", e);
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "Failed to fetch Okta groups", "detail": e.to_string()})))
            })?,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid evidence_type",
                    "detail": "Use 'org_summary', 'users_snapshot', or 'groups_snapshot'"
                })),
            ));
        }
    };

    let id = create_evidence(
        pool,
        body.control_id,
        &evidence.evidence_type,
        Some(&evidence.source),
        Some(&evidence.description),
        evidence.link_url.as_deref(),
        Some(user.sub.as_str()),
        None,
        None,
        None,
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!("create_evidence (Okta) failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to save evidence"})))
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateEvidenceResponse { id, control_id: body.control_id, collected_at: Utc::now() }),
    ))
}

/// G4: Collect evidence from AWS IAM and link to control.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/aws/evidence",
    tag = "controls",
    request_body = AwsEvidenceRequest,
    responses(
        (status = 201, description = "Evidence collected from AWS IAM and linked to control", body = CreateEvidenceResponse),
        (status = 400, description = "Invalid request or evidence_type"),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "AWS integration not configured or API error"),
    )
)]
pub async fn post_aws_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<AwsEvidenceRequest>,
) -> Result<(StatusCode, Json<CreateEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let evidence_type_trim = body.evidence_type.trim().to_lowercase();
    let evidence = match evidence_type_trim.as_str() {
        "iam_summary" => fetch_aws_iam_summary(config.aws_region.as_deref())
            .await
            .map_err(|e| {
                tracing::error!("AWS iam_summary failed: {}", e);
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                    "error": "Failed to fetch AWS IAM summary",
                    "detail": e.to_string()
                })))
            })?,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid evidence_type",
                    "detail": "Use 'iam_summary'"
                })),
            ));
        }
    };

    let id = create_evidence(
        pool,
        body.control_id,
        &evidence.evidence_type,
        Some(&evidence.source),
        Some(&evidence.description),
        evidence.link_url.as_deref(),
        Some(user.sub.as_str()),
        None,
        None,
        None,
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!("create_evidence (AWS) failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to save evidence"})))
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateEvidenceResponse { id, control_id: body.control_id, collected_at: Utc::now() }),
    ))
}

/// D3: Collect email metadata from M365 (Microsoft Graph) and create one evidence row per message.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/m365/evidence",
    tag = "controls",
    request_body = M365EvidenceRequest,
    responses(
        (status = 200, description = "Evidence created for each email (metadata only)", body = M365EvidenceResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "M365 integration not configured or API error"),
    )
)]
pub async fn post_m365_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<M365EvidenceRequest>,
) -> Result<(StatusCode, Json<M365EvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let tenant_id = config
        .m365_tenant_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "M365 integration not configured",
                "detail": "Set M365_TENANT_ID, M365_CLIENT_ID, M365_CLIENT_SECRET, M365_MAILBOX_USER_ID"
            })),
        ))?;
    let client_id = config
        .m365_client_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "M365 integration not configured", "detail": "Set M365_CLIENT_ID"})),
        ))?;
    let client_secret = config
        .m365_client_secret
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "M365 integration not configured", "detail": "Set M365_CLIENT_SECRET"})),
        ))?;
    let mailbox_user_id = config
        .m365_mailbox_user_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "M365 integration not configured", "detail": "Set M365_MAILBOX_USER_ID"})),
        ))?;

    let limit = body.limit.unwrap_or(50);

    let emails = fetch_m365_email_list(
        tenant_id,
        client_id,
        client_secret,
        mailbox_user_id,
        limit,
    )
    .await
    .map_err(|e| {
        tracing::error!("M365 email list failed: {}", e);
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": "Failed to fetch M365 email list",
                "detail": e.to_string()
            })),
        )
    })?;

    let mut evidence_ids = Vec::with_capacity(emails.len());
    for ev in &emails {
        match create_evidence(
            pool,
            body.control_id,
            "email",
            Some(ev.source.as_str()),
            None,
            ev.link_url.as_deref(),
            Some(user.sub.as_str()),
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(id) => evidence_ids.push(id),
            Err(e) => {
                tracing::error!("create_evidence (M365) failed: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to save one or more email evidence",
                        "detail": format!("Created {} before error", evidence_ids.len())
                    })),
                ));
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(M365EvidenceResponse {
            created: evidence_ids.len(),
            evidence_ids,
        }),
    ))
}

/// D4: Store a DLP or compliance scan result as one evidence item (no external DLP call).
#[utoipa::path(
    post,
    path = "/api/v1/integrations/dlp/evidence",
    tag = "controls",
    request_body = DlpEvidenceRequest,
    responses(
        (status = 201, description = "DLP/scan result stored as evidence", body = CreateEvidenceResponse),
        (status = 400, description = "Invalid request (e.g. empty summary)"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_dlp_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<DlpEvidenceRequest>,
) -> Result<(StatusCode, Json<CreateEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let summary = body.summary.trim();
    if summary.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "summary is required and must be non-empty"})),
        ));
    }

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let scan_date = body
        .scan_date
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| today.as_str());
    let source = format!("DLP/compliance scan {}: {}", scan_date, summary);

    let id = create_evidence(
        pool,
        body.control_id,
        "dlp_scan",
        Some(&source),
        body.details.as_deref(),
        body.link_url.as_deref(),
        Some(user.sub.as_str()),
        None,
        None,
        None,
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!("create_evidence (DLP) failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save evidence"})),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateEvidenceResponse {
            id,
            control_id: body.control_id,
            collected_at: Utc::now(),
        }),
    ))
}

/// C3: List files in Google Drive folder and create one evidence row per file (link only).
#[utoipa::path(
    post,
    path = "/api/v1/integrations/drive/evidence",
    tag = "controls",
    request_body = DriveEvidenceRequest,
    responses(
        (status = 200, description = "Evidence created for each file", body = DriveEvidenceResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Drive integration not configured or API error"),
    )
)]
pub async fn post_drive_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<DriveEvidenceRequest>,
) -> Result<(StatusCode, Json<DriveEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let credentials_path = config
        .google_drive_credentials_path
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Google Drive integration not configured",
                "detail": "Set GOOGLE_DRIVE_CREDENTIALS_PATH and GOOGLE_DRIVE_FOLDER_ID"
            })),
        ))?;

    let folder_id = body
        .folder_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| config.google_drive_folder_id.as_deref())
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Folder ID required",
                "detail": "Set folder_id in request or GOOGLE_DRIVE_FOLDER_ID"
            })),
        ))?;

    let limit = body.limit.unwrap_or(50);

    let files = fetch_drive_file_list(credentials_path, folder_id, limit)
        .await
        .map_err(|e| {
            tracing::error!("Drive file list failed: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": "Failed to fetch Drive file list",
                    "detail": e.to_string()
                })),
            )
        })?;

    let mut evidence_ids = Vec::with_capacity(files.len());
    for ev in &files {
        match create_evidence(
            pool,
            body.control_id,
            "document",
            Some(ev.source.as_str()),
            None,
            ev.link_url.as_deref(),
            Some(user.sub.as_str()),
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(id) => evidence_ids.push(id),
            Err(e) => {
                tracing::error!("create_evidence (Drive) failed: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to save one or more Drive evidence",
                        "detail": format!("Created {} before error", evidence_ids.len())
                    })),
                ));
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(DriveEvidenceResponse {
            created: evidence_ids.len(),
            evidence_ids,
        }),
    ))
}

/// SharePoint: List files in a site/document library and create one evidence row per file (link only).
#[utoipa::path(
    post,
    path = "/api/v1/integrations/sharepoint/evidence",
    tag = "controls",
    request_body = SharepointEvidenceRequest,
    responses(
        (status = 200, description = "Evidence created for each file", body = SharepointEvidenceResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "SharePoint integration not configured or API error"),
    )
)]
pub async fn post_sharepoint_evidence(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<SharepointEvidenceRequest>,
) -> Result<(StatusCode, Json<SharepointEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let tenant_id = config
        .m365_tenant_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "SharePoint integration not configured",
                "detail": "Set M365_TENANT_ID, M365_CLIENT_ID, M365_CLIENT_SECRET (same app needs Sites.Read.All)"
            })),
        ))?;
    let client_id = config
        .m365_client_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "SharePoint integration not configured",
                "detail": "Set M365_CLIENT_ID"
            })),
        ))?;
    let client_secret = config
        .m365_client_secret
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "SharePoint integration not configured",
                "detail": "Set M365_CLIENT_SECRET"
            })),
        ))?;

    let site_id = body.site_id.trim();
    if site_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "site_id is required",
                "detail": "Provide SharePoint site ID (e.g. from site URL or Graph)"
            })),
        ));
    }

    let limit = body.limit.unwrap_or(50);

    let files = fetch_sharepoint_file_list(
        tenant_id,
        client_id,
        client_secret,
        site_id,
        body.drive_id.as_deref(),
        body.folder_path.as_deref(),
        limit,
    )
    .await
    .map_err(|e| {
        tracing::error!("SharePoint file list failed: {}", e);
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": "Failed to fetch SharePoint file list",
                "detail": e.to_string()
            })),
        )
    })?;

    let mut evidence_ids = Vec::with_capacity(files.len());
    for ev in &files {
        match create_evidence(
            pool,
            body.control_id,
            "document",
            Some(ev.source.as_str()),
            None,
            ev.link_url.as_deref(),
            Some(user.sub.as_str()),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(id) => evidence_ids.push(id),
            Err(e) => {
                tracing::error!("create_evidence (SharePoint) failed: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to save one or more SharePoint evidence",
                        "detail": format!("Created {} before error", evidence_ids.len())
                    })),
                ));
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(SharepointEvidenceResponse {
            created: evidence_ids.len(),
            evidence_ids,
        }),
    ))
}

/// Phase C1: Batch create evidence from document URLs (scan documents).
const SCAN_DOCUMENTS_MAX: usize = 50;

#[utoipa::path(
    post,
    path = "/api/v1/scan/documents",
    tag = "controls",
    request_body = ScanDocumentsRequest,
    responses(
        (status = 200, description = "Evidence created for each document URL", body = ScanDocumentsResponse),
        (status = 400, description = "Invalid request or too many documents"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_scan_documents(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<ScanDocumentsRequest>,
) -> Result<(StatusCode, Json<ScanDocumentsResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    if body.documents.len() > SCAN_DOCUMENTS_MAX {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Too many documents",
                "detail": format!("Maximum {} documents per request", SCAN_DOCUMENTS_MAX)
            })),
        ));
    }

    let mut evidence_ids = Vec::with_capacity(body.documents.len());
    for item in &body.documents {
        let url = item.url.trim();
        if url.is_empty() {
            continue;
        }
        let ev_type = item
            .r#type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("document");
        let source = format!("Scan: {}", url);
        match create_evidence(
            pool,
            body.control_id,
            ev_type,
            Some(&source),
            Some(url),
            Some(url),
            Some(user.sub.as_str()),
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(id) => evidence_ids.push(id),
            Err(e) => {
                tracing::error!("create_evidence (scan) failed for url {}: {}", url, e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to create evidence for one or more documents",
                        "detail": format!("Created {} before error", evidence_ids.len())
                    })),
                ));
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(ScanDocumentsResponse {
            created: evidence_ids.len(),
            evidence_ids,
        }),
    ))
}

/// Phase C2: Batch upload multiple files; create evidence for each (with text extraction).
const SCAN_UPLOAD_BATCH_MAX: usize = 20;

#[utoipa::path(
    post,
    path = "/api/v1/scan/upload-batch",
    tag = "controls",
    responses(
        (status = 200, description = "Evidence created for each file", body = ScanDocumentsResponse),
        (status = 400, description = "Invalid request or no files"),
        (status = 401, description = "Unauthorized"),
        (status = 413, description = "File too large"),
        (status = 503, description = "Upload not configured: COMP_AI_EVIDENCE_STORAGE_PATH"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn post_scan_upload_batch(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ScanDocumentsResponse>), (StatusCode, Json<serde_json::Value>)> {
    let (config, pool, _) = &state;
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let storage_path = match &config.comp_ai_evidence_storage_path {
        Some(p) => p.clone(),
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "Upload not configured: set COMP_AI_EVIDENCE_STORAGE_PATH"})),
            ));
        }
    };
    let max_bytes = config.comp_ai_evidence_max_file_bytes;

    let mut control_id: Option<i64> = None;
    let mut evidence_type_default: String = "document".to_string();
    let mut files: Vec<(String, String, Vec<u8>)> = Vec::new(); // (file_name, content_type, bytes)

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::warn!("multipart next_field: {}", e);
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid multipart"})))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == "control_id" {
            if let Ok(data) = field.bytes().await {
                if let Ok(s) = std::str::from_utf8(&data) {
                    control_id = s.trim().parse().ok();
                }
            }
        } else if name == "type" {
            if let Ok(data) = field.bytes().await {
                if let Ok(s) = std::str::from_utf8(&data) {
                    let t = s.trim();
                    if !t.is_empty() {
                        evidence_type_default = t.to_string();
                    }
                }
            }
        } else if name == "file" || name.starts_with("file") {
            let file_name = field.file_name().map(String::from).unwrap_or_else(|| "upload".to_string());
            let content_type = field.content_type().map(|c| c.to_string()).unwrap_or_else(|| "application/octet-stream".to_string());
            if let Ok(data) = field.bytes().await {
                let bytes = data.to_vec();
                if bytes.len() as u64 > max_bytes {
                    return Err((
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(serde_json::json!({"error": "File too large", "max_bytes": max_bytes, "file": file_name})),
                    ));
                }
                files.push((file_name, content_type, bytes));
            }
        }
    }

    let control_id = control_id.ok_or((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "control_id required"})),
    ))?;
    if files.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "At least one file required"})),
        ));
    }
    if files.len() > SCAN_UPLOAD_BATCH_MAX {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Too many files",
                "detail": format!("Maximum {} files per request", SCAN_UPLOAD_BATCH_MAX)
            })),
        ));
    }

    let _ = std::fs::create_dir_all(&storage_path);
    let mut evidence_ids = Vec::with_capacity(files.len());
    let mut saved_paths: Vec<std::path::PathBuf> = Vec::new();

    for (file_name, content_type, file_bytes) in files {
        let extracted_text = extract_text_from_file(&content_type, &file_bytes);
        let safe_name: String = file_name.chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' }).collect();
        let rel_path = format!("{}_{}", Uuid::new_v4().simple(), safe_name);
        let full_path = std::path::Path::new(&storage_path).join(&rel_path);
        if std::fs::write(&full_path, &file_bytes).is_err() {
            for p in saved_paths {
                let _ = std::fs::remove_file(&p);
            }
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to save file", "file": file_name})),
            ));
        }
        saved_paths.push(full_path.clone());

        match create_evidence(
            pool,
            control_id,
            &evidence_type_default,
            Some(&file_name),
            None,
            None,
            Some(user.sub.as_str()),
            Some(&rel_path),
            Some(&file_name),
            Some(&content_type),
            extracted_text.as_deref(),
        )
        .await
        {
            Ok(id) => evidence_ids.push(id),
            Err(e) => {
                tracing::error!("create_evidence (upload-batch) failed for {}: {}", file_name, e);
                for p in &saved_paths {
                    let _ = std::fs::remove_file(p);
                }
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to create evidence for one or more files",
                        "detail": format!("Created {} before error", evidence_ids.len())
                    })),
                ));
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(ScanDocumentsResponse {
            created: evidence_ids.len(),
            evidence_ids,
        }),
    ))
}
