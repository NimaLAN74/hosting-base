//! API integration tests: /health, /api/v1/status, quotes, openapi, auth, watchlist and alert flows.
//! Require DATABASE_URL or POSTGRES_* env (same as integration_config_db).
//! Run with: DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres cargo test

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::http::header::{CONTENT_TYPE, HeaderName};
use lianel_stock_monitoring_service::app::{create_router, AppState, QuoteService};
use lianel_stock_monitoring_service::auth::KeycloakJwtValidator;
use lianel_stock_monitoring_service::config::AppConfig;
use lianel_stock_monitoring_service::db;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower::ServiceExt;

const X_AUTH_REQUEST_USER: HeaderName = HeaderName::from_static("x-auth-request-user");

#[tokio::test]
async fn health_returns_200_ok() {
    let state = test_state().await;
    let app = create_router(state);

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body.as_ref(), b"ok");
}

#[tokio::test]
async fn status_returns_200_and_includes_service_info() {
    let state = test_state().await;
    let app = create_router(state);

    let req = Request::builder()
        .uri("/api/v1/status")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let s = String::from_utf8_lossy(&body);
    assert!(s.contains("stock-monitoring"), "status body should include service name: {}", s);
    // When DB is connected, body includes "database":"connected"; when not, it still has service/version/scope
    assert!(
        s.contains("\"service\"") || s.contains("stock-monitoring"),
        "status body should be JSON with service info: {}",
        s
    );
}

async fn test_state() -> AppState {
    let config = AppConfig::from_env().expect("config must load in CI");
    let pool = db::create_pool(&config.database_url())
        .await
        .expect("pool must connect in CI");
    let config = Arc::new(config);
    let validator = Arc::new(KeycloakJwtValidator::new(config));
    let quote_service = QuoteService {
        provider: "yahoo".to_string(),
        cache_ttl: Duration::from_secs(30),
        data_provider_api_key: None,
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .expect("http client"),
        cache: Arc::new(RwLock::new(HashMap::new())),
    };
    AppState {
        pool,
        validator,
        quote_service,
    }
}

fn request_with_test_user(method: &str, uri: &str, body: Option<&str>) -> Request<Body> {
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header(X_AUTH_REQUEST_USER, "test-user-7-2");
    if let Some(body) = body {
        b.header(CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    } else {
        b.body(Body::empty()).unwrap()
    }
}

#[tokio::test]
async fn quotes_missing_symbols_returns_400() {
    let state = test_state().await;
    let app = create_router(state);
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/quotes")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn quotes_with_symbols_returns_200_and_shape() {
    let state = test_state().await;
    let app = create_router(state);
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/quotes?symbols=AAPL,MSFT")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("provider").is_some());
    assert!(json.get("as_of_ms").is_some());
    assert!(json.get("quotes").is_some());
    assert!(json.get("warnings").is_some());
}

#[tokio::test]
async fn openapi_doc_returns_200_and_valid_openapi() {
    let state = test_state().await;
    let app = create_router(state);
    let req = Request::builder()
        .method("GET")
        .uri("/api-doc")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("openapi").is_some());
}

#[tokio::test]
async fn protected_route_without_auth_returns_401() {
    let state = test_state().await;
    let app = create_router(state);
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/me")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_with_x_auth_request_user_returns_200() {
    let state = test_state().await;
    let app = create_router(state);
    let req = request_with_test_user("GET", "/api/v1/me", None);
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        json.get("user_id").and_then(|v| v.as_str()),
        Some("test-user-7-2")
    );
}

#[tokio::test]
async fn watchlist_flow_crud() {
    let state = test_state().await;
    let app = create_router(state);

    // Create watchlist
    let req = request_with_test_user("POST", "/api/v1/watchlists", Some(r#"{"name":"Test WL 7.2"}"#));
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let wl_id = created.get("id").and_then(|v| v.as_str()).expect("watchlist id");

    // List watchlists
    let req = request_with_test_user("GET", "/api/v1/watchlists", None);
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = list.as_array().unwrap();
    assert!(items
        .iter()
        .any(|o| o.get("id").and_then(|v| v.as_str()) == Some(wl_id)));

    // Add item
    let req = request_with_test_user(
        "POST",
        &format!("/api/v1/watchlists/{}/items", wl_id),
        Some(r#"{"symbol":"AAPL"}"#),
    );
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // List items
    let req = request_with_test_user("GET", &format!("/api/v1/watchlists/{}/items", wl_id), None);
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let items: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = items.as_array().unwrap();
    let item = arr
        .iter()
        .find(|o| o.get("symbol").and_then(|v| v.as_str()) == Some("AAPL"))
        .unwrap();
    let item_id = item.get("id").and_then(|v| v.as_str()).expect("item id");

    // Delete item
    let req = request_with_test_user(
        "DELETE",
        &format!("/api/v1/watchlists/{}/items/{}", wl_id, item_id),
        None,
    );
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Delete watchlist
    let req = request_with_test_user("DELETE", &format!("/api/v1/watchlists/{}", wl_id), None);
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn alert_flow_crud() {
    let state = test_state().await;
    let app = create_router(state);

    // Create alert
    let req = request_with_test_user(
        "POST",
        "/api/v1/alerts",
        Some(r#"{"symbol":"MSFT","condition_type":"above","condition_value":500.0,"enabled":true}"#),
    );
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let alert_id = created.get("id").and_then(|v| v.as_str()).expect("alert id");

    // List alerts
    let req = request_with_test_user("GET", "/api/v1/alerts", None);
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(list
        .as_array()
        .unwrap()
        .iter()
        .any(|o| o.get("id").and_then(|v| v.as_str()) == Some(alert_id)));

    // Update (e.g. disable)
    let req = request_with_test_user(
        "PUT",
        &format!("/api/v1/alerts/{}", alert_id),
        Some(r#"{"enabled":false}"#),
    );
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Delete
    let req = request_with_test_user("DELETE", &format!("/api/v1/alerts/{}", alert_id), None);
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}
