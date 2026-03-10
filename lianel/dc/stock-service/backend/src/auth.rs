//! Keycloak token validation. Uses same pattern as profile-service: validate via userinfo
//! endpoint so tokens from any Keycloak frontend URL (www.lianel.se/auth, auth.lianel.se) work.

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

/// Cached JWKS + config. Primary validation is via Keycloak userinfo (same as profile-service).
pub struct KeycloakJwtValidator {
    config: Arc<AppConfig>,
    jwks: RwLock<Option<JWKS>>,
    http_client: reqwest::Client,
}

impl KeycloakJwtValidator {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self {
            config,
            jwks: RwLock::new(None),
            http_client: reqwest::Client::new(),
        }
    }

    /// Validate token via Keycloak userinfo endpoint (same as profile-service).
    /// Accepts tokens from any issuer (www.lianel.se/auth, auth.lianel.se) so SSO works.
    pub async fn validate_bearer_identity_via_userinfo(&self, token: &str) -> anyhow::Result<UserIdentity> {
        let base = self.config.keycloak_url.trim_end_matches('/');
        let url = format!(
            "{}/realms/{}/protocol/openid-connect/userinfo",
            base,
            self.config.keycloak_realm
        );
        let res = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Userinfo request failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Keycloak userinfo returned {}: {}",
                status,
                body.lines().next().unwrap_or("")
            ));
        }

        let userinfo: Value = res
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Userinfo JSON failed: {}", e))?;

        let sub = userinfo
            .get("sub")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());

        let display_name = userinfo
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| {
                let given = userinfo.get("given_name").and_then(Value::as_str).map(str::trim);
                let family = userinfo.get("family_name").and_then(Value::as_str).map(str::trim);
                match (given, family) {
                    (Some(g), Some(f)) if !g.is_empty() || !f.is_empty() => {
                        Some(format!("{} {}", g, f).trim().to_string())
                    }
                    (Some(g), _) if !g.is_empty() => Some(g.to_string()),
                    (_, Some(f)) if !f.is_empty() => Some(f.to_string()),
                    _ => None,
                }
            })
            .or_else(|| {
                userinfo
                    .get("preferred_username")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from)
            });

        let email = userinfo
            .get("email")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);

        Ok(UserIdentity {
            user_id: sub,
            display_name,
            email,
        })
    }

    async fn get_jwks(&self) -> anyhow::Result<JWKS> {
        {
            let g = self.jwks.read().await;
            if let Some(ref j) = *g {
                return Ok(j.clone());
            }
        }
        let url = self.config.keycloak_jwks_url();
        // Keycloak JWKS may include non-signing keys (e.g. RSA-OAEP) that `alcoholic_jwt` can't deserialize.
        // Filter to signing keys (RS256 / alg missing) before deserializing.
        let raw: Value = reqwest::get(&url)
            .await
            .map_err(|e| anyhow::anyhow!("JWKS fetch failed: {}", e))?
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("JWKS JSON failed: {}", e))?;

        let filtered = match raw {
            Value::Object(mut obj) => {
                if let Some(Value::Array(keys)) = obj.get_mut("keys") {
                    keys.retain(|k| {
                        let alg = k.get("alg").and_then(Value::as_str);
                        let use_ = k.get("use").and_then(Value::as_str);
                        // Prefer signature keys. Keep if explicitly RS256, or if alg missing but use=sig.
                        matches!(alg, Some("RS256")) || (alg.is_none() && matches!(use_, Some("sig")))
                    });
                }
                Value::Object(obj)
            }
            other => other,
        };

        let jwks: JWKS = serde_json::from_value(filtered)
            .map_err(|e| anyhow::anyhow!("JWKS parse failed: {}", e))?;
        {
            let mut g = self.jwks.write().await;
            *g = Some(jwks.clone());
        }
        Ok(jwks)
    }

    /// Validate Bearer token and return user identity claims.
    /// Tries each accepted issuer (primary + KEYCLOAK_ISSUER_ALT) so tokens from www.lianel.se/auth and auth.lianel.se both work.
    pub async fn validate_bearer_identity(&self, token: &str) -> anyhow::Result<UserIdentity> {
        let kid = token_kid(token).map_err(|_| anyhow::anyhow!("Invalid token format"))?
            .ok_or_else(|| anyhow::anyhow!("Token missing kid"))?;
        let jwks = self.get_jwks().await?;
        let jwk = jwks.find(&kid).ok_or_else(|| anyhow::anyhow!("Key not found in JWKS"))?;
        let issuers = self.config.keycloak_issuers();
        let mut last_err = None;
        for issuer in &issuers {
            let validations = vec![
                Validation::Issuer(issuer.clone()),
                Validation::SubjectPresent,
            ];
            match validate(token, jwk, validations) {
                Ok(valid) => {
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
                    return Ok(UserIdentity {
                        user_id: sub,
                        display_name,
                        email,
                    });
                }
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("Token validation failed (issuer {}): {:?}", issuer, e));
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("No issuer matched")))
    }

    /// Validate Bearer token and return subject (user_id). Returns Err on missing/invalid token.
    pub async fn validate_bearer(&self, token: &str) -> anyhow::Result<String> {
        Ok(self.validate_bearer_identity(token).await?.user_id)
    }
}

/// User ID (Keycloak sub) extracted from valid JWT.
#[derive(Clone)]
pub struct UserId(pub String);
