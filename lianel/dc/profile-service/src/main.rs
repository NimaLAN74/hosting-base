use axum::{
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::env;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Debug, Clone)]
struct AppConfig {
    keycloak_url: String,
    keycloak_realm: String,
    keycloak_admin_user: String,
    keycloak_admin_password: String,
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
        health
    ),
    components(schemas(
        UserProfile,
        UpdateProfileRequest,
        ChangePasswordRequest,
        SuccessResponse,
        ErrorResponse,
        HealthResponse
    )),
    tags(
        (name = "profile", description = "User profile management endpoints"),
        (name = "health", description = "Health check endpoint")
    ),
    info(
        title = "Lianel Profile Service API",
        description = "API for managing user profiles via Keycloak",
        version = "1.0.0"
    ),
    servers(
        (url = "https://lianel.se", description = "Production server")
    )
)]
struct ApiDoc;

// Extract user info from headers
fn extract_user_from_headers(headers: &HeaderMap) -> Result<(String, String, Option<String>), StatusCode> {
    // Try to get username/preferred_username first, fallback to user ID
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

    // Get user ID if available (might be UUID)
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
    axum::extract::State(config): axum::extract::State<AppConfig>,
) -> Result<Json<UserProfile>, (StatusCode, Json<ErrorResponse>)> {
    let (username, _email, user_id_from_header) = extract_user_from_headers(&headers)
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
    axum::extract::State(config): axum::extract::State<AppConfig>,
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

    let (username, _email, user_id_from_header) = extract_user_from_headers(&headers)
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
    axum::extract::State(config): axum::extract::State<AppConfig>,
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

    let (username, email, user_id_from_header) = extract_user_from_headers(&headers)
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

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
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
    };

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("Invalid PORT value");

    tracing::info!("Starting profile service on port {}", port);
    tracing::info!("Keycloak URL: {}", config.keycloak_url);
    tracing::info!("Keycloak Realm: {}", config.keycloak_realm);

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .route("/api/profile", get(get_profile).put(update_profile))
        .route("/api/profile/change-password", post(change_password))
        .route("/health", get(health))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .into_inner(),
        )
        .with_state(config);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Failed to bind to address");

    tracing::info!("Profile service listening on 0.0.0.0:{}", port);
    tracing::info!("Swagger UI available at http://0.0.0.0:{}/swagger-ui", port);

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
