//! Phase 4: Controls and evidence API handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use sqlx::PgPool;
use crate::config::AppConfig;
use crate::auth::extract_user;
use crate::models::{
    Control, ControlWithRequirements, EvidenceItem, CreateEvidenceRequest, CreateEvidenceResponse,
    GitHubEvidenceRequest,
};
use crate::db::queries::{list_controls, get_control_with_requirements, list_evidence, create_evidence};
use crate::integrations::github::{fetch_last_commit_evidence, fetch_branch_protection_evidence};
use chrono::Utc;
use crate::handlers::comp_ai::ResponseCache;

type AppState = (Arc<AppConfig>, PgPool, ResponseCache);

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
    State((config, pool, _)): State<AppState>,
) -> Result<Json<Vec<Control>>, (StatusCode, Json<serde_json::Value>)> {
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let controls = list_controls(&pool)
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
    State((config, pool, _)): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ControlWithRequirements>, (StatusCode, Json<serde_json::Value>)> {
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let control = get_control_with_requirements(&pool, id)
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
    State((config, pool, _)): State<AppState>,
    Query(q): Query<EvidenceQuery>,
) -> Result<Json<Vec<EvidenceItem>>, (StatusCode, Json<serde_json::Value>)> {
    let _user = extract_user(&headers, config.clone())
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);

    let evidence = list_evidence(&pool, q.control_id, limit, offset)
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
    State((config, pool, _)): State<AppState>,
    Json(body): Json<CreateEvidenceRequest>,
) -> Result<(StatusCode, Json<CreateEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
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
        &pool,
        body.control_id,
        type_trim,
        body.source.as_deref(),
        body.description.as_deref(),
        body.link_url.as_deref(),
        user.sub.as_deref(),
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
    State((config, pool, _)): State<AppState>,
    Json(body): Json<GitHubEvidenceRequest>,
) -> Result<(StatusCode, Json<CreateEvidenceResponse>), (StatusCode, Json<serde_json::Value>)> {
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
        &pool,
        body.control_id,
        &evidence.evidence_type,
        Some(&evidence.source),
        Some(&evidence.description),
        evidence.link_url.as_deref(),
        user.sub.as_deref(),
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
