//! API integration tests: /health, /api/v1/status. Stock service minimal API.
//! Run with: cargo test (no DB required).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use lianel_stock_service::app::{create_router, AppState};
use lianel_stock_service::auth::KeycloakJwtValidator;
use lianel_stock_service::config::AppConfig;
use lianel_stock_service::watchlist;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_200_ok() {
    let state = test_state();
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
    let state = test_state();
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
    assert!(
        s.contains("stock-service"),
        "status body should include service name: {}",
        s
    );
    assert!(
        s.contains("ibkr_oauth_configured"),
        "status body should include ibkr_oauth_configured: {}",
        s
    );
}

#[tokio::test]
async fn me_without_auth_returns_401() {
    let state = test_state();
    let app = create_router(state);

    let req = Request::builder()
        .uri("/api/v1/me")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn test_state() -> AppState {
    let config = AppConfig::from_env().expect("config must load (set KEYCLOAK_URL/KEYCLOAK_REALM or use defaults)");
    let config = Arc::new(config);
    let validator = Arc::new(KeycloakJwtValidator::new(Arc::clone(&config)));
    AppState {
        validator,
        config: Some(config),
        ibkr_client: None,
        watchlist_cache: Arc::new(tokio::sync::RwLock::new(
            watchlist::WatchlistCache::default(),
        )),
        redis: None,
    }
}
