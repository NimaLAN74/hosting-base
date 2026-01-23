// Keycloak authentication implementation
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use base64::{Engine, engine::general_purpose};
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

    /// Validate token using Keycloak's userinfo endpoint
    /// This works with tokens from any client, including public clients
    pub async fn validate_token(&self, token: &str) -> Result<KeycloakTokenClaims> {
        let client = reqwest::Client::new();
        
        // Use userinfo endpoint (works with tokens from any client)
        let userinfo_url = format!(
            "{}/realms/{}/protocol/openid-connect/userinfo",
            self.config.keycloak_url, self.config.keycloak_realm
        );
        
        let userinfo_response = client
            .get(&userinfo_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to connect to Keycloak")?;

        if !userinfo_response.status().is_success() {
            let status = userinfo_response.status();
            let error_text = userinfo_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Keycloak userinfo endpoint returned status {}: {}",
                status,
                error_text
            ));
        }

        // Get userinfo data
        let userinfo: Value = userinfo_response
            .json()
            .await
            .context("Failed to parse userinfo response")?;
        
        // Try to decode JWT token to get additional claims (exp, iat, realm_access)
        let mut claims = KeycloakTokenClaims {
            sub: userinfo
                .get("sub")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            email: userinfo
                .get("email")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            preferred_username: userinfo
                .get("preferred_username")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            realm_access: None,
            exp: None,
            iat: None,
        };

        // Decode JWT token to get additional claims
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 3 {
            // Try URL_SAFE_NO_PAD first, then fall back to URL_SAFE with padding
            let claims_json = general_purpose::URL_SAFE_NO_PAD
                .decode(parts[1])
                .or_else(|_| {
                    // Add padding if needed and try again
                    let mut padded = parts[1].to_string();
                    while padded.len() % 4 != 0 {
                        padded.push('=');
                    }
                    general_purpose::URL_SAFE.decode(&padded)
                });
            
            if let Ok(claims_bytes) = claims_json {
                if let Ok(jwt_claims) = serde_json::from_slice::<Value>(&claims_bytes) {
                    // Extract realm_access
                    if let Some(realm_access) = jwt_claims.get("realm_access") {
                        if let Ok(ra) = serde_json::from_value(realm_access.clone()) {
                            claims.realm_access = Some(ra);
                        }
                    }
                    
                    // Extract exp and iat
                    claims.exp = jwt_claims
                        .get("exp")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize);
                    claims.iat = jwt_claims
                        .get("iat")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize);
                }
            }
        }

        Ok(claims)
    }
}
