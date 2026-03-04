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

/// Extract id from JSON (backend returns numeric id).
fn json_id(v: &serde_json::Value) -> Option<String> {
    v.get("id").and_then(|n| {
        n.as_i64()
            .map(|i| i.to_string())
            .or_else(|| n.as_str().map(String::from))
    })
}

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
        finnhub_api_key: None,
        finnhub_webhook_secret: None,
        alpaca_api_key_id: None,
        alpaca_api_secret_key: None,
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
        finnhub_webhook_secret: None,
        #[cfg(feature = "redis")]
        redis: None,
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
async fn quotes_with_pairs_returns_200_and_shape() {
    let state = test_state().await;
    let app = create_router(state);
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/quotes?pairs=AAPL:yahoo,MSFT:yahoo,SAP.DE:yahoo")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("quotes").is_some());
    let quotes = json.get("quotes").and_then(|v| v.as_array()).unwrap();
    assert!(!quotes.is_empty() || json.get("warnings").and_then(|v| v.as_array()).map(|a| !a.is_empty()).unwrap_or(false), "pairs= returns quotes or warnings");
    for q in quotes {
        assert!(q.get("symbol").and_then(|v| v.as_str()).is_some());
        assert!(q.get("source").is_some() || q.get("price").is_some());
    }
}

#[tokio::test]
async fn quotes_pairs_after_ingest_returns_cached_by_provider() {
    let state = test_state().await;
    let app = create_router(state);
    let ingest_body = r#"{"quotes":[{"symbol":"PAIRTEST","price":42.5,"currency":"USD","provider":"yahoo"}]}"#;
    let req = Request::builder()
        .method("POST")
        .uri("/internal/quotes/ingest")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(ingest_body.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/quotes?pairs=PAIRTEST:yahoo")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let quotes = json.get("quotes").and_then(|v| v.as_array()).unwrap();
    let q = quotes.iter().find(|o| o.get("symbol").and_then(|v| v.as_str()) == Some("PAIRTEST"));
    assert!(q.is_some(), "pairs=PAIRTEST:yahoo must return cached quote after ingest");
    let q = q.unwrap();
    assert_eq!(q.get("price").and_then(|v| v.as_f64()), Some(42.5));
    assert_eq!(q.get("source").and_then(|v| v.as_str()), Some("yahoo"));
}

#[tokio::test]
async fn internal_quotes_ingest_accepts_and_caches() {
    let state = test_state().await;
    let app = create_router(state);
    let body = r#"{"quotes":[{"symbol":"INGESTTEST","price":99.5,"currency":"EUR"}]}"#;
    let req = Request::builder()
        .method("POST")
        .uri("/internal/quotes/ingest")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(json.get("ingested").and_then(|v| v.as_u64()), Some(1));
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/quotes?symbols=INGESTTEST")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let quotes = json.get("quotes").and_then(|v| v.as_array()).unwrap();
    let q = quotes.iter().find(|o| o.get("symbol").and_then(|v| v.as_str()) == Some("INGESTTEST")).unwrap();
    assert_eq!(q.get("price").and_then(|v| v.as_f64()), Some(99.5));
    assert_eq!(q.get("currency").and_then(|v| v.as_str()), Some("EUR"));
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
    let wl_id = json_id(&created).expect("watchlist id");

    // List watchlists
    let req = request_with_test_user("GET", "/api/v1/watchlists", None);
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = list.as_array().unwrap();
    assert!(items.iter().any(|o| json_id(o).as_deref() == Some(wl_id.as_str())));

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
    let item_id = json_id(item).expect("item id");

    // Delete item
    let req = request_with_test_user(
        "DELETE",
        &format!("/api/v1/watchlists/{}/items/{}", wl_id, item_id),
        None,
    );
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Delete watchlist
    let req = request_with_test_user("DELETE", &format!("/api/v1/watchlists/{}", wl_id), None);
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
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
    let alert_id = json_id(&created).expect("alert id");

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
        .any(|o| json_id(o).as_deref() == Some(alert_id.as_str())));

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
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn symbols_providers_returns_200_and_list() {
    let state = test_state().await;
    let app = create_router(state);
    let req = request_with_test_user("GET", "/api/v1/symbols/providers", None);
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().expect("response is array");
    assert!(arr.len() >= 5, "at least 5 providers (yahoo, finnhub, alpaca, stooq, alpha_vantage)");
    let ids: Vec<&str> = arr.iter().filter_map(|o| o.get("id").and_then(|v| v.as_str())).collect();
    assert!(ids.contains(&"yahoo"), "list includes yahoo");
    assert!(ids.contains(&"finnhub"), "list includes finnhub");
}

#[tokio::test]
async fn symbols_list_returns_200_from_db() {
    let state = test_state().await;
    let app = create_router(state);
    let req = request_with_test_user("GET", "/api/v1/symbols?provider=finnhub&exchange=US", None);
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array(), "response must be array of symbols");
}

/// Price-history API: one endpoint returns both daily (last N days) and intraday_today.
/// GET /api/v1/price-history?symbol=SYMBOL&days=7
#[tokio::test]
async fn price_history_returns_daily_and_intraday_arrays() {
    let state = test_state().await;
    let app = create_router(state);

    let req = request_with_test_user("GET", "/api/v1/price-history?symbol=SHL.L&days=7", None);
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "price-history must return 200");
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json.get("symbol").and_then(|v| v.as_str()), Some("SHL.L"));
    let daily = json.get("daily").and_then(|v| v.as_array()).expect("response must have 'daily' array");
    let intraday = json.get("intraday_today").and_then(|v| v.as_array()).expect("response must have 'intraday_today' array");

    for (i, point) in daily.iter().enumerate() {
        assert!(point.get("trade_date").and_then(|v| v.as_str()).is_some(), "daily[{}] must have trade_date string", i);
        assert!(point.get("close_price").and_then(|v| v.as_f64()).is_some(), "daily[{}] must have close_price number", i);
    }
    for (i, point) in intraday.iter().enumerate() {
        assert!(point.get("observed_at").and_then(|v| v.as_str()).is_some(), "intraday_today[{}] must have observed_at string", i);
        assert!(point.get("price").and_then(|v| v.as_f64()).is_some(), "intraday_today[{}] must have price number", i);
    }
}

/// GET /api/v1/price-history/daily — 7-day history only (from DB).
#[tokio::test]
async fn price_history_daily_returns_daily_array() {
    let state = test_state().await;
    let app = create_router(state);
    let req = request_with_test_user("GET", "/api/v1/price-history/daily?symbol=AAPL&days=7", None);
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.get("symbol").and_then(|v| v.as_str()), Some("AAPL"));
    let daily = json.get("daily").and_then(|v| v.as_array()).expect("must have daily array");
    for (i, point) in daily.iter().enumerate() {
        assert!(point.get("trade_date").and_then(|v| v.as_str()).is_some(), "daily[{}] trade_date", i);
        assert!(point.get("close_price").and_then(|v| v.as_f64()).is_some(), "daily[{}] close_price", i);
    }
}

/// GET /api/v1/price-history/intraday — minute-by-minute today (DB + Redis + cache).
#[tokio::test]
async fn price_history_intraday_returns_intraday_array() {
    let state = test_state().await;
    let app = create_router(state);
    let req = request_with_test_user("GET", "/api/v1/price-history/intraday?symbol=AAPL", None);
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.get("symbol").and_then(|v| v.as_str()), Some("AAPL"));
    let intraday = json.get("intraday_today").and_then(|v| v.as_array()).expect("must have intraday_today array");
    for (i, point) in intraday.iter().enumerate() {
        assert!(point.get("observed_at").and_then(|v| v.as_str()).is_some(), "intraday[{}] observed_at", i);
        assert!(point.get("price").and_then(|v| v.as_f64()).is_some(), "intraday[{}] price", i);
    }
}
