mod config;
mod models;
mod handlers;
mod auth;

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
use handlers::comp_ai::process_request;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health_check,
        handlers::comp_ai::process_request,
        handlers::comp_ai::get_request_history,
    ),
    components(schemas(
        models::CompAIRequest,
        models::CompAIResponse,
        models::RequestHistory,
        models::RequestHistoryResponse,
        models::RequestHistoryQueryParams,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "comp-ai", description = "Comp AI service endpoints"),
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

    // Build the application router
    let app = Router::new()
        // Public routes (no authentication required)
        .route("/health", get(health_check))
        // Protected routes (authentication required)
        .route("/api/v1/process", axum::routing::post(process_request))
        .route("/api/v1/history", get(handlers::comp_ai::get_request_history))
        // Swagger UI (public)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(AllowOrigin::any())
                        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION]),
                )
                // Add config to request extensions for AuthenticatedUser extractor  
                .layer(tower::AddExtensionLayer::new(config.clone()))
        )
        .with_state(config.clone());

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("Comp AI Service listening on http://0.0.0.0:{}", config.port);
    tracing::info!("Swagger UI available at http://0.0.0.0:{}/swagger-ui", config.port);

    axum::serve(listener, app).await?;

    Ok(())
}
