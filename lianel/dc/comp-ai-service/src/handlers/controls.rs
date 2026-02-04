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
};
use crate::db::queries::{
    list_controls, get_control_with_requirements, list_evidence, create_evidence,
    list_controls_without_evidence, list_remediation_tasks, get_remediation_by_control_id,
    upsert_remediation, list_requirements,
    list_control_tests, list_all_control_tests, record_control_test_result,
};
use crate::inference;
use crate::utils::{format_eu_date, parse_eu_date};
use crate::integrations::github::{fetch_last_commit_evidence, fetch_branch_protection_evidence};
use chrono::Utc;
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
