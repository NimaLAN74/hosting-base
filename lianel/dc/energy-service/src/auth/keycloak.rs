// Reuse Keycloak validation pattern from profile-service
// This is a simplified version - can be enhanced later

use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeycloakTokenClaims {
    pub sub: String,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub preferred_username: Option<String>,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub name: Option<String>,
    pub email_verified: Option<bool>,
}

pub struct KeycloakValidator {
    keycloak_url: String,
    keycloak_realm: String,
}

impl KeycloakValidator {
    pub fn new(keycloak_url: String, keycloak_realm: String) -> Self {
        Self {
            keycloak_url,
            keycloak_realm,
        }
    }

    pub async fn validate_token(&self, token: &str) -> Result<KeycloakTokenClaims> {
        let client = reqwest::Client::new();
        
        let userinfo_url = format!(
            "{}/realms/{}/protocol/openid-connect/userinfo",
            self.keycloak_url, self.keycloak_realm
        );
        
        let response = client
            .get(&userinfo_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Token validation failed: {}", response.status());
        }

        let userinfo: Value = response.json().await?;
        
        Ok(KeycloakTokenClaims {
            sub: userinfo.get("sub")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default(),
            exp: None,
            iat: None,
            preferred_username: userinfo.get("preferred_username")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            email: userinfo.get("email")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            given_name: userinfo.get("given_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            family_name: userinfo.get("family_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            name: userinfo.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            email_verified: userinfo.get("email_verified")
                .and_then(|v| v.as_bool()),
        })
    }
}

