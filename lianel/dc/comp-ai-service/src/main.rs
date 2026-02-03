mod config;
mod models;
mod handlers;
mod auth;
mod db;
mod inference;
mod rate_limit;
mod frameworks;
mod response_cache;
mod integrations;

use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{CorsLayer, AllowOrigin},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use config::AppConfig;
use handlers::health::health_check;
use handlers::comp_ai::{get_frameworks, get_request_history, process_request};
use handlers::controls::{
    get_controls, get_control, get_controls_export, get_controls_gaps,
    get_remediation, get_control_remediation, put_control_remediation,
    get_evidence, post_evidence, post_github_evidence,
};
use db::create_pool;
use rate_limit::{RateLimitLayer, RateLimitState};
use response_cache::create_cache;
use sqlx::PgPool;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health_check,
        handlers::comp_ai::process_request,
        handlers::comp_ai::get_request_history,
        handlers::comp_ai::get_frameworks,
        handlers::controls::get_controls,
        handlers::controls::get_control,
        handlers::controls::get_controls_export,
        handlers::controls::get_controls_gaps,
        handlers::controls::get_remediation,
        handlers::controls::get_control_remediation,
        handlers::controls::put_control_remediation,
        handlers::controls::get_evidence,
        handlers::controls::post_evidence,
        handlers::controls::post_github_evidence,
    ),
    components(schemas(
        models::CompAIRequest,
        models::CompAIResponse,
        models::ChatMessage,
        models::FrameworkItemResponse,
        models::FrameworksListResponse,
        models::RequestHistory,
        models::RequestHistoryResponse,
        models::RequestHistoryQueryParams,
        models::Control,
        models::ControlWithRequirements,
        models::RequirementRef,
        models::EvidenceItem,
        models::CreateEvidenceRequest,
        models::CreateEvidenceResponse,
        models::GitHubEvidenceRequest,
        models::ControlExportEntry,
        models::AuditExport,
        models::RemediationTask,
        models::UpsertRemediationRequest,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "comp-ai", description = "Comp AI service endpoints"),
        (name = "controls", description = "Phase 4: controls and evidence"),
    ),
    servers(
        (url = "https://www.lianel.se", description = "Production server")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Load configuration
    let config = AppConfig::from_env()?;
    let config = Arc::new(config);

    tracing::info!("Starting Comp AI Service");
    tracing::info!("Port: {}", config.port);
    tracing::info!("Connecting to database...");

    // Create database pool
    let pool = create_pool(&config.database_url()).await?;
    tracing::info!("Database connection established");

    // Test connection
    sqlx::query("SELECT 1").execute(&pool).await?;
    tracing::info!("Database connection verified");

    // Rate limiting for API routes (per-IP from X-Real-IP / X-Forwarded-For)
    let rate_limit = RateLimitState::new(
        config.comp_ai_rate_limit_requests,
        config.comp_ai_rate_limit_window_secs,
    );
    let rate_limit_layer = RateLimitLayer::new(rate_limit);
    if config.comp_ai_rate_limit_requests > 0 {
        tracing::info!(
            "Rate limit: {} requests per {} seconds (per client IP)",
            config.comp_ai_rate_limit_requests,
            config.comp_ai_rate_limit_window_secs,
        );
    }

    // Response cache (optional; disabled when TTL = 0)
    let response_cache = create_cache(
        config.comp_ai_response_cache_ttl_secs,
        config.comp_ai_response_cache_max_entries,
    ).map(Arc::new);
    if response_cache.is_some() {
        tracing::info!(
            "Response cache: TTL {}s, max {} entries",
            config.comp_ai_response_cache_ttl_secs,
            config.comp_ai_response_cache_max_entries,
        );
    }

    // Protected API routes (auth + rate limit)
    let api_routes = Router::new()
        .route("/api/v1/process", axum::routing::post(process_request))
        .route("/api/v1/history", get(get_request_history))
        .route("/api/v1/frameworks", get(get_frameworks))
        .route("/api/v1/controls", get(get_controls))
        .route("/api/v1/controls/export", get(get_controls_export))
        .route("/api/v1/controls/gaps", get(get_controls_gaps))
        .route("/api/v1/controls/:id/remediation", get(get_control_remediation).put(put_control_remediation))
        .route("/api/v1/controls/:id", get(get_control))
        .route("/api/v1/remediation", get(get_remediation))
        .route("/api/v1/evidence", get(get_evidence).post(post_evidence))
        .route("/api/v1/integrations/github/evidence", axum::routing::post(post_github_evidence))
        .layer(rate_limit_layer)
        .with_state((config.clone(), pool, response_cache));

    // Build the application router
    let app = Router::new()
        // Public routes (no authentication required)
        .route("/health", get(health_check))
        // Swagger UI (public)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .merge(api_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(AllowOrigin::any())
                        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT])
                        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION]),
                )
        );

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("Comp AI Service listening on http://0.0.0.0:{}", config.port);
    tracing::info!("Swagger UI available at http://0.0.0.0:{}/swagger-ui", config.port);

    axum::serve(listener, app).await?;

    Ok(())
}
