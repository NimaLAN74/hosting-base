//! Stock Monitoring Service – binary. EU markets MVP.
//! Step 1.3 Keycloak JWT (JWKS), 1.4 public vs protected routes.

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::sync::RwLock;

use lianel_stock_monitoring_service::app::{create_router, AppState, QuoteService};
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
    let quote_service = QuoteService {
        provider: config.quote_provider.clone(),
        cache_ttl: Duration::from_secs(config.quote_cache_ttl_seconds),
        data_provider_api_key: config.data_provider_api_key.clone(),
        finnhub_api_key: config.finnhub_api_key.clone(),
        finnhub_webhook_secret: config.finnhub_webhook_secret.clone(),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()?,
        cache: Arc::new(RwLock::new(HashMap::new())),
    };

    let redis = if let Some(ref url) = config.redis_url {
        match redis::Client::open(url.as_str()) {
            Ok(client) => match client.get_multiplexed_tokio_connection().await {
                Ok(conn) => {
                    tracing::info!("Redis connected for intraday cache");
                    Some(conn)
                }
                Err(e) => {
                    tracing::warn!("Redis connection failed: {}; intraday cache disabled", e);
                    None
                }
            }
            Err(e) => {
                tracing::warn!("Redis URL invalid: {}; intraday cache disabled", e);
                None
            }
        }
    } else {
        None
    };

    let state = AppState {
        pool: pool.clone(),
        validator: validator.clone(),
        quote_service,
        finnhub_webhook_secret: config.finnhub_webhook_secret.clone(),
        redis,
    };

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
