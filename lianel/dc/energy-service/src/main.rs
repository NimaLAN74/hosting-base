mod config;
mod models;
mod handlers;
mod db;

use axum::{
    routing::get,
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{CorsLayer, AllowOrigin},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use config::AppConfig;
use db::create_pool;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health_check,
        handlers::energy::get_energy_annual,
        handlers::energy::get_energy_by_country,
        handlers::energy::get_energy_by_year,
        handlers::energy::get_energy_summary,
        handlers::metadata::get_service_info,
    ),
    components(schemas(
        models::EnergyRecord,
        models::EnergyResponse,
        models::EnergySummary,
        models::EnergySummaryResponse,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "energy", description = "Energy data endpoints"),
        (name = "info", description = "Service information endpoints"),
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

    let config = AppConfig::from_env();
    
    tracing::info!("Starting lianel-energy-service on port {}", config.port);
    tracing::info!("Connecting to database...");

    // Create database pool
    let pool = create_pool(&config.database_url).await?;
    tracing::info!("Database connection established");

    // Test connection
    sqlx::query("SELECT 1").execute(&pool).await?;
    tracing::info!("Database connection verified");

    // Build application
    // Note: Routes don't include /api/energy prefix because Nginx strips it
    // Nginx rewrites /api/energy/annual -> /annual before proxying
    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .route("/health", get(handlers::health::health_check))
        .route("/info", get(handlers::metadata::get_service_info))
        .route("/annual", get(handlers::energy::get_energy_annual))
        .route("/annual/by-country/:country_code", get(handlers::energy::get_energy_by_country))
        .route("/annual/by-year/:year", get(handlers::energy::get_energy_by_year))
        .route("/annual/summary", get(handlers::energy::get_energy_summary))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(AllowOrigin::any())
                        .allow_methods([axum::http::Method::GET, axum::http::Method::OPTIONS])
                        .allow_headers([
                            axum::http::header::AUTHORIZATION,
                            axum::http::header::CONTENT_TYPE,
                        ]),
                )
        )
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("Server listening on http://0.0.0.0:{}", config.port);
    tracing::info!("Swagger UI available at http://0.0.0.0:{}/swagger-ui", config.port);

    axum::serve(listener, app).await?;

    Ok(())
}

// Route fix: /info instead of /api/info
