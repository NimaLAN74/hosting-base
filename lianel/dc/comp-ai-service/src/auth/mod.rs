// Keycloak authentication module
pub mod keycloak;

pub use keycloak::*;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{Response, IntoResponse},
};
use std::sync::Arc;
use std::result::Result;
use crate::config::AppConfig;
use crate::auth::KeycloakValidator;

/// Authenticated user information extracted from Keycloak token
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub sub: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub roles: Vec<String>,
}

/// Axum extractor for authenticated users
/// Usage: `async fn handler(user: AuthenticatedUser, ...)`
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Get config from state (set by with_state)
        let config = parts
            .extensions
            .get::<Arc<AppConfig>>()
            .ok_or_else(|| {
                // Try to get from state if available
                // For now, we'll use extensions as fallback
                AuthError::ConfigMissing
            })?;
        
        let config = config.clone();

        // Get Authorization header
        let headers = &parts.headers;
        let auth_header = headers
            .get("Authorization")
            .ok_or(AuthError::MissingToken)?
            .to_str()
            .map_err(|_| AuthError::InvalidToken)?;

        // Extract token from "Bearer <token>"
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidToken)?;

        // Validate token
        let validator = KeycloakValidator::new(config);
        let claims = validator
            .validate_token(token)
            .await
            .map_err(|e| {
                tracing::warn!("Token validation failed: {}", e);
                AuthError::InvalidToken
            })?;

        // Extract roles
        let roles = claims
            .realm_access
            .map(|ra| ra.roles)
            .unwrap_or_default();

        Ok(AuthenticatedUser {
            sub: claims.sub,
            email: claims.email,
            preferred_username: claims.preferred_username,
            roles,
        })
    }
}

/// Authentication error type
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    ConfigMissing,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "Missing authorization token".to_string(),
            ),
            AuthError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "Invalid or expired token".to_string(),
            ),
            AuthError::ConfigMissing => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration missing".to_string(),
            ),
        };

        let body = serde_json::json!({
            "error": error_message
        });

        (status, axum::Json(body)).into_response()
    }
}
