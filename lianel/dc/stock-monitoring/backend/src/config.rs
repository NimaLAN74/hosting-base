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
            quote_cache_ttl_seconds: env::var("STOCK_MONITORING_QUOTE_CACHE_TTL_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("Invalid STOCK_MONITORING_QUOTE_CACHE_TTL_SECONDS")?,
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

    /// Issuer expected in JWT (for validation).
    pub fn keycloak_issuer(&self) -> String {
        format!(
            "{}/realms/{}",
            self.keycloak_url.trim_end_matches('/'),
            self.keycloak_realm
        )
    }
}
