// Keycloak authentication implementation
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeycloakTokenClaims {
    pub sub: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub realm_access: Option<RealmAccess>,
    pub exp: Option<usize>,
    pub iat: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmAccess {
    pub roles: Vec<String>,
}

pub struct KeycloakValidator {
    config: Arc<AppConfig>,
}

impl KeycloakValidator {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// Validate token using Keycloak's introspection endpoint.
    /// Introspection works with both user tokens and client_credentials (service account) tokens,
    /// unlike userinfo which only works for user-centric tokens.
    pub async fn validate_token(&self, token: &str) -> Result<KeycloakTokenClaims> {
        let client = reqwest::Client::new();

        let introspect_url = format!(
            "{}/realms/{}/protocol/openid-connect/token/introspect",
            self.config.keycloak_url,
            self.config.keycloak_realm
        );

        let params = [
            ("token", token),
            ("client_id", self.config.keycloak_client_id.as_str()),
            ("client_secret", self.config.keycloak_client_secret.as_str()),
        ];

        let introspect_response = client
            .post(&introspect_url)
            .form(&params)
            .send()
            .await
            .context("Failed to connect to Keycloak")?;

        if !introspect_response.status().is_success() {
            let status = introspect_response.status();
            let error_text = introspect_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Keycloak introspection returned status {}: {}",
                status,
                error_text
            ));
        }

        let introspect: Value = introspect_response
            .json()
            .await
            .context("Failed to parse introspection response")?;

        let active = introspect.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
        if !active {
            return Err(anyhow::anyhow!("Token is not active (expired or invalid)"));
        }

        let mut claims = KeycloakTokenClaims {
            sub: introspect
                .get("sub")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            email: introspect
                .get("email")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            preferred_username: introspect
                .get("preferred_username")
                .or_else(|| introspect.get("username"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            realm_access: introspect
                .get("realm_access")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            exp: introspect
                .get("exp")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize),
            iat: introspect
                .get("iat")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize),
        };

        Ok(claims)
    }
}
