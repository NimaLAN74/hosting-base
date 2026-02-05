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
    GapAnalysisRequest, GapAnalysisResponse, EvidenceAnalyzeResponse,
};
use crate::db::queries::{
    list_controls, get_control_with_requirements, list_evidence, create_evidence, get_evidence_by_id,
    list_controls_without_evidence, list_remediation_tasks, get_remediation_by_control_id,
    upsert_remediation, list_requirements,
    list_control_tests, list_all_control_tests, record_control_test_result,
};
use crate::inference;
use crate::utils::{format_eu_date, parse_eu_date};
use crate::integrations::github::{fetch_last_commit_evidence, fetch_branch_protection_evidence};
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

#[utoipa::path(
    get,
    path = "/api/v1/controls",
    tag = "controls",
    responses(
        (status = 200, description = "List of controls", body = Vec<Control>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_controls(
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<Vec<Control>>, (StatusCode, Json<serde_json::Value>)> {
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
