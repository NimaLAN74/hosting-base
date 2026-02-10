//! Stock Monitoring Service – binary. EU markets MVP.
//! Step 1.3 Keycloak JWT (JWKS), 1.4 public vs protected routes.

use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lianel_stock_monitoring_service::app::{create_router, AppState};
use lianel_stock_monitoring_service::auth::KeycloakJwtValidator;
use lianel_stock_monitoring_service::config::AppConfig;
use lianel_stock_monitoring_service::db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;
    let pool = db::create_pool(&config.database_url()).await?;
    let config = Arc::new(config);
    let validator = Arc::new(KeycloakJwtValidator::new(config.clone()));

    let state = AppState {
        pool: pool.clone(),
        validator: validator.clone(),
    };

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
