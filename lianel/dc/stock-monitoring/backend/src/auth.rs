//! Keycloak JWT validation via JWKS (no client secret). Step 1.3.

use alcoholic_jwt::{token_kid, validate, Validation, JWKS};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::AppConfig;

/// Cached JWKS + config for validating Bearer tokens.
pub struct KeycloakJwtValidator {
    config: Arc<AppConfig>,
    jwks: RwLock<Option<JWKS>>,
}

impl KeycloakJwtValidator {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self {
            config,
            jwks: RwLock::new(None),
        }
    }

    async fn get_jwks(&self) -> anyhow::Result<JWKS> {
        {
            let g = self.jwks.read().await;
            if let Some(ref j) = *g {
                return Ok(j.clone());
            }
        }
        let url = self.config.keycloak_jwks_url();
        let jwks: JWKS = reqwest::get(&url)
            .await
            .map_err(|e| anyhow::anyhow!("JWKS fetch failed: {}", e))?
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("JWKS parse failed: {}", e))?;
        {
            let mut g = self.jwks.write().await;
            *g = Some(jwks.clone());
        }
        Ok(jwks)
    }

    /// Validate Bearer token and return subject (user_id). Returns Err on missing/invalid token.
    pub async fn validate_bearer(&self, token: &str) -> anyhow::Result<String> {
        let kid = token_kid(token).map_err(|_| anyhow::anyhow!("Invalid token format"))?
            .ok_or_else(|| anyhow::anyhow!("Token missing kid"))?;
        let jwks = self.get_jwks().await?;
        let jwk = jwks.find(&kid).ok_or_else(|| anyhow::anyhow!("Key not found in JWKS"))?;
        let issuer = self.config.keycloak_issuer();
        let validations = vec![
            Validation::Issuer(issuer),
            Validation::SubjectPresent,
        ];
        let valid = validate(token, jwk, validations)
            .map_err(|e| anyhow::anyhow!("Token validation failed: {}", e))?;
        let sub = valid
            .claims
            .get("sub")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());
        Ok(sub)
    }
}

/// User ID (Keycloak sub) extracted from valid JWT.
#[derive(Clone)]
pub struct UserId(pub String);
