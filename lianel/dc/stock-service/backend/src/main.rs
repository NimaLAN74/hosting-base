//! Stock service – minimal backend for SSO verification and IBKR. Health, status, /me.

use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lianel_stock_service::app::{create_router, AppState};
use lianel_stock_service::auth::KeycloakJwtValidator;
use lianel_stock_service::config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;
    let config = Arc::new(config);
    let validator = Arc::new(KeycloakJwtValidator::new(config.clone()));

    let state = AppState {
        validator,
        config: Some(config),
    };

    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Stock service listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
