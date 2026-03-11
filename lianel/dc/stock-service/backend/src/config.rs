//! Configuration from environment. Minimal stock service: Keycloak + IBKR OAuth only.

use std::env;
use std::path::Path;
use anyhow::{Context, Result};

fn read_gateway_session_cookie() -> Option<String> {
    if let Ok(v) = env::var("IBKR_GATEWAY_SESSION_COOKIE") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(v);
        }
    }
    if let Ok(path) = env::var("IBKR_GATEWAY_SESSION_COOKIE_FILE") {
        let path = path.trim();
        if !path.is_empty() && Path::new(path).exists() {
            if let Ok(s) = std::fs::read_to_string(path) {
                let v = s.trim().to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

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
    /// When using a local Client Portal Gateway (HTTPS with self-signed cert), set to true to skip TLS verify.
    pub ibkr_insecure_skip_tls_verify: bool,
    pub ibkr_username: Option<String>,
    pub ibkr_password: Option<String>,
    /// When using Client Portal Gateway (base URL points to Gateway), optional session cookie value (the part after "api=").
    /// If set, we use this instead of OAuth/tickle for session.
    pub ibkr_gateway_session_cookie: Option<String>,
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
                .or(Some("test_realm".to_string())),
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
            ibkr_insecure_skip_tls_verify: env::var("IBKR_INSECURE_SKIP_TLS_VERIFY")
                .unwrap_or_else(|_| "false".to_string())
                .trim()
                .eq_ignore_ascii_case("true")
                || env::var("IBKR_INSECURE_SKIP_TLS_VERIFY").unwrap_or_default().trim() == "1",
            ibkr_username: env::var("IBKR_USERNAME").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            ibkr_password: env::var("IBKR_PASSWORD").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            ibkr_gateway_session_cookie: read_gateway_session_cookie(),
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

    /// True when using Client Portal Gateway with a pre-obtained session cookie (no OAuth).
    pub fn is_ibkr_gateway_cookie_mode(&self) -> bool {
        let base = self.ibkr_api_base_url.as_str();
        let looks_like_gateway = base.contains("ibkr-gateway") || base.contains(":5000");
        looks_like_gateway && self.ibkr_gateway_session_cookie.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
    }
}
