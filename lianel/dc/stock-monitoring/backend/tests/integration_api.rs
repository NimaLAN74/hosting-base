//! API integration tests: /health and /api/v1/status (public routes).
//! Require DATABASE_URL or POSTGRES_* env (same as integration_config_db).
//! Run with: DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres cargo test

use axum::body::Body;
use axum::http::Request;
use lianel_stock_monitoring_service::app::{create_router, AppState, QuoteService};
use lianel_stock_monitoring_service::auth::KeycloakJwtValidator;
use lianel_stock_monitoring_service::config::AppConfig;
use lianel_stock_monitoring_service::db;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower::ServiceExt;

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
