//! Stock Monitoring Service – binary. EU markets MVP.
//! Step 1.3 Keycloak JWT (JWKS), 1.4 public vs protected routes.

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::sync::RwLock;

use lianel_stock_monitoring_service::app::{create_router, refresh_quotes_cache_loop, AppState, QuoteService};
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
        alpaca_api_key_id: config.alpaca_api_key_id.clone(),
        alpaca_api_secret_key: config.alpaca_api_secret_key.clone(),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()?,
        cache: Arc::new(RwLock::new(HashMap::new())),
        day_history: Arc::new(RwLock::new(HashMap::new())),
    };

    #[cfg(feature = "redis")]
    let redis = if let Some(ref url) = config.redis_url {
        match redis::Client::open(url.as_str()) {
            Ok(client) => match client.get_multiplexed_tokio_connection().await {
                Ok(conn) => {
                    tracing::info!("Redis connected for intraday cache (price-history chart will show all points from Redis)");
                    Some(conn)
                }
                Err(e) => {
                    tracing::warn!("Redis connection failed: {}; intraday cache disabled (chart uses Postgres + in-memory only)", e);
                    None
                }
            }
            Err(e) => {
                tracing::warn!("Redis URL invalid: {}; intraday cache disabled", e);
                None
            }
        }
    } else {
        tracing::info!("REDIS_URL not set; intraday chart uses Postgres + in-memory only");
        None
    };

    let state = AppState {
        pool: pool.clone(),
        validator: validator.clone(),
        quote_service,
        finnhub_webhook_secret: config.finnhub_webhook_secret.clone(),
        #[cfg(feature = "redis")]
        redis,
    };

    // Refresh cache every 60s from watchlist_items so fetch_finnhub_quotes (and other providers) run every TTL even when no request arrives.
    tokio::spawn(refresh_quotes_cache_loop(state.clone()));

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Quote providers: Finnhub={}, Alpaca={}, Yahoo=yes, Stooq=yes, Alpha Vantage={}",
        if config.finnhub_api_key.is_some() { "yes" } else { "no (set FINNHUB_API_KEY for multi-provider)" },
        if config.alpaca_api_key_id.is_some() && config.alpaca_api_secret_key.is_some() { "yes" } else { "no (set ALPACA_API_KEY_ID + ALPACA_API_SECRET_KEY)" },
        if config.data_provider_api_key.is_some() { "yes" } else { "no" });
    tracing::info!("listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
