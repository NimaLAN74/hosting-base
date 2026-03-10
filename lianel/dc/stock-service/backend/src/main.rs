//! Stock service – minimal backend for SSO verification and IBKR. Health, status, /me.

use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lianel_stock_service::app::{create_router, AppState};
use lianel_stock_service::auth::KeycloakJwtValidator;
use lianel_stock_service::config::AppConfig;
use lianel_stock_service::ibkr::IbkrOAuthClient;
use lianel_stock_service::watchlist;

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
    let port = config.port;
    let validator = Arc::new(KeycloakJwtValidator::new(config.clone()));
    let ibkr_client = if config.ibkr_oauth_configured() {
        Some(Arc::new(IbkrOAuthClient::new(config.clone())))
    } else {
        None
    };

    let watchlist_cache = std::sync::Arc::new(tokio::sync::RwLock::new(
        watchlist::WatchlistCache::default(),
    ));
    watchlist::spawn_watchlist_ticker(
        watchlist_cache.clone(),
        ibkr_client.clone(),
        config.ibkr_api_base_url.clone(),
    );

    let state = AppState {
        validator,
        config: Some(config),
        ibkr_client,
        watchlist_cache,
    };

    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Stock service listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
