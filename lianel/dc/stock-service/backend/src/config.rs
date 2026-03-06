//! Configuration from environment. Minimal stock service: Keycloak + IBKR OAuth only.

use std::env;
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    /// Keycloak base URL (e.g. https://auth.lianel.se). Required for JWT validation.
    pub keycloak_url: String,
    /// Keycloak realm (e.g. lianel). Required for JWT validation.
    pub keycloak_realm: String,
    // --- IBKR Web API (OAuth 1.0a first-party) ---
    pub ibkr_oauth_consumer_key: Option<String>,
    pub ibkr_oauth_access_token: Option<String>,
    pub ibkr_oauth_access_token_secret: Option<String>,
    pub ibkr_oauth_realm: Option<String>,
    pub ibkr_oauth_dh_param_path: Option<String>,
    pub ibkr_oauth_private_encryption_key_path: Option<String>,
    pub ibkr_oauth_private_signature_key_path: Option<String>,
    pub ibkr_api_base_url: String,
    pub ibkr_username: Option<String>,
    pub ibkr_password: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3003".to_string())
                .parse()
                .context("Invalid PORT")?,
            keycloak_url: env::var("KEYCLOAK_URL").unwrap_or_else(|_| "https://auth.lianel.se".to_string()),
            keycloak_realm: env::var("KEYCLOAK_REALM").unwrap_or_else(|_| "lianel".to_string()),
            ibkr_oauth_consumer_key: env::var("IBKR_OAUTH_CONSUMER_KEY")
                .or_else(|_| env::var("STOCK_MONITORING_IBKR_OAUTH_CONSUMER_KEY"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            ibkr_oauth_access_token: env::var("IBKR_OAUTH_ACCESS_TOKEN")
                .or_else(|_| env::var("STOCK_MONITORING_IBKR_OAUTH_ACCESS_TOKEN"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            ibkr_oauth_access_token_secret: env::var("IBKR_OAUTH_ACCESS_TOKEN_SECRET")
                .or_else(|_| env::var("STOCK_MONITORING_IBKR_OAUTH_ACCESS_TOKEN_SECRET"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            ibkr_oauth_realm: env::var("IBKR_OAUTH_REALM")
                .or_else(|_| env::var("STOCK_MONITORING_IBKR_OAUTH_REALM"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .or(Some("limited_poa".to_string())),
            ibkr_oauth_dh_param_path: env::var("IBKR_OAUTH_DH_PARAM_PATH")
                .or_else(|_| env::var("STOCK_MONITORING_IBKR_OAUTH_DH_PARAM_PATH"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            ibkr_oauth_private_encryption_key_path: env::var("IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH")
                .or_else(|_| env::var("STOCK_MONITORING_IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            ibkr_oauth_private_signature_key_path: env::var("IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH")
                .or_else(|_| env::var("STOCK_MONITORING_IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH"))
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            ibkr_api_base_url: env::var("IBKR_API_BASE_URL")
                .or_else(|_| env::var("STOCK_MONITORING_IBKR_API_BASE_URL"))
                .unwrap_or_else(|_| "https://api.ibkr.com/v1/api".to_string())
                .trim_end_matches('/')
                .to_string(),
            ibkr_username: env::var("IBKR_USERNAME").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            ibkr_password: env::var("IBKR_PASSWORD").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
        })
    }

    pub fn keycloak_jwks_url(&self) -> String {
        format!(
            "{}/realms/{}/protocol/openid-connect/certs",
            self.keycloak_url.trim_end_matches('/'),
            self.keycloak_realm
        )
    }

    pub fn keycloak_issuer(&self) -> String {
        format!(
            "{}/realms/{}",
            self.keycloak_url.trim_end_matches('/'),
            self.keycloak_realm
        )
    }

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

    pub fn ibkr_oauth_configured(&self) -> bool {
        self.ibkr_oauth_consumer_key.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            && self.ibkr_oauth_access_token.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            && self.ibkr_oauth_access_token_secret.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            && self.ibkr_oauth_dh_param_path.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            && self.ibkr_oauth_private_encryption_key_path.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            && self.ibkr_oauth_private_signature_key_path.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
    }
}
