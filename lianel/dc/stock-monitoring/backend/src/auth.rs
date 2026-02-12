//! Keycloak JWT validation via JWKS (no client secret). Step 1.3.

use alcoholic_jwt::{token_kid, validate, Validation, JWKS};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct UserIdentity {
    pub user_id: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

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

    /// Validate Bearer token and return user identity claims.
    pub async fn validate_bearer_identity(&self, token: &str) -> anyhow::Result<UserIdentity> {
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
            .map_err(|e| anyhow::anyhow!("Token validation failed: {:?}", e))?;

        let claim_str = |key: &str| -> Option<String> {
            valid.claims
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
        };

        let sub = valid
            .claims
            .get("sub")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());
        let display_name = claim_str("given_name")
            .and_then(|given| {
                claim_str("family_name")
                    .map(|family| format!("{} {}", given, family))
                    .or(Some(given))
            })
            .or_else(|| claim_str("name"))
            .or_else(|| claim_str("preferred_username"))
            .or_else(|| claim_str("email"));
        let email = claim_str("email");
        Ok(UserIdentity {
            user_id: sub,
            display_name,
            email,
        })
    }

    /// Validate Bearer token and return subject (user_id). Returns Err on missing/invalid token.
    pub async fn validate_bearer(&self, token: &str) -> anyhow::Result<String> {
        Ok(self.validate_bearer_identity(token).await?.user_id)
    }
}

/// User ID (Keycloak sub) extracted from valid JWT.
#[derive(Clone)]
pub struct UserId(pub String);
