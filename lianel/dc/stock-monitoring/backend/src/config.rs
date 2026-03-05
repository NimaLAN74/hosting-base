//! Configuration from environment (TDD: used by integration tests and main).

use std::env;
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_user: String,
    pub postgres_password: String,
    pub postgres_db: String,
    /// Keycloak base URL (e.g. https://auth.lianel.se). Required for JWT validation.
    pub keycloak_url: String,
    /// Keycloak realm (e.g. lianel). Required for JWT validation.
    pub keycloak_realm: String,
    /// Quotes provider identifier (MVP default: yahoo).
    pub quote_provider: String,
    /// Quote cache TTL in seconds for provider results.
    pub quote_cache_ttl_seconds: u64,
    /// Optional API key for secondary provider fallback (e.g. Alpha Vantage).
    pub data_provider_api_key: Option<String>,
    /// Optional Finnhub API key (finnhub.io). When set, Finnhub is used as a quote source.
    pub finnhub_api_key: Option<String>,
    /// Optional Finnhub webhook secret. Sent as X-Finnhub-Secret on outbound requests; verified on webhook POST.
    pub finnhub_webhook_secret: Option<String>,
    /// Optional Alpaca Markets API key ID (data.alpaca.markets). When set, Alpaca is used as a quote source for US stocks.
    pub alpaca_api_key_id: Option<String>,
    /// Optional Alpaca Markets API secret key.
    pub alpaca_api_secret_key: Option<String>,
    /// Optional Redis URL for intraday price cache (e.g. redis://:password@redis:6379/1). When set, intraday points are written/read from Redis.
    pub redis_url: Option<String>,
}

impl AppConfig {
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.postgres_user,
            self.postgres_password,
            self.postgres_host,
            self.postgres_port,
            self.postgres_db
        )
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3003".to_string())
                .parse()
                .context("Invalid PORT")?,
            postgres_host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            postgres_port: env::var("POSTGRES_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .context("Invalid POSTGRES_PORT")?,
            postgres_user: env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string()),
            postgres_password: env::var("POSTGRES_PASSWORD")
                .context("POSTGRES_PASSWORD must be set")?,
            postgres_db: env::var("POSTGRES_DB").unwrap_or_else(|_| "postgres".to_string()),
            keycloak_url: env::var("KEYCLOAK_URL").unwrap_or_else(|_| "https://auth.lianel.se".to_string()),
            keycloak_realm: env::var("KEYCLOAK_REALM").unwrap_or_else(|_| "lianel".to_string()),
            quote_provider: env::var("STOCK_MONITORING_QUOTE_PROVIDER")
                .unwrap_or_else(|_| "yahoo".to_string()),
            // Source called at most once per minute; cache TTL 60s so provider is not called more than every 1 min.
            quote_cache_ttl_seconds: env::var("STOCK_MONITORING_QUOTE_CACHE_TTL_SECONDS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("Invalid STOCK_MONITORING_QUOTE_CACHE_TTL_SECONDS")?,
            data_provider_api_key: env::var("STOCK_MONITORING_DATA_PROVIDER_API_KEY")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            finnhub_api_key: env::var("STOCK_MONITORING_FINNHUB_API_KEY")
                .or_else(|_| env::var("FINNHUB_API_KEY"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            finnhub_webhook_secret: env::var("STOCK_MONITORING_FINNHUB_WEBHOOK_SECRET")
                .or_else(|_| env::var("FINNHUB_WEBHOOK_SECRET"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            alpaca_api_key_id: env::var("STOCK_MONITORING_ALPACA_API_KEY")
                .or_else(|_| env::var("STOCK_MONITORING_ALPACA_API_KEY_ID"))
                .or_else(|_| env::var("ALPACA_API_KEY_ID"))
                .or_else(|_| env::var("ALPACA_API_KEY"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            alpaca_api_secret_key: env::var("STOCK_MONITORING_ALPACA_API_SECRET")
                .or_else(|_| env::var("STOCK_MONITORING_ALPACA_API_SECRET_KEY"))
                .or_else(|_| env::var("ALPACA_API_SECRET_KEY"))
                .or_else(|_| env::var("ALPACA_API_SECRET"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            redis_url: env::var("REDIS_URL")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
        })
    }

    /// JWKS URL for JWT validation (no client secret needed).
    pub fn keycloak_jwks_url(&self) -> String {
        format!(
            "{}/realms/{}/protocol/openid-connect/certs",
            self.keycloak_url.trim_end_matches('/'),
            self.keycloak_realm
        )
    }

    /// Primary issuer expected in JWT (for validation).
    pub fn keycloak_issuer(&self) -> String {
        format!(
            "{}/realms/{}",
            self.keycloak_url.trim_end_matches('/'),
            self.keycloak_realm
        )
    }

    /// All accepted issuers (primary + optional KEYCLOAK_ISSUER_ALT). Main app may use https://www.lianel.se/auth so tokens have that issuer.
    pub fn keycloak_issuers(&self) -> Vec<String> {
        let primary = self.keycloak_issuer();
        let alt = env::var("KEYCLOAK_ISSUER_ALT")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let mut out = vec![primary.clone()];
        if let Some(a) = alt {
            for part in a.split(',') {
                let t = part.trim();
                if !t.is_empty() && t != primary {
                    out.push(t.to_string());
                }
            }
        }
        out
    }
}
