use axum::{
    extract::Path,
    http::{self, HeaderMap, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, AllowOrigin, AllowHeaders};
use tower_http::trace::TraceLayer;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use serde_json::Value;
use base64::{Engine, engine::general_purpose};
use axum::extract::Query;
use http::HeaderValue;

#[derive(Debug, Clone)]
struct AppConfig {
    keycloak_url: String,
    keycloak_realm: String,
    keycloak_admin_user: String,
    keycloak_admin_password: String,
    backend_client_secret: String,
}

#[derive(Debug, Clone)]
struct KeycloakValidator {
    config: AppConfig,
}

impl KeycloakValidator {
    fn new(config: AppConfig) -> Self {
        Self { config }
    }

    // Validate token using Keycloak's userinfo endpoint (works with public client tokens)
    // Falls back to introspection if userinfo fails
    async fn validate_token(&self, token: &str) -> Result<KeycloakTokenClaims, anyhow::Error> {
        let client = reqwest::Client::new();
        
        // First, try userinfo endpoint (works with tokens from any client, including public clients)
        let userinfo_url = format!(
            "{}/realms/{}/protocol/openid-connect/userinfo",
            self.config.keycloak_url, self.config.keycloak_realm
        );
        
        let userinfo_response = client
            .get(&userinfo_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if userinfo_response.status().is_success() {
            // Userinfo endpoint works - token is valid
            let userinfo: Value = userinfo_response.json().await?;
            
            // Decode token to get additional claims (exp, iat, etc.)
            // JWT tokens use URL-safe base64 encoding, may need padding
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
                    if let Ok(mut claims) = serde_json::from_slice::<KeycloakTokenClaims>(&claims_bytes) {
                        // Update with userinfo data (more reliable)
                        claims.preferred_username = userinfo.get("preferred_username")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        claims.email = userinfo.get("email")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        claims.given_name = userinfo.get("given_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        claims.family_name = userinfo.get("family_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        claims.name = userinfo.get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        claims.email_verified = userinfo.get("email_verified")
                            .and_then(|v| v.as_bool());
                        return Ok(claims);
                    }
                }
            }
            
            // Fallback: construct claims from userinfo
            return Ok(KeycloakTokenClaims {
                sub: userinfo.get("sub")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
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
                exp: userinfo.get("exp")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                iat: userinfo.get("iat")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize),
                iss: format!("{}/realms/{}", self.config.keycloak_url, self.config.keycloak_realm),
                aud: None,
                realm_access: None,
            });
        }
        
        // Fallback to introspection (for confidential client tokens)
        let admin_token = get_admin_token(&self.config).await?;
        
        let introspect_response = client
            .post(format!(
                "{}/realms/{}/protocol/openid-connect/token/introspect",
                self.config.keycloak_url, self.config.keycloak_realm
            ))
            .form(&[
                ("token", token),
                ("client_id", "backend-api"),
                ("client_secret", &self.config.backend_client_secret),
            ])
            .send()
            .await?;

        if introspect_response.status().is_success() {
            let introspect_result: Value = introspect_response.json().await?;
        
            if introspect_result.get("active").and_then(|v| v.as_bool()).unwrap_or(false) {
                // Parse token to get claims
                // JWT tokens use URL-safe base64 encoding, may need padding
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
                        if let Ok(claims) = serde_json::from_slice::<KeycloakTokenClaims>(&claims_bytes) {
                            return Ok(claims);
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Token validation failed: userinfo and introspection both failed"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct KeycloakTokenClaims {
    sub: String,
    preferred_username: Option<String>,
    email: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    name: Option<String>,
    email_verified: Option<bool>,
    exp: usize,
    iat: Option<usize>,
    iss: String,
    aud: Option<String>,
    realm_access: Option<RealmAccess>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RealmAccess {
    roles: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct UserProfile {
    id: String,
    username: String,
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    firstName: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lastName: Option<String>,
    name: String,
    email_verified: bool,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct UpdateProfileRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    firstName: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lastName: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct CreateUserRequest {
    username: String,
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    firstName: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lastName: Option<String>,
    password: String,
    enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AdminChangePasswordRequest {
    new_password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct UserListResponse {
    users: Vec<UserProfile>,
    total: usize,
}

#[derive(Debug, Deserialize)]
struct ListUsersQuery {
    page: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct SuccessResponse {
    message: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct HealthResponse {
    status: String,
    service: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_profile,
        update_profile,
        change_password,
        health,
        check_admin,
        list_users,
        get_user_by_id,
        create_user,
        update_user,
        delete_user,
        admin_change_password
    ),
    components(schemas(
        UserProfile,
        UpdateProfileRequest,
        ChangePasswordRequest,
        CreateUserRequest,
        AdminChangePasswordRequest,
        SuccessResponse,
        ErrorResponse,
        HealthResponse,
        UserListResponse
    )),
    tags(
        (name = "profile", description = "User profile management endpoints"),
        (name = "admin", description = "Admin user management endpoints"),
        (name = "health", description = "Health check endpoint")
    ),
    info(
        title = "Lianel Profile Service API",
        description = "API for managing user profiles via Keycloak",
        version = "1.0.0"
    ),
    servers(
        (url = "https://www.lianel.se", description = "Production server")
    )
)]
struct ApiDoc;

// Extract and validate Keycloak token from Authorization header
async fn validate_keycloak_token(
    headers: &HeaderMap,
    validator: &KeycloakValidator,
) -> Result<KeycloakTokenClaims, StatusCode> {
    // Get Authorization header
    let auth_header = headers
        .get("authorization")
        .or_else(|| headers.get("Authorization"))
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token via introspection
    validator.validate_token(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

// Extract user info from Keycloak token (backward compatibility with header-based auth)
async fn extract_user_from_token_or_headers(
    headers: &HeaderMap,
    validator: Option<&KeycloakValidator>,
) -> Result<(String, String, Option<String>), StatusCode> {
    // Try Keycloak token first
    if let Some(v) = validator {
        match validate_keycloak_token(headers, v).await {
            Ok(claims) => {
                let username = claims.preferred_username
                    .unwrap_or_else(|| claims.sub.clone());
                let email = claims.email
                    .unwrap_or_else(|| format!("{}@unknown", username));
                return Ok((username, email, Some(claims.sub)));
            }
            Err(_) => {
                // Fall back to headers if token validation fails
            }
        }
    }

    // Fallback to header-based extraction (for backward compatibility)
    let username = headers
        .get("x-auth-request-preferred-username")
        .or_else(|| headers.get("x-preferred-username"))
        .or_else(|| headers.get("x-user"))
        .or_else(|| headers.get("x-auth-request-user"))
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let email = headers
        .get("x-email")
        .or_else(|| headers.get("x-auth-request-email"))
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = headers
        .get("x-auth-request-user-id")
        .or_else(|| headers.get("x-user-id"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    Ok((username.to_string(), email.to_string(), user_id))
}

// Get admin token from Keycloak
async fn get_admin_token(config: &AppConfig) -> Result<String, anyhow::Error> {
    let client = reqwest::Client::new();
    let params = [
        ("username", config.keycloak_admin_user.as_str()),
        ("password", config.keycloak_admin_password.as_str()),
        ("grant_type", "password"),
        ("client_id", "admin-cli"),
    ];

    let response = client
        .post(format!(
            "{}/realms/master/protocol/openid-connect/token",
            config.keycloak_url
        ))
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let text = response.text().await?;
        return Err(anyhow::anyhow!("Failed to get admin token: {}", text));
    }

    let json: serde_json::Value = response.json().await?;
    json.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No access token in response"))
}

// Get user ID from username or email
async fn get_user_id(
    config: &AppConfig,
    admin_token: &str,
    identifier: &str,
) -> Result<String, anyhow::Error> {
    let client = reqwest::Client::new();

    // Try username first
    let response = client
        .get(format!(
            "{}/admin/realms/{}/users",
            config.keycloak_url, config.keycloak_realm
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .query(&[("username", identifier), ("exact", "true")])
        .send()
        .await?;

    if response.status().is_success() {
        let users: Vec<serde_json::Value> = response.json().await?;
        if let Some(user) = users.first() {
            if let Some(id) = user.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }
    }

    // Try email
    let response = client
        .get(format!(
            "{}/admin/realms/{}/users",
            config.keycloak_url, config.keycloak_realm
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .query(&[("email", identifier), ("exact", "true")])
        .send()
        .await?;

    if response.status().is_success() {
        let users: Vec<serde_json::Value> = response.json().await?;
        if let Some(user) = users.first() {
            if let Some(id) = user.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }
    }

    Err(anyhow::anyhow!("User not found"))
}

// Check if user has admin role
async fn is_admin(
    config: &AppConfig,
    admin_token: &str,
    user_id: &str,
) -> Result<bool, anyhow::Error> {
    let client = reqwest::Client::new();
    
    // Get realm roles for the user
    let response = client
        .get(format!(
            "{}/admin/realms/{}/users/{}/role-mappings/realm",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(false);
    }

    let roles: Vec<serde_json::Value> = response.json().await?;
    
    // Check if user has "admin" or "realm-admin" role
    for role in roles {
        if let Some(name) = role.get("name").and_then(|v| v.as_str()) {
            if name.to_lowercase() == "admin" || name.to_lowercase() == "realm-admin" {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Get current user's profile
#[utoipa::path(
    get,
    path = "/api/profile",
    tag = "profile",
    responses(
        (status = 200, description = "User profile retrieved successfully", body = UserProfile),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn get_profile(
    headers: HeaderMap,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
) -> Result<Json<UserProfile>, (StatusCode, Json<ErrorResponse>)> {
    let (username, _email, user_id_from_header) = extract_user_from_token_or_headers(&headers, Some(&*validator))
        .await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    // If we have a user ID from header (UUID), use it directly, otherwise look up by username/email
    let user_id = if let Some(ref uid) = user_id_from_header {
        uid.clone()
    } else if username.len() == 36 && username.contains('-') {
        // Likely a UUID, try using it directly
        username.clone()
    } else {
        // Look up by username or email
        get_user_id(&config, &admin_token, &username)
            .await
            .map_err(|e| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "User not found".to_string(),
                        details: Some(e.to_string()),
                    }),
                )
            })?
    };

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "{}/admin/realms/{}/users/{}",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to fetch user".to_string(),
                details: Some(text),
            }),
        ));
    }

    let user: serde_json::Value = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to parse user data".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let first_name = user.get("firstName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let last_name = user.get("lastName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let name = if let (Some(fn_val), Some(ln_val)) = (&first_name, &last_name) {
        format!("{} {}", fn_val, ln_val)
    } else {
        first_name
            .clone()
            .or(last_name.clone())
            .unwrap_or_else(|| user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string())
    };

    let profile = UserProfile {
        id: user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        username: user
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        email: user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        firstName: first_name,
        lastName: last_name,
        name,
        email_verified: user.get("emailVerified").and_then(|v| v.as_bool()).unwrap_or(false),
        enabled: user.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    Ok(Json(profile))
}

/// Update user profile
#[utoipa::path(
    put,
    path = "/api/profile",
    tag = "profile",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated successfully", body = UserProfile),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn update_profile(
    headers: HeaderMap,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfile>, (StatusCode, Json<ErrorResponse>)> {
    if payload.firstName.is_none() && payload.lastName.is_none() && payload.email.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No fields to update".to_string(),
                details: None,
            }),
        ));
    }

    let (username, _email, user_id_from_header) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let user_id = if username.len() == 36 && username.matches('-').count() == 4 {
        username.clone()
    } else if let Some(ref uid) = user_id_from_header {
        uid.clone()
    } else {
        get_user_id(&config, &admin_token, &username)
            .await
            .map_err(|e| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "User not found".to_string(),
                        details: Some(e.to_string()),
                    }),
                )
            })?
    };

    // Build update payload
    let mut update_payload = serde_json::Map::new();
    if let Some(ref fn_val) = payload.firstName {
        update_payload.insert("firstName".to_string(), serde_json::Value::String(fn_val.clone()));
    }
    if let Some(ref ln_val) = payload.lastName {
        update_payload.insert("lastName".to_string(), serde_json::Value::String(ln_val.clone()));
    }
    if let Some(ref email) = payload.email {
        update_payload.insert("email".to_string(), serde_json::Value::String(email.clone()));
        update_payload.insert("emailVerified".to_string(), serde_json::Value::Bool(false));
    }

    let client = reqwest::Client::new();
    let response = client
        .put(format!(
            "{}/admin/realms/{}/users/{}",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .json(&update_payload)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to update profile".to_string(),
                details: Some(text),
            }),
        ));
    }

    // Fetch updated user
    let response = client
        .get(format!(
            "{}/admin/realms/{}/users/{}",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch updated user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let user: serde_json::Value = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to parse updated user data".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let first_name = user.get("firstName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let last_name = user.get("lastName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let name = if let (Some(fn_val), Some(ln_val)) = (&first_name, &last_name) {
        format!("{} {}", fn_val, ln_val)
    } else {
        first_name
            .clone()
            .or(last_name.clone())
            .unwrap_or_else(|| user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string())
    };

    let profile = UserProfile {
        id: user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        username: user
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        email: user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        firstName: first_name,
        lastName: last_name,
        name,
        email_verified: user.get("emailVerified").and_then(|v| v.as_bool()).unwrap_or(false),
        enabled: user.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    Ok(Json(profile))
}

/// Change user password
#[utoipa::path(
    post,
    path = "/api/profile/change-password",
    tag = "profile",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized or incorrect current password", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn change_password(
    headers: HeaderMap,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.new_password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "New password must be at least 8 characters long".to_string(),
                details: None,
            }),
        ));
    }

    let (username, email, user_id_from_header) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    // Get actual username for password verification (not UUID)
    // If username is a UUID, use email prefix as login username
    let login_username = if username.len() == 36 && username.matches('-').count() == 4 {
        email.split('@').next().unwrap_or(&email)
    } else {
        &username
    };

    // Verify current password
    let client = reqwest::Client::new();
    let params = [
        ("username", login_username),
        ("password", payload.current_password.as_str()),
        ("grant_type", "password"),
        ("client_id", "oauth2-proxy"),
    ];

    let response = client
        .post(format!(
            "{}/realms/{}/protocol/openid-connect/token",
            config.keycloak_url, config.keycloak_realm
        ))
        .form(&params)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to verify current password".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Current password is incorrect".to_string(),
                details: None,
            }),
        ));
    }

    // Get admin token and update password
    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let user_id = if username.len() == 36 && username.matches('-').count() == 4 {
        username.clone()
    } else if let Some(ref uid) = user_id_from_header {
        uid.clone()
    } else {
        get_user_id(&config, &admin_token, login_username)
            .await
            .map_err(|e| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "User not found".to_string(),
                        details: Some(e.to_string()),
                    }),
                )
            })?
    };

    let password_payload = serde_json::json!({
        "type": "password",
        "value": payload.new_password,
        "temporary": false
    });

    let response = client
        .put(format!(
            "{}/admin/realms/{}/users/{}/reset-password",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .json(&password_payload)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to change password".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to change password".to_string(),
                details: Some(text),
            }),
        ));
    }

    Ok(Json(SuccessResponse {
        message: "Password changed successfully".to_string(),
    }))
}

/// Check if current user is admin
#[utoipa::path(
    get,
    path = "/api/admin/check",
    tag = "admin",
    responses(
        (status = 200, description = "Admin status", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn check_admin(
    headers: HeaderMap,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let (_username, _email, user_id_from_header) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let user_id = user_id_from_header.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "User ID not found".to_string(),
                details: None,
            }),
        )
    })?;

    let is_admin_user = is_admin(&config, &admin_token, &user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to check admin status".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "isAdmin": is_admin_user
    })))
}

/// List all users (admin only)
#[utoipa::path(
    get,
    path = "/api/admin/users",
    tag = "admin",
    responses(
        (status = 200, description = "List of users", body = UserListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not admin", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn list_users(
    headers: HeaderMap,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
    Query(params): Query<ListUsersQuery>,
) -> Result<Json<UserListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_username, _email, user_id_from_header) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let user_id = user_id_from_header.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "User ID not found".to_string(),
                details: None,
            }),
        )
    })?;

    // Check if user is admin
    if !is_admin(&config, &admin_token, &user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to check admin status".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })? {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Admin access required".to_string(),
                details: None,
            }),
        ));
    }

    let client = reqwest::Client::new();

    // Pagination and search mapping to Keycloak parameters
    let page = params.page.unwrap_or(1).max(1);
    let size = params.size.unwrap_or(20).clamp(1, 100);
    let first = (page - 1) * size;

    // Build query for users endpoint
    let mut req = client
        .get(format!(
            "{}/admin/realms/{}/users",
            config.keycloak_url, config.keycloak_realm
        ))
        .header("Authorization", format!("Bearer {}", admin_token));

    // Apply pagination
    req = req.query(&[("first", first.to_string()), ("max", size.to_string())]);

    // Apply search if provided
    if let Some(search) = params.search.as_ref() {
        if !search.trim().is_empty() {
            req = req.query(&[("search", search.trim())]);
        }
    }

    let response = req
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch users".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to fetch users".to_string(),
                details: Some(text),
            }),
        ));
    }

    let users: Vec<serde_json::Value> = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to parse users".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let user_profiles: Vec<UserProfile> = users
        .into_iter()
        .map(|user| {
            let first_name = user.get("firstName").and_then(|v| v.as_str()).map(|s| s.to_string());
            let last_name = user.get("lastName").and_then(|v| v.as_str()).map(|s| s.to_string());
            let name = if let (Some(fn_val), Some(ln_val)) = (&first_name, &last_name) {
                format!("{} {}", fn_val, ln_val)
            } else {
                first_name
                    .clone()
                    .or(last_name.clone())
                    .unwrap_or_else(|| user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string())
            };

            UserProfile {
                id: user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                username: user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                email: user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                firstName: first_name,
                lastName: last_name,
                name,
                email_verified: user.get("emailVerified").and_then(|v| v.as_bool()).unwrap_or(false),
                enabled: user.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            }
        })
        .collect();

    // Fetch total count using Keycloak count endpoint (with same filters)
    let mut count_req = client
        .get(format!(
            "{}/admin/realms/{}/users/count",
            config.keycloak_url, config.keycloak_realm
        ))
        .header("Authorization", format!("Bearer {}", admin_token));

    if let Some(search) = params.search.as_ref() {
        if !search.trim().is_empty() {
            count_req = count_req.query(&[("search", search.trim())]);
        }
    }

    let total = match count_req.send().await {
        Ok(rsp) if rsp.status().is_success() => match rsp.json::<serde_json::Value>().await {
            Ok(val) => val.as_u64().unwrap_or(user_profiles.len() as u64) as usize,
            Err(_) => user_profiles.len(),
        },
        _ => user_profiles.len(),
    };

    Ok(Json(UserListResponse { total, users: user_profiles }))
}

/// Get user by ID (admin only)
#[utoipa::path(
    get,
    path = "/api/admin/users/{user_id}",
    tag = "admin",
    responses(
        (status = 200, description = "User profile", body = UserProfile),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not admin", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn get_user_by_id(
    headers: HeaderMap,
    Path(user_id): Path<String>,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
) -> Result<Json<UserProfile>, (StatusCode, Json<ErrorResponse>)> {
    let (_username, _email, current_user_id) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let current_user_id = current_user_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "User ID not found".to_string(),
                details: None,
            }),
        )
    })?;

    // Check if user is admin
    if !is_admin(&config, &admin_token, &current_user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to check admin status".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })? {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Admin access required".to_string(),
                details: None,
            }),
        ));
    }

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "{}/admin/realms/{}/users/{}",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "User not found".to_string(),
                details: Some(text),
            }),
        ));
    }

    let user: serde_json::Value = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to parse user data".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let first_name = user.get("firstName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let last_name = user.get("lastName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let name = if let (Some(fn_val), Some(ln_val)) = (&first_name, &last_name) {
        format!("{} {}", fn_val, ln_val)
    } else {
        first_name
            .clone()
            .or(last_name.clone())
            .unwrap_or_else(|| user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string())
    };

    let profile = UserProfile {
        id: user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        username: user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        email: user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        firstName: first_name,
        lastName: last_name,
        name,
        email_verified: user.get("emailVerified").and_then(|v| v.as_bool()).unwrap_or(false),
        enabled: user.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    Ok(Json(profile))
}

/// Create new user (admin only)
#[utoipa::path(
    post,
    path = "/api/admin/users",
    tag = "admin",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = UserProfile),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not admin", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn create_user(
    headers: HeaderMap,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserProfile>, (StatusCode, Json<ErrorResponse>)> {
    if payload.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password must be at least 8 characters long".to_string(),
                details: None,
            }),
        ));
    }

    let (_username, _email, user_id_from_header) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let user_id = user_id_from_header.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "User ID not found".to_string(),
                details: None,
            }),
        )
    })?;

    // Check if user is admin
    if !is_admin(&config, &admin_token, &user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to check admin status".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })? {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Admin access required".to_string(),
                details: None,
            }),
        ));
    }

    // Create user payload
    let mut user_payload = serde_json::json!({
        "username": payload.username,
        "email": payload.email,
        "enabled": payload.enabled.unwrap_or(true),
        "emailVerified": false,
        "credentials": [{
            "type": "password",
            "value": payload.password,
            "temporary": false
        }]
    });

    if let Some(ref fn_val) = payload.firstName {
        user_payload["firstName"] = serde_json::Value::String(fn_val.clone());
    }
    if let Some(ref ln_val) = payload.lastName {
        user_payload["lastName"] = serde_json::Value::String(ln_val.clone());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "{}/admin/realms/{}/users",
            config.keycloak_url, config.keycloak_realm
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .json(&user_payload)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Failed to create user".to_string(),
                details: Some(text),
            }),
        ));
    }

    // Get the created user ID from Location header
    let location = response.headers().get("location");
    let created_user_id = if let Some(loc) = location {
        loc.to_str().ok()
            .and_then(|s| s.split('/').last().map(|s| s.to_string()))
    } else {
        // Fallback: search for the user by username
        get_user_id(&config, &admin_token, &payload.username).await.ok()
    };

    let created_user_id = created_user_id.ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "User created but ID not found".to_string(),
                details: None,
            }),
        )
    })?;

    // Fetch the created user
    let response = client
        .get(format!(
            "{}/admin/realms/{}/users/{}",
            config.keycloak_url, config.keycloak_realm, created_user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch created user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let user: serde_json::Value = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to parse user data".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let first_name = user.get("firstName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let last_name = user.get("lastName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let name = if let (Some(fn_val), Some(ln_val)) = (&first_name, &last_name) {
        format!("{} {}", fn_val, ln_val)
    } else {
        first_name
            .clone()
            .or(last_name.clone())
            .unwrap_or_else(|| user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string())
    };

    let profile = UserProfile {
        id: user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        username: user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        email: user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        firstName: first_name,
        lastName: last_name,
        name,
        email_verified: user.get("emailVerified").and_then(|v| v.as_bool()).unwrap_or(false),
        enabled: user.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    Ok(Json(profile))
}

/// Update user (admin only)
#[utoipa::path(
    put,
    path = "/api/admin/users/{user_id}",
    tag = "admin",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserProfile),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not admin", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn update_user(
    headers: HeaderMap,
    Path(user_id): Path<String>,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfile>, (StatusCode, Json<ErrorResponse>)> {
    let (_username, _email, current_user_id) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let current_user_id = current_user_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "User ID not found".to_string(),
                details: None,
            }),
        )
    })?;

    // Check if user is admin
    if !is_admin(&config, &admin_token, &current_user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to check admin status".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })? {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Admin access required".to_string(),
                details: None,
            }),
        ));
    }

    // Build update payload
    let mut update_payload = serde_json::Map::new();
    if let Some(ref fn_val) = payload.firstName {
        update_payload.insert("firstName".to_string(), serde_json::Value::String(fn_val.clone()));
    }
    if let Some(ref ln_val) = payload.lastName {
        update_payload.insert("lastName".to_string(), serde_json::Value::String(ln_val.clone()));
    }
    if let Some(ref email) = payload.email {
        update_payload.insert("email".to_string(), serde_json::Value::String(email.clone()));
        update_payload.insert("emailVerified".to_string(), serde_json::Value::Bool(false));
    }

    let client = reqwest::Client::new();
    let response = client
        .put(format!(
            "{}/admin/realms/{}/users/{}",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .json(&update_payload)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to update user".to_string(),
                details: Some(text),
            }),
        ));
    }

    // Fetch updated user
    let response = client
        .get(format!(
            "{}/admin/realms/{}/users/{}",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch updated user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let user: serde_json::Value = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to parse updated user data".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let first_name = user.get("firstName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let last_name = user.get("lastName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let name = if let (Some(fn_val), Some(ln_val)) = (&first_name, &last_name) {
        format!("{} {}", fn_val, ln_val)
    } else {
        first_name
            .clone()
            .or(last_name.clone())
            .unwrap_or_else(|| user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string())
    };

    let profile = UserProfile {
        id: user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        username: user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        email: user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        firstName: first_name,
        lastName: last_name,
        name,
        email_verified: user.get("emailVerified").and_then(|v| v.as_bool()).unwrap_or(false),
        enabled: user.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    Ok(Json(profile))
}

/// Delete user (admin only)
#[utoipa::path(
    delete,
    path = "/api/admin/users/{user_id}",
    tag = "admin",
    responses(
        (status = 200, description = "User deleted successfully", body = SuccessResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not admin", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn delete_user(
    headers: HeaderMap,
    Path(user_id): Path<String>,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_username, _email, current_user_id) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let current_user_id = current_user_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "User ID not found".to_string(),
                details: None,
            }),
        )
    })?;

    // Check if user is admin
    if !is_admin(&config, &admin_token, &current_user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to check admin status".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })? {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Admin access required".to_string(),
                details: None,
            }),
        ));
    }

    // Prevent deleting yourself
    if user_id == current_user_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Cannot delete your own account".to_string(),
                details: None,
            }),
        ));
    }

    let client = reqwest::Client::new();
    let response = client
        .delete(format!(
            "{}/admin/realms/{}/users/{}",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete user".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "User not found or could not be deleted".to_string(),
                details: Some(text),
            }),
        ));
    }

    Ok(Json(SuccessResponse {
        message: "User deleted successfully".to_string(),
    }))
}

/// Change user password (admin only, no current password required)
#[utoipa::path(
    post,
    path = "/api/admin/users/{user_id}/change-password",
    tag = "admin",
    request_body = AdminChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not admin", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    ),
    security(
        ("oauth2" = [])
    )
)]
async fn admin_change_password(
    headers: HeaderMap,
    Path(user_id): Path<String>,
    axum::extract::State((config, validator)): axum::extract::State<(AppConfig, Arc<KeycloakValidator>)>,
    Json(payload): Json<AdminChangePasswordRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.new_password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "New password must be at least 8 characters long".to_string(),
                details: None,
            }),
        ));
    }

    let (_username, _email, current_user_id) = extract_user_from_token_or_headers(&headers, Some(&*validator)).await
        .map_err(|e| (e, Json(ErrorResponse { error: "Unauthorized".to_string(), details: None })))?;

    let admin_token = get_admin_token(&config)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Keycloak".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let current_user_id = current_user_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "User ID not found".to_string(),
                details: None,
            }),
        )
    })?;

    // Check if user is admin
    if !is_admin(&config, &admin_token, &current_user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to check admin status".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })? {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Admin access required".to_string(),
                details: None,
            }),
        ));
    }

    let password_payload = serde_json::json!({
        "type": "password",
        "value": payload.new_password,
        "temporary": false
    });

    let client = reqwest::Client::new();
    let response = client
        .put(format!(
            "{}/admin/realms/{}/users/{}/reset-password",
            config.keycloak_url, config.keycloak_realm, user_id
        ))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .json(&password_payload)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to change password".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to change password".to_string(),
                details: Some(text),
            }),
        ));
    }

    Ok(Json(SuccessResponse {
        message: "Password changed successfully".to_string(),
    }))
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/profile/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "lianel-profile-service".to_string(),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig {
        keycloak_url: env::var("KEYCLOAK_URL").unwrap_or_else(|_| "http://keycloak:8080".to_string()),
        keycloak_realm: env::var("KEYCLOAK_REALM").unwrap_or_else(|_| "lianel".to_string()),
        keycloak_admin_user: env::var("KEYCLOAK_ADMIN_USER")
            .expect("KEYCLOAK_ADMIN_USER environment variable is required"),
        keycloak_admin_password: env::var("KEYCLOAK_ADMIN_PASSWORD")
            .expect("KEYCLOAK_ADMIN_PASSWORD environment variable is required"),
        backend_client_secret: env::var("BACKEND_CLIENT_SECRET")
            .expect("BACKEND_CLIENT_SECRET environment variable is required"),
    };

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("Invalid PORT value");

    tracing::info!("Starting profile service on port {}", port);
    tracing::info!("Keycloak URL: {}", config.keycloak_url);
    tracing::info!("Keycloak Realm: {}", config.keycloak_realm);

    // Initialize Keycloak validator
    let validator = Arc::new(KeycloakValidator::new(config.clone()));

    // Build CORS from environment (tighten to production origins)
    let allowed_origins_env = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "https://lianel.se,https://www.lianel.se".to_string());
    let origins: Vec<&str> = allowed_origins_env
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let allow_origin = if origins.is_empty() {
        AllowOrigin::list(vec![HeaderValue::from_static("https://lianel.se")])
    } else {
        let mut vals: Vec<HeaderValue> = Vec::new();
        for o in origins {
            if let Ok(h) = HeaderValue::from_str(o) {
                vals.push(h);
            }
        }
        if vals.is_empty() {
            AllowOrigin::list(vec![HeaderValue::from_static("https://lianel.se")])
        } else {
            AllowOrigin::list(vals)
        }
    };

    let cors = CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([http::Method::GET, http::Method::POST, http::Method::PUT, http::Method::DELETE, http::Method::OPTIONS])
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .route("/api/profile", get(get_profile).put(update_profile))
        .route("/api/profile/change-password", post(change_password))
        .route("/api/admin/check", get(check_admin))
        .route("/api/admin/users", get(list_users).post(create_user))
        .route("/api/admin/users/:user_id", get(get_user_by_id).put(update_user).delete(delete_user))
        .route("/api/admin/users/:user_id/change-password", post(admin_change_password))
        .route("/health", get(health))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .into_inner(),
        )
        .with_state((config, validator));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Failed to bind to address");

    tracing::info!("Profile service listening on 0.0.0.0:{}", port);
    tracing::info!("Swagger UI available at http://0.0.0.0:{}/swagger-ui", port);

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
