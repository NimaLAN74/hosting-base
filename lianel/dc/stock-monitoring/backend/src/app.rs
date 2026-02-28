//! Axum router and handlers. Exposed for integration tests (oneshot) and main binary.

use axum::{
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, Request, StatusCode},
    middleware,
    response::{Html, IntoResponse},
    routing::{delete, get, post, put},
    Json, Router,
};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use chrono::TimeZone;

use crate::auth::{KeycloakJwtValidator, UserId};

#[derive(Clone)]
pub struct QuoteService {
    pub provider: String,
    pub cache_ttl: Duration,
    pub data_provider_api_key: Option<String>,
    pub finnhub_api_key: Option<String>,
    /// Sent as X-Finnhub-Secret on outbound Finnhub API requests when set.
    pub finnhub_webhook_secret: Option<String>,
    pub http: reqwest::Client,
    pub cache: Arc<RwLock<HashMap<String, CachedQuote>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CachedQuote {
    pub symbol: String,
    pub price: f64,
    pub currency: Option<String>,
    pub fetched_at_ms: u64,
}

/// Redis key prefix for intraday points (when feature "redis" enabled): stock:intraday:{symbol}:{yyyy-mm-dd}.
#[cfg(feature = "redis")]
const REDIS_INTRADAY_KEY_PREFIX: &str = "stock:intraday";
#[cfg(feature = "redis")]
const REDIS_INTRADAY_TTL_SECS: usize = 25 * 3600; // 25 hours so key expires after EOD roll

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub validator: Arc<KeycloakJwtValidator>,
    pub quote_service: QuoteService,
    /// Verified against X-Finnhub-Secret on incoming webhook POSTs when set.
    pub finnhub_webhook_secret: Option<String>,
    /// When feature "redis" is enabled and set, intraday points are written/read from Redis.
    #[cfg(feature = "redis")]
    pub redis: Option<redis::aio::MultiplexedConnection>,
}

#[derive(Clone)]
struct UserDisplayName(pub String);

#[derive(Clone)]
struct UserEmail(pub Option<String>);

fn looks_like_user_id(candidate: &str) -> bool {
    let s = candidate.trim();
    if s.is_empty() {
        return true;
    }
    let uuid_like = s.len() == 36
        && s.chars().filter(|c| *c == '-').count() == 4
        && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    let tokenish = s.contains('|') || s.starts_with("auth0|");
    uuid_like || tokenish
}

fn normalize_display_name(
    display_name: Option<String>,
    user_id: &str,
    email: Option<&str>,
) -> Option<String> {
    let cleaned = display_name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .filter(|v| v != user_id)
        .filter(|v| !looks_like_user_id(v));
    if cleaned.is_some() {
        return cleaned;
    }
    email
        .and_then(|v| v.split('@').next())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(String::from)
}

/// Build the application router (used by main and by API integration tests).
pub fn create_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/quotes", get(quotes))
        .route("/api-doc", get(openapi_doc))
        .route("/swagger-ui", get(swagger_ui))
        .route("/swagger-ui/", get(swagger_ui))
        .route("/internal/alerts/evaluate", post(evaluate_alerts_internal))
        .route("/internal/quotes/ingest", post(ingest_quotes_internal))
        .route("/internal/symbols/refresh", post(symbols_refresh_internal))
        .route("/internal/price-history/roll-daily", post(price_history_roll_daily))
        .route("/internal/webhooks/finnhub", post(finnhub_webhook))
        .route("/api/v1/internal/webhooks/finnhub", post(finnhub_webhook));

    let protected = Router::new()
        .route("/api/v1/me", get(me))
        .route("/api/v1/watchlists", get(list_watchlists).post(create_watchlist))
        .route(
            "/api/v1/watchlists/:watchlist_id",
            put(update_watchlist).delete(delete_watchlist),
        )
        .route("/api/v1/watchlists/:watchlist_id/items", get(list_watchlist_items).post(add_watchlist_item))
        .route("/api/v1/watchlists/:watchlist_id/items/:item_id", delete(delete_watchlist_item))
        .route("/api/v1/alerts", get(list_alerts).post(create_alert))
        .route("/api/v1/alerts/:alert_id", put(update_alert).delete(delete_alert))
        .route("/api/v1/notifications", get(list_notifications))
        .route("/api/v1/notifications/:notification_id/read", put(mark_notification_read))
        .route("/api/v1/notifications/read-all", post(mark_all_notifications_read))
        .route("/api/v1/symbols/providers", get(symbols_providers))
        .route("/api/v1/symbols", get(symbols_list))
        .route("/api/v1/price-history", get(price_history))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .merge(public)
        .merge(protected)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    let parse_bearer = |raw: &str| {
        raw.strip_prefix("Bearer ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };

    let headers = req.headers();
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_bearer)
        // oauth2-proxy auth_request token header (forwarded by nginx)
        .or_else(|| {
            headers
                .get("x-auth-request-access-token")
                .and_then(|v| v.to_str().ok())
                .and_then(|raw| {
                    let trimmed = raw.trim();
                    if trimmed.is_empty() {
                        None
                    } else if let Some(parsed) = parse_bearer(trimmed) {
                        Some(parsed)
                    } else {
                        Some(trimmed.to_string())
                    }
                })
        });

    if let Some(token) = token {
        match state.validator.validate_bearer_identity(&token).await {
            Ok(identity) => {
                let display_name = normalize_display_name(
                    identity.display_name.clone(),
                    &identity.user_id,
                    identity.email.as_deref(),
                )
                .unwrap_or_else(|| "User".to_string());
                req.extensions_mut().insert(UserId(identity.user_id));
                req.extensions_mut().insert(UserDisplayName(display_name));
                req.extensions_mut().insert(UserEmail(identity.email));
                return next.run(req).await;
            }
            Err(e) => {
                tracing::debug!("JWT validation failed: {}", e);
            }
        }
    }

    // Last-resort trusted proxy identity when oauth2 already authenticated upstream.
    let forwarded_user = headers
        .get("x-auth-request-user")
        .or_else(|| headers.get("x-forwarded-user"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let forwarded_email = headers
        .get("x-auth-request-email")
        .or_else(|| headers.get("x-forwarded-email"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    match forwarded_user {
        Some(sub) => {
            let display_name = normalize_display_name(None, &sub, forwarded_email.as_deref())
                .unwrap_or_else(|| "User".to_string());
            req.extensions_mut().insert(UserId(sub));
            req.extensions_mut().insert(UserDisplayName(display_name));
            req.extensions_mut().insert(UserEmail(forwarded_email));
            next.run(req).await
        }
        None => {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Missing authorization token"})),
            )
                .into_response()
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn openapi_doc() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Stock Monitoring API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "EU markets MVP API for watchlists, quotes, alerts, notifications, and profile endpoints."
        },
        "servers": [
            { "url": "/", "description": "Direct backend" },
            { "url": "/api/v1/stock-monitoring", "description": "Nginx proxied base path" }
        ],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            }
        },
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/v1/status": {
                "get": {
                    "summary": "Service status",
                    "responses": {
                        "200": { "description": "Service metadata and DB connectivity status" }
                    }
                }
            },
            "/api/v1/quotes": {
                "get": {
                    "summary": "Get latest quotes for symbols",
                    "parameters": [
                        {
                            "name": "symbols",
                            "in": "query",
                            "required": true,
                            "schema": { "type": "string" },
                            "description": "Comma-separated symbols, e.g. ASML.AS,SAP.DE,ERIC-B.ST"
                        }
                    ],
                    "responses": {
                        "200": { "description": "Latest quote payload" },
                        "400": { "description": "Invalid or missing symbols" }
                    }
                }
            },
            "/api/v1/me": {
                "get": {
                    "summary": "Current authenticated user",
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "200": { "description": "User identity profile" },
                        "401": { "description": "Unauthorized" }
                    }
                }
            },
            "/api/v1/watchlists": {
                "get": {
                    "summary": "List watchlists",
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "200": { "description": "Watchlists for user" },
                        "401": { "description": "Unauthorized" }
                    }
                },
                "post": {
                    "summary": "Create watchlist",
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["name"],
                                    "properties": { "name": { "type": "string" } }
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": { "description": "Watchlist created" },
                        "400": { "description": "Validation error" },
                        "409": { "description": "Duplicate watchlist name" },
                        "401": { "description": "Unauthorized" }
                    }
                }
            },
            "/api/v1/watchlists/{watchlist_id}": {
                "put": {
                    "summary": "Rename watchlist",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [{
                        "name": "watchlist_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "integer", "format": "int64" }
                    }],
                    "responses": {
                        "200": { "description": "Watchlist updated" },
                        "404": { "description": "Watchlist not found" }
                    }
                },
                "delete": {
                    "summary": "Delete watchlist",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [{
                        "name": "watchlist_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "integer", "format": "int64" }
                    }],
                    "responses": {
                        "200": { "description": "Watchlist deleted" },
                        "404": { "description": "Watchlist not found" }
                    }
                }
            },
            "/api/v1/watchlists/{watchlist_id}/items": {
                "get": {
                    "summary": "List watchlist items",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [{
                        "name": "watchlist_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "integer", "format": "int64" }
                    }],
                    "responses": {
                        "200": { "description": "Watchlist symbols" }
                    }
                },
                "post": {
                    "summary": "Add item to watchlist",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [{
                        "name": "watchlist_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "integer", "format": "int64" }
                    }],
                    "responses": {
                        "201": { "description": "Item created" },
                        "400": { "description": "Invalid symbol/MIC" },
                        "409": { "description": "Duplicate item in watchlist" }
                    }
                }
            },
            "/api/v1/watchlists/{watchlist_id}/items/{item_id}": {
                "delete": {
                    "summary": "Delete watchlist item",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        {
                            "name": "watchlist_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "integer", "format": "int64" }
                        },
                        {
                            "name": "item_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "integer", "format": "int64" }
                        }
                    ],
                    "responses": {
                        "200": { "description": "Item deleted" },
                        "404": { "description": "Item not found" }
                    }
                }
            },
            "/api/v1/alerts": {
                "get": {
                    "summary": "List alerts",
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "200": { "description": "Alerts for user" }
                    }
                },
                "post": {
                    "summary": "Create alert",
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "201": { "description": "Alert created" },
                        "400": { "description": "Validation error" }
                    }
                }
            },
            "/api/v1/alerts/{alert_id}": {
                "put": {
                    "summary": "Update alert",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [{
                        "name": "alert_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "integer", "format": "int64" }
                    }],
                    "responses": {
                        "200": { "description": "Alert updated" },
                        "404": { "description": "Alert not found" }
                    }
                },
                "delete": {
                    "summary": "Delete alert",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [{
                        "name": "alert_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "integer", "format": "int64" }
                    }],
                    "responses": {
                        "200": { "description": "Alert deleted" },
                        "404": { "description": "Alert not found" }
                    }
                }
            },
            "/api/v1/notifications": {
                "get": {
                    "summary": "List in-app notifications",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [
                        {
                            "name": "include_read",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "boolean" }
                        },
                        {
                            "name": "limit",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "integer", "format": "int64" }
                        }
                    ],
                    "responses": {
                        "200": { "description": "Notifications list" }
                    }
                }
            },
            "/api/v1/notifications/{notification_id}/read": {
                "put": {
                    "summary": "Mark one notification as read",
                    "security": [{ "bearerAuth": [] }],
                    "parameters": [{
                        "name": "notification_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "integer", "format": "int64" }
                    }],
                    "responses": {
                        "200": { "description": "Mark result" }
                    }
                }
            },
            "/api/v1/notifications/read-all": {
                "post": {
                    "summary": "Mark all notifications as read",
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "200": { "description": "Mark result" }
                    }
                }
            },
            "/internal/alerts/evaluate": {
                "post": {
                    "summary": "Evaluate alerts for all users (internal)",
                    "responses": {
                        "200": { "description": "Evaluation summary" }
                    }
                }
            },
            "/internal/quotes/ingest": {
                "post": {
                    "summary": "Push quotes into cache (internal)",
                    "responses": {
                        "200": { "description": "Ingest result with count" }
                    }
                }
            }
        }
    }))
}

async fn swagger_ui() -> Html<&'static str> {
    Html(r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Stock Monitoring API Docs</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
      window.ui = SwaggerUIBundle({
        url: '/api-doc',
        dom_id: '#swagger-ui',
        deepLinking: true,
        presets: [SwaggerUIBundle.presets.apis],
      });
    </script>
  </body>
</html>"#)
}

async fn status(State(state): State<AppState>) -> String {
    let scope = serde_json::json!({
        "service": "stock-monitoring",
        "version": env!("CARGO_PKG_VERSION"),
        "scope": "EU markets MVP"
    });
    if sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok()
    {
        serde_json::json!({
            "service": scope["service"],
            "version": scope["version"],
            "scope": scope["scope"],
            "database": "connected"
        })
        .to_string()
    } else {
        scope.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct QuotesQuery {
    symbols: Option<String>,
}

#[derive(Debug, Serialize)]
struct QuoteDto {
    symbol: String,
    price: f64,
    currency: Option<String>,
    fetched_at_ms: u64,
    stale: bool,
}

#[derive(Debug, Serialize)]
struct QuotesResponse {
    provider: String,
    as_of_ms: u64,
    quotes: Vec<QuoteDto>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SymbolProviderDto {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct SymbolDto {
    symbol: String,
    display_symbol: String,
    description: Option<String>,
    #[serde(rename = "type")]
    type_: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SymbolsQuery {
    provider: Option<String>,
    q: Option<String>,
    exchange: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SymbolsRefreshQuery {
    provider: Option<String>,
}

#[derive(Debug, Serialize)]
struct SymbolsRefreshResult {
    ok: bool,
    provider: String,
    synced: usize,
}

#[derive(Debug, Deserialize)]
struct PriceHistoryQuery {
    symbol: Option<String>,
    days: Option<u32>,
}

#[derive(Debug, Serialize)]
struct PriceHistoryDailyPoint {
    trade_date: String,
    open_price: f64,
    high_price: f64,
    low_price: f64,
    close_price: f64,
}

#[derive(Debug, Serialize)]
struct PriceHistoryIntradayPoint {
    observed_at: String,
    price: f64,
}

#[derive(Debug, Serialize)]
struct PriceHistoryResponse {
    symbol: String,
    daily: Vec<PriceHistoryDailyPoint>,
    intraday_today: Vec<PriceHistoryIntradayPoint>,
}

#[derive(Debug, Serialize)]
struct AlertEvaluationResult {
    ok: bool,
    users_evaluated: usize,
    evaluated_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct IngestQuoteItem {
    symbol: String,
    price: f64,
    #[serde(default)]
    currency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IngestQuotesRequest {
    quotes: Vec<IngestQuoteItem>,
}

#[derive(Debug, Serialize)]
struct IngestQuotesResult {
    ok: bool,
    ingested: usize,
}

#[derive(Debug, Deserialize)]
struct YahooQuoteEnvelope {
    #[serde(rename = "quoteResponse")]
    quote_response: YahooQuoteResponse,
}

#[derive(Debug, Deserialize)]
struct YahooQuoteResponse {
    result: Vec<YahooQuoteItem>,
}

#[derive(Debug, Deserialize)]
struct YahooQuoteItem {
    symbol: String,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    currency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlphaVantageGlobalQuoteEnvelope {
    #[serde(rename = "Global Quote")]
    global_quote: Option<AlphaVantageGlobalQuote>,
    #[serde(rename = "Note")]
    note: Option<String>,
    #[serde(rename = "Error Message")]
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlphaVantageGlobalQuote {
    #[serde(rename = "05. price")]
    price: Option<String>,
}

/// Finnhub.io quote API: https://finnhub.io/docs/api/quote
#[derive(Debug, Deserialize)]
struct FinnhubQuoteResponse {
    /// Current price
    #[serde(default)]
    c: f64,
    /// Error message when API key invalid or rate limited
    #[serde(default)]
    error: Option<String>,
}

/// Finnhub search: GET /search?q=... and stock/symbol?exchange=...
#[derive(Debug, Deserialize)]
struct FinnhubSearchResultItem {
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "displaySymbol", default)]
    display_symbol: Option<String>,
    #[serde(default)]
    symbol: String,
    #[serde(rename = "type", default)]
    type_: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FinnhubSearchEnvelope {
    #[serde(default)]
    count: Option<u32>,
    #[serde(default)]
    result: Option<Vec<FinnhubSearchResultItem>>,
}

async fn quotes(
    State(state): State<AppState>,
    Query(query): Query<QuotesQuery>,
) -> Result<Json<QuotesResponse>, (StatusCode, Json<serde_json::Value>)> {
    let symbols = parse_symbols(query.symbols.as_deref())?;
    let now_ms = current_time_ms();

    let mut cached = Vec::new();
    let mut missing = Vec::new();
    {
        let cache = state.quote_service.cache.read().await;
        for symbol in &symbols {
            if let Some(entry) = cache.get(symbol) {
                let age_ms = now_ms.saturating_sub(entry.fetched_at_ms);
                if age_ms <= state.quote_service.cache_ttl.as_millis() as u64 {
                    cached.push(QuoteDto {
                        symbol: entry.symbol.clone(),
                        price: entry.price,
                        currency: entry.currency.clone(),
                        fetched_at_ms: entry.fetched_at_ms,
                        stale: false,
                    });
                    continue;
                }
            }
            missing.push(symbol.clone());
        }
    }

    let mut fetched_quotes = Vec::new();
    let mut warnings = Vec::new();
    if !missing.is_empty() {
        match fetch_provider_quotes(&state.quote_service, &missing).await {
            Ok(items) => {
                let mut write_cache = state.quote_service.cache.write().await;
                for item in &items {
                    let resolved_currency = item
                        .currency
                        .clone()
                        .or_else(|| infer_currency_from_symbol(&item.symbol));
                    let quote = CachedQuote {
                        symbol: item.symbol.clone(),
                        price: item.price,
                        currency: resolved_currency,
                        fetched_at_ms: now_ms,
                    };
                    write_cache.insert(item.symbol.clone(), quote.clone());
                    fetched_quotes.push(QuoteDto {
                        symbol: quote.symbol,
                        price: quote.price,
                        currency: quote.currency,
                        fetched_at_ms: quote.fetched_at_ms,
                        stale: false,
                    });
                }
                let pool = state.pool.clone();
                let intraday_items: Vec<(String, f64)> = items.iter().map(|i| (i.symbol.clone(), i.price)).collect();
                let observed_at_secs = current_time_secs();
                tokio::spawn({
                    let items = intraday_items.clone();
                    async move {
                        if let Err(e) = persist_intraday_quotes(&pool, &items, observed_at_secs).await {
                            tracing::warn!("persist intraday: {}", e);
                        }
                    }
                });
                #[cfg(feature = "redis")]
                if let Some(redis) = state.redis.clone() {
                    let items = intraday_items;
                    let observed = observed_at_secs;
                    tokio::spawn(async move {
                        if let Err(e) = redis_push_intraday(redis, &items, observed).await {
                            tracing::warn!("redis push intraday: {}", e);
                        }
                    });
                }
            }
            Err(err) => {
                warnings.push(err.to_string());
            }
        }
    }

    if !warnings.is_empty() && fetched_quotes.is_empty() {
        let cache = state.quote_service.cache.read().await;
        for symbol in &missing {
            if let Some(entry) = cache.get(symbol) {
                cached.push(QuoteDto {
                    symbol: entry.symbol.clone(),
                    price: entry.price,
                    currency: entry.currency.clone(),
                    fetched_at_ms: entry.fetched_at_ms,
                    stale: true,
                });
            }
        }
    }

    let mut quotes = Vec::new();
    quotes.extend(cached);
    quotes.extend(fetched_quotes);
    quotes.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    // Persist all returned quotes as intraday points so the price-history chart gets history (even when from cache).
    let intraday_from_response: Vec<(String, f64)> = quotes.iter().map(|q| (q.symbol.clone(), q.price)).collect();
    if !intraday_from_response.is_empty() {
        let pool = state.pool.clone();
        let observed_at_secs = current_time_secs();
        let for_db = intraday_from_response.clone();
        tokio::spawn(async move {
            if let Err(e) = persist_intraday_quotes(&pool, &for_db, observed_at_secs).await {
                tracing::warn!("quotes response persist intraday: {}", e);
            }
        });
        #[cfg(feature = "redis")]
        if let Some(redis) = state.redis.clone() {
            let items = intraday_from_response;
            let observed = observed_at_secs;
            tokio::spawn(async move {
                if let Err(e) = redis_push_intraday(redis, &items, observed).await {
                    tracing::warn!("quotes response redis push intraday: {}", e);
                }
            });
        }
    }

    let unresolved: Vec<String> = symbols
        .iter()
        .filter(|symbol| !quotes.iter().any(|q| &q.symbol == *symbol))
        .cloned()
        .collect();
    if !unresolved.is_empty() {
        warnings.push(format!("No provider quote for: {}", unresolved.join(",")));
    }

    Ok(Json(QuotesResponse {
        provider: state.quote_service.provider.clone(),
        as_of_ms: now_ms,
        quotes,
        warnings,
    }))
}

async fn symbols_providers(State(_state): State<AppState>) -> Json<Vec<SymbolProviderDto>> {
    let list = vec![SymbolProviderDto {
        id: "finnhub".to_string(),
        name: "Finnhub".to_string(),
    }];
    Json(list)
}

async fn symbols_list(
    State(state): State<AppState>,
    Query(query): Query<SymbolsQuery>,
) -> Result<Json<Vec<SymbolDto>>, (StatusCode, Json<serde_json::Value>)> {
    let provider = query
        .provider
        .as_deref()
        .unwrap_or("finnhub")
        .trim()
        .to_lowercase();
    if provider != "finnhub" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Unsupported provider",
                "detail": "Only provider=finnhub is supported"
            })),
        ));
    }

    let exchange_filter = query.exchange.as_deref().unwrap_or("").trim();
    let search_pattern = query.q.as_deref().unwrap_or("").trim();

    let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>)>(
        r#"
        SELECT symbol, display_symbol, description, "type"
        FROM stock_monitoring.symbols
        WHERE provider = $1
          AND ($2 = '' OR exchange = $2)
          AND ($3 = '' OR symbol ILIKE '%' || $3 || '%' OR display_symbol ILIKE '%' || $3 || '%' OR description ILIKE '%' || $3 || '%')
        ORDER BY symbol
        LIMIT 5000
        "#,
    )
    .bind(provider.as_str())
    .bind(exchange_filter)
    .bind(search_pattern)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::warn!("symbols list from DB: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to load symbol list",
                "detail": "Database error"
            })),
        )
    })?;

    let symbols = rows
        .into_iter()
        .map(|(symbol, display_symbol, description, type_)| SymbolDto {
            symbol,
            display_symbol,
            description,
            type_,
        })
        .collect::<Vec<_>>();

    Ok(Json(symbols))
}

/// Exchanges to sync from Finnhub (US, LON, AS=Amsterdam, DE=Xetra).
const FINNHUB_SYNC_EXCHANGES: &[&str] = &["US", "LON", "AS", "DE"];

/// Batch size for symbol upserts to avoid long-running single queries and timeouts.
const SYMBOL_UPSERT_BATCH_SIZE: usize = 250;

async fn symbols_refresh_internal(
    State(state): State<AppState>,
    Query(query): Query<SymbolsRefreshQuery>,
) -> Result<Json<SymbolsRefreshResult>, (StatusCode, Json<serde_json::Value>)> {
    let provider = query.provider.as_deref().unwrap_or("finnhub").trim().to_lowercase();
    if provider != "finnhub" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Unsupported provider",
                "detail": "Only provider=finnhub is supported for refresh"
            })),
        ));
    }
    let api_key = match state.quote_service.finnhub_api_key.as_deref() {
        Some(k) if !k.is_empty() => k,
        _ => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Symbol refresh unavailable",
                    "detail": "Finnhub API key not configured"
                })),
            ));
        }
    };

    #[derive(Clone)]
    struct SymbolRow {
        symbol: String,
        display_symbol: String,
        description: Option<String>,
        type_: Option<String>,
        exchange: String,
    }

    let mut all_rows: Vec<SymbolRow> = Vec::new();
    for exchange in FINNHUB_SYNC_EXCHANGES {
        let batch = fetch_finnhub_symbols_by_exchange(
            &state.quote_service.http,
            api_key,
            exchange,
        )
        .await;
        for dto in batch {
            let symbol = dto.symbol.trim().to_uppercase();
            if symbol.is_empty() {
                continue;
            }
            let display_symbol = dto.display_symbol.trim();
            let display_symbol = if display_symbol.is_empty() { symbol.clone() } else { display_symbol.to_string() };
            all_rows.push(SymbolRow {
                symbol,
                display_symbol,
                description: dto.description,
                type_: dto.type_,
                exchange: exchange.to_string(),
            });
        }
    }

    let mut synced = 0usize;
    for chunk in all_rows.chunks(SYMBOL_UPSERT_BATCH_SIZE) {
        let n = chunk.len();
        let mut query_str = r#"
                INSERT INTO stock_monitoring.symbols (provider, symbol, display_symbol, description, "type", exchange, updated_at)
                VALUES "#
            .to_string();
        let mut first = true;
        for i in 0..n {
            if !first {
                query_str.push_str(", ");
            }
            first = false;
            let base = i * 6;
            query_str.push_str(&format!(
                "(${}, ${}, ${}, ${}, ${}, ${}, CURRENT_TIMESTAMP)",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
                base + 6
            ));
        }
        query_str.push_str(
            r#"
                ON CONFLICT (provider, symbol) DO UPDATE SET
                    display_symbol = EXCLUDED.display_symbol,
                    description = EXCLUDED.description,
                    "type" = EXCLUDED."type",
                    exchange = EXCLUDED.exchange,
                    updated_at = CURRENT_TIMESTAMP"#,
        );

        let mut q = sqlx::query(&query_str);
        for row in chunk {
            q = q
                .bind(provider.as_str())
                .bind(&row.symbol)
                .bind(&row.display_symbol)
                .bind(row.description.as_deref())
                .bind(row.type_.as_deref())
                .bind(&row.exchange);
        }
        if let Ok(result) = q.execute(&state.pool).await {
            synced += result.rows_affected() as usize;
        }
    }

    Ok(Json(SymbolsRefreshResult {
        ok: true,
        provider,
        synced,
    }))
}

#[derive(Debug, Serialize)]
struct PriceHistoryRollResult {
    ok: bool,
    daily_rows: u64,
    intraday_deleted: u64,
}

async fn price_history_roll_daily(
    State(state): State<AppState>,
) -> Result<Json<PriceHistoryRollResult>, (StatusCode, Json<serde_json::Value>)> {
    let pool = &state.pool;
    let daily_rows = sqlx::query(
        r#"
        INSERT INTO stock_monitoring.price_history_daily (symbol, trade_date, open_price, high_price, low_price, close_price)
        SELECT
            symbol,
            (observed_at AT TIME ZONE 'UTC')::date AS trade_date,
            (array_agg(price ORDER BY observed_at))[1] AS open_price,
            max(price) AS high_price,
            min(price) AS low_price,
            (array_agg(price ORDER BY observed_at DESC))[1] AS close_price
        FROM stock_monitoring.price_history_intraday
        WHERE (observed_at AT TIME ZONE 'UTC')::date < (now() AT TIME ZONE 'UTC')::date
        GROUP BY symbol, (observed_at AT TIME ZONE 'UTC')::date
        ON CONFLICT (symbol, trade_date) DO UPDATE SET
            open_price = EXCLUDED.open_price,
            high_price = EXCLUDED.high_price,
            low_price = EXCLUDED.low_price,
            close_price = EXCLUDED.close_price,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::warn!("price_history roll: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Roll failed"})),
        )
    })?;
    let daily_rows = daily_rows.rows_affected();

    let intraday_deleted = sqlx::query(
        r#"
        DELETE FROM stock_monitoring.price_history_intraday
        WHERE (observed_at AT TIME ZONE 'UTC')::date < (now() AT TIME ZONE 'UTC')::date
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::warn!("price_history intraday delete: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Delete failed"})),
        )
    })?;
    let intraday_deleted = intraday_deleted.rows_affected();

    Ok(Json(PriceHistoryRollResult {
        ok: true,
        daily_rows,
        intraday_deleted,
    }))
}

async fn price_history(
    State(state): State<AppState>,
    Query(query): Query<PriceHistoryQuery>,
) -> Result<Json<PriceHistoryResponse>, (StatusCode, Json<serde_json::Value>)> {
    let symbol = query
        .symbol
        .as_deref()
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Missing symbol", "detail": "Use ?symbol=AAPL"})),
        ))?;
    let days = query.days.unwrap_or(90).min(365);

    let daily_rows = match sqlx::query_as::<_, (String, f64, f64, f64, f64)>(
        r#"
        SELECT trade_date::text, open_price, high_price, low_price, close_price
        FROM stock_monitoring.price_history_daily
        WHERE symbol = $1
        ORDER BY trade_date DESC
        LIMIT $2
        "#,
    )
    .bind(symbol.as_str())
    .bind(days as i64)
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("price_history daily (returning empty): {}", e);
            vec![]
        }
    };

    let intraday_rows = match sqlx::query_as::<_, (String, f64)>(
        r#"
        SELECT observed_at::text, price
        FROM stock_monitoring.price_history_intraday
        WHERE symbol = $1
          AND (observed_at AT TIME ZONE 'UTC')::date = (now() AT TIME ZONE 'UTC')::date
        ORDER BY observed_at
        "#,
    )
    .bind(symbol.as_str())
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("price_history intraday (returning empty): {}", e);
            vec![]
        }
    };

    let daily: Vec<PriceHistoryDailyPoint> = daily_rows
        .into_iter()
        .map(|(d, o, h, l, c)| PriceHistoryDailyPoint {
            trade_date: d,
            open_price: o,
            high_price: h,
            low_price: l,
            close_price: c,
        })
        .collect();

    let mut intraday_today: Vec<PriceHistoryIntradayPoint> = intraday_rows
        .into_iter()
        .map(|(t, p)| PriceHistoryIntradayPoint {
            observed_at: t,
            price: p,
        })
        .collect();

    let intraday_from_db = intraday_today.len();
    #[cfg(feature = "redis")]
    if let Some(redis) = state.redis.clone() {
        match redis_fetch_intraday_today(redis, &symbol).await {
            Ok(redis_points) => {
                let n_redis = redis_points.len();
                for (observed_at, price) in redis_points {
                    intraday_today.push(PriceHistoryIntradayPoint { observed_at, price });
                }
                tracing::info!("price_history: symbol={} intraday_from_db={} intraday_from_redis={}", symbol, intraday_from_db, n_redis);
            }
            Err(e) => tracing::warn!("redis fetch intraday (skipping): {}", e),
        }
    }
    #[cfg(not(feature = "redis"))]
    let _ = intraday_from_db;

    // Merge latest quote from in-memory cache so the chart shows current price without waiting for DB persist
    let cache = state.quote_service.cache.read().await;
    if let Some(entry) = cache.get(&symbol) {
        if let Some(dt) = chrono::Utc.timestamp_millis_opt(entry.fetched_at_ms as i64).single() {
            let observed_at = format!("{}", dt.format("%Y-%m-%dT%H:%M:%S%.3fZ"));
            intraday_today.push(PriceHistoryIntradayPoint {
                observed_at,
                price: entry.price,
            });
        }
    }
    drop(cache);

    intraday_today.sort_by(|a, b| a.observed_at.cmp(&b.observed_at));
    tracing::info!("price_history: symbol={} daily={} intraday_today={} (chart will show all)", symbol, daily.len(), intraday_today.len());

    Ok(Json(PriceHistoryResponse {
        symbol: symbol.clone(),
        daily,
        intraday_today,
    }))
}

fn parse_symbols(raw: Option<&str>) -> Result<Vec<String>, (StatusCode, Json<serde_json::Value>)> {
    let Some(raw_symbols) = raw else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Missing symbols query parameter",
                "detail": "Use /api/v1/quotes?symbols=ASML.AS,SAP.DE"
            })),
        ));
    };

    let mut symbols = Vec::new();
    for item in raw_symbols.split(',') {
        let symbol = item.trim().to_uppercase();
        if symbol.is_empty() {
            continue;
        }
        if symbol.len() > 20 || !symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_') {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid symbol",
                    "detail": format!("Invalid symbol format: {}", symbol)
                })),
            ));
        }
        symbols.push(symbol);
    }

    if symbols.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "No symbols provided",
                "detail": "Provide at least one symbol"
            })),
        ));
    }

    symbols.sort();
    symbols.dedup();
    Ok(symbols)
}

async fn fetch_provider_quotes(
    quote_service: &QuoteService,
    symbols: &[String],
) -> anyhow::Result<Vec<CachedQuote>> {
    let mut all = Vec::new();
    let http = &quote_service.http;

    // Finnhub (finnhub.io): when API key is set, try first for all symbols.
    if let Some(api_key) = quote_service.finnhub_api_key.as_deref() {
        let webhook_secret = quote_service.finnhub_webhook_secret.as_deref();
        let mut finnhub_quotes = fetch_finnhub_quotes(http, symbols, api_key, webhook_secret).await;
        all.append(&mut finnhub_quotes);
    }

    // Yahoo bulk endpoint (when Finnhub did not resolve all).
    let unresolved_after_finnhub: Vec<String> = symbols
        .iter()
        .filter(|symbol| !all.iter().any(|q| q.symbol.eq_ignore_ascii_case(symbol)))
        .cloned()
        .collect();
    if !unresolved_after_finnhub.is_empty() {
        match fetch_yahoo_quotes(http, &unresolved_after_finnhub).await {
            Ok(mut quotes) => all.append(&mut quotes),
            Err(err) => tracing::warn!("Yahoo quote fetch failed: {}", err),
        }
    }

    // Fallback: per-symbol Stooq CSV endpoint (delayed data).
    let unresolved: Vec<String> = symbols
        .iter()
        .filter(|symbol| !all.iter().any(|q| q.symbol.eq_ignore_ascii_case(symbol)))
        .cloned()
        .collect();

    for symbol in unresolved {
        if let Some(quote) = fetch_stooq_quote(http, &symbol).await {
            all.push(quote);
        }
    }

    // Secondary provider fallback for unresolved symbols (notably Stockholm .ST).
    if let Some(api_key) = quote_service.data_provider_api_key.as_deref() {
        let unresolved_after_stooq: Vec<String> = symbols
            .iter()
            .filter(|symbol| !all.iter().any(|q| q.symbol.eq_ignore_ascii_case(symbol)))
            .cloned()
            .collect();
        if !unresolved_after_stooq.is_empty() {
            let mut alpha_quotes = fetch_alpha_vantage_quotes(http, &unresolved_after_stooq, api_key).await;
            all.append(&mut alpha_quotes);
        }
    }

    if all.is_empty() {
        anyhow::bail!("Provider fetch returned no data (Finnhub/Yahoo/Stooq)");
    }
    Ok(all)
}

/// Finnhub.io quote API: GET /api/v1/quote?symbol=SYMBOL&token=TOKEN
/// When webhook_secret is set, sends X-Finnhub-Secret header per Finnhub requirement.
async fn fetch_finnhub_quotes(
    http: &reqwest::Client,
    symbols: &[String],
    api_key: &str,
    webhook_secret: Option<&str>,
) -> Vec<CachedQuote> {
    const BASE: &str = "https://finnhub.io/api/v1/quote";
    let mut out = Vec::new();
    for original_symbol in symbols {
        let symbol_for_request = original_symbol.trim();
        if symbol_for_request.is_empty() {
            continue;
        }
        let mut req = http.get(BASE).query(&[("symbol", symbol_for_request), ("token", api_key)]);
        if let Some(secret) = webhook_secret {
            req = req.header("X-Finnhub-Secret", secret);
        }
        let response = req.send().await;
        let Ok(resp) = response else {
            continue;
        };
        let Ok(payload) = resp.json::<FinnhubQuoteResponse>().await else {
            continue;
        };
        if let Some(ref err) = payload.error {
            tracing::warn!("Finnhub quote for {}: {}", symbol_for_request, err);
            continue;
        }
        if !payload.c.is_finite() || payload.c <= 0.0 {
            continue;
        }
        out.push(CachedQuote {
            symbol: original_symbol.to_uppercase(),
            price: payload.c,
            currency: infer_currency_from_symbol(original_symbol),
            fetched_at_ms: current_time_ms(),
        });
    }
    out
}

/// Finnhub search: GET /api/v1/search?q=QUERY&token=TOKEN
async fn fetch_finnhub_search(
    http: &reqwest::Client,
    api_key: &str,
    q: &str,
) -> Vec<SymbolDto> {
    let url = "https://finnhub.io/api/v1/search";
    let response = http
        .get(url)
        .query(&[("q", q), ("token", api_key)])
        .send()
        .await;
    let Ok(resp) = response else {
        return Vec::new();
    };
    let Ok(envelope) = resp.json::<FinnhubSearchEnvelope>().await else {
        return Vec::new();
    };
    let Some(items) = envelope.result else {
        return Vec::new();
    };
    items
        .into_iter()
        .map(|r| SymbolDto {
            symbol: r.symbol.clone(),
            display_symbol: r.display_symbol.unwrap_or(r.symbol),
            description: r.description,
            type_: r.type_,
        })
        .collect()
}

/// Finnhub stock symbols by exchange: GET /api/v1/stock/symbol?exchange=US&token=TOKEN
async fn fetch_finnhub_symbols_by_exchange(
    http: &reqwest::Client,
    api_key: &str,
    exchange: &str,
) -> Vec<SymbolDto> {
    let url = "https://finnhub.io/api/v1/stock/symbol";
    let response = http
        .get(url)
        .query(&[("exchange", exchange), ("token", api_key)])
        .send()
        .await;
    let Ok(resp) = response else {
        return Vec::new();
    };
    let Ok(items) = resp.json::<Vec<FinnhubSearchResultItem>>().await else {
        return Vec::new();
    };
    items
        .into_iter()
        .map(|r| SymbolDto {
            symbol: r.symbol.clone(),
            display_symbol: r.display_symbol.unwrap_or(r.symbol),
            description: r.description,
            type_: r.type_,
        })
        .collect()
}

async fn fetch_alpha_vantage_quotes(
    http: &reqwest::Client,
    symbols: &[String],
    api_key: &str,
) -> Vec<CachedQuote> {
    let mut out = Vec::new();
    for original_symbol in symbols {
        let provider_symbol = map_to_alpha_vantage_symbol(original_symbol);
        let response = http
            .get("https://www.alphavantage.co/query")
            .query(&[
                ("function", "GLOBAL_QUOTE"),
                ("symbol", provider_symbol.as_str()),
                ("apikey", api_key),
            ])
            .send()
            .await;
        let Ok(resp) = response else {
            continue;
        };
        let parsed = resp.json::<AlphaVantageGlobalQuoteEnvelope>().await;
        let Ok(payload) = parsed else {
            continue;
        };
        if payload.note.is_some() || payload.error_message.is_some() {
            continue;
        }
        let price = payload
            .global_quote
            .and_then(|q| q.price)
            .and_then(|raw| raw.parse::<f64>().ok());
        let Some(price) = price else {
            continue;
        };
        out.push(CachedQuote {
            symbol: original_symbol.to_uppercase(),
            price,
            currency: infer_currency_from_symbol(original_symbol),
            fetched_at_ms: current_time_ms(),
        });
    }
    out
}

pub(crate) fn map_to_alpha_vantage_symbol(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    if upper.ends_with(".ST") {
        return upper.replace(".ST", ".STO");
    }
    upper
}

async fn fetch_yahoo_quotes(
    http: &reqwest::Client,
    symbols: &[String],
) -> anyhow::Result<Vec<CachedQuote>> {
    let url = "https://query1.finance.yahoo.com/v7/finance/quote";
    let joined = symbols.join(",");
    let resp = http
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; LianelStockMonitoring/1.0)")
        .header(ACCEPT, "application/json,text/plain,*/*")
        .query(&[("symbols", joined)])
        .send()
        .await?
        .error_for_status()?
        .json::<YahooQuoteEnvelope>()
        .await?;

    let mut out = Vec::new();
    for item in resp.quote_response.result {
        if let Some(price) = item.regular_market_price {
            out.push(CachedQuote {
                symbol: item.symbol.to_uppercase(),
                price,
                currency: item
                    .currency
                    .or_else(|| infer_currency_from_symbol(&item.symbol)),
                fetched_at_ms: current_time_ms(),
            });
        }
    }
    Ok(out)
}

async fn fetch_stooq_quote(http: &reqwest::Client, symbol: &str) -> Option<CachedQuote> {
    let stooq_symbol = to_stooq_symbol(symbol);
    let url = format!("https://stooq.com/q/l/?s={}&i=d", stooq_symbol.to_lowercase());
    let text = http
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; LianelStockMonitoring/1.0)")
        .header(ACCEPT, "text/csv,text/plain,*/*")
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .await
        .ok()?;
    parse_stooq_line(symbol, &text)
}

fn parse_stooq_line(original_symbol: &str, line: &str) -> Option<CachedQuote> {
    let first = line.lines().next()?.trim().trim_end_matches(',');
    let parts: Vec<&str> = first.split(',').collect();
    if parts.len() < 7 {
        return None;
    }
    let close = parts[6].trim();
    if close.eq_ignore_ascii_case("N/D") {
        return None;
    }
    let price = close.parse::<f64>().ok()?;
    Some(CachedQuote {
        symbol: original_symbol.to_uppercase(),
        price,
        currency: infer_currency_from_symbol(original_symbol),
        fetched_at_ms: current_time_ms(),
    })
}

fn to_stooq_symbol(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    if upper.ends_with(".L") {
        return upper.replace(".L", ".UK");
    }
    upper
}

pub(crate) fn infer_currency_from_symbol(symbol: &str) -> Option<String> {
    let upper = symbol.to_uppercase();
    if upper.ends_with(".L") || upper.ends_with(".UK") {
        return Some("GBP".to_string());
    }
    if upper.ends_with(".ST") {
        return Some("SEK".to_string());
    }
    if upper.ends_with(".CO") {
        return Some("DKK".to_string());
    }
    if upper.ends_with(".OL") {
        return Some("NOK".to_string());
    }
    if upper.ends_with(".HE")
        || upper.ends_with(".AS")
        || upper.ends_with(".DE")
        || upper.ends_with(".F")
        || upper.ends_with(".PA")
        || upper.ends_with(".MI")
        || upper.ends_with(".MC")
        || upper.ends_with(".BR")
    {
        return Some("EUR".to_string());
    }
    if upper.ends_with(".SW") {
        return Some("CHF".to_string());
    }
    // US-style symbols (no exchange suffix): e.g. AAPL, MSFT from Finnhub/Nasdaq/NYSE -> USD
    if !upper.contains('.') && upper.len() <= 5 && upper.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some("USD".to_string());
    }
    if upper.ends_with(".US") || upper.ends_with(".U") {
        return Some("USD".to_string());
    }
    None
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

fn current_time_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

#[cfg(feature = "redis")]
fn redis_intraday_key(symbol: &str, date_yyyy_mm_dd: &str) -> String {
    format!("{}:{}:{}", REDIS_INTRADAY_KEY_PREFIX, symbol, date_yyyy_mm_dd)
}

#[cfg(feature = "redis")]
async fn redis_push_intraday(
    mut conn: redis::aio::MultiplexedConnection,
    items: &[(String, f64)],
    observed_at_secs: i64,
) -> anyhow::Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let date_str = chrono::Utc
        .timestamp_opt(observed_at_secs, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string());
    let score = observed_at_secs as f64;
    let n = items.len();
    for (symbol, price) in items {
        let key = redis_intraday_key(symbol, &date_str);
        let _: () = redis::cmd("ZADD")
            .arg(&key)
            .arg(score)
            .arg(price.to_string())
            .query_async(&mut conn)
            .await?;
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(REDIS_INTRADAY_TTL_SECS)
            .query_async(&mut conn)
            .await?;
    }
    tracing::info!("redis push intraday: {} points at {}", n, date_str);
    Ok(())
}

#[cfg(feature = "redis")]
async fn redis_fetch_intraday_today(
    mut conn: redis::aio::MultiplexedConnection,
    symbol: &str,
) -> anyhow::Result<Vec<(String, f64)>> {
    let now_secs = current_time_secs();
    let date_str = chrono::Utc
        .timestamp_opt(now_secs, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string());
    let key = redis_intraday_key(symbol, &date_str);
    let raw: Vec<String> = redis::cmd("ZRANGE")
        .arg(&key)
        .arg(0)
        .arg(-1)
        .arg("WITHSCORES")
        .query_async(&mut conn)
        .await?;
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < raw.len() {
        let price_str = &raw[i];
        let score_str = &raw[i + 1];
        if let (Ok(price), Ok(score)) = (price_str.parse::<f64>(), score_str.parse::<f64>()) {
            let observed_at = chrono::Utc
                .timestamp_opt(score as i64, 0)
                .single()
                .map(|dt| format!("{}", dt.format("%Y-%m-%dT%H:%M:%S%.3fZ")))
                .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string());
            out.push((observed_at, price));
        }
        i += 2;
    }
    Ok(out)
}

async fn persist_intraday_quotes(
    pool: &sqlx::PgPool,
    items: &[(String, f64)],
    observed_at_secs: i64,
) -> anyhow::Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let observed_at = observed_at_secs as f64;
    for (symbol, price) in items {
        sqlx::query(
            r#"
            INSERT INTO stock_monitoring.price_history_intraday (symbol, observed_at, price)
            VALUES ($1, to_timestamp($2), $3)
            "#,
        )
        .bind(symbol)
        .bind(observed_at)
        .bind(price)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct WatchlistDto {
    id: i64,
    name: String,
    item_count: i64,
}

#[derive(Debug, Deserialize)]
struct CreateWatchlistRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct UpdateWatchlistRequest {
    name: String,
}

#[derive(Debug, Serialize)]
struct WatchlistItemDto {
    id: i64,
    symbol: String,
    mic: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddWatchlistItemRequest {
    symbol: String,
    mic: Option<String>,
}

#[derive(Debug, Serialize)]
struct AlertDto {
    id: i64,
    symbol: String,
    mic: Option<String>,
    condition_type: String,
    condition_value: f64,
    enabled: bool,
    triggered: bool,
}

#[derive(Debug, Deserialize)]
struct CreateAlertRequest {
    symbol: String,
    mic: Option<String>,
    condition_type: String,
    condition_value: f64,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateAlertRequest {
    symbol: Option<String>,
    mic: Option<String>,
    condition_type: Option<String>,
    condition_value: Option<f64>,
    enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct DeleteResult {
    ok: bool,
}

#[derive(Debug, Serialize)]
struct NotificationDto {
    id: i64,
    alert_id: Option<i64>,
    message: String,
    read: bool,
    created_at_ms: i64,
}

#[derive(Debug, Deserialize)]
struct NotificationsQuery {
    include_read: Option<bool>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct MarkReadResult {
    ok: bool,
}

async fn list_watchlists(
    State(state): State<AppState>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Result<Json<Vec<WatchlistDto>>, (StatusCode, Json<serde_json::Value>)> {
    let rows = sqlx::query_as::<_, (i64, String, i64)>(
        r#"
        SELECT w.id, w.name, COALESCE(COUNT(i.id), 0) AS item_count
        FROM stock_monitoring.watchlists w
        LEFT JOIN stock_monitoring.watchlist_items i ON i.watchlist_id = w.id
        WHERE w.user_id = $1
        GROUP BY w.id, w.name, w.updated_at
        ORDER BY w.updated_at DESC, w.id DESC
        "#,
    )
    .bind(&user_id.0)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let out = rows
        .into_iter()
        .map(|(id, name, item_count)| WatchlistDto { id, name, item_count })
        .collect::<Vec<_>>();
    Ok(Json(out))
}

async fn create_watchlist(
    State(state): State<AppState>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
    Json(body): Json<CreateWatchlistRequest>,
) -> Result<(StatusCode, Json<WatchlistDto>), (StatusCode, Json<serde_json::Value>)> {
    let name = body.name.trim();
    if name.is_empty() || name.len() > 255 {
        return Err(bad_request("Invalid watchlist name"));
    }

    let inserted = sqlx::query_as::<_, (i64, String)>(
        r#"
        INSERT INTO stock_monitoring.watchlists (user_id, name)
        VALUES ($1, $2)
        RETURNING id, name
        "#,
    )
    .bind(&user_id.0)
    .bind(name)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("duplicate key") || msg.contains("unique") {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Watchlist already exists for this user"
                })),
            )
        } else {
            internal_error(e)
        }
    })?;

    write_audit(
        &state.pool,
        Some(&user_id.0),
        "watchlist.create",
        Some("watchlist"),
        Some(&inserted.0.to_string()),
        Some(serde_json::json!({ "name": inserted.1 })),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(WatchlistDto {
            id: inserted.0,
            name: inserted.1,
            item_count: 0,
        }),
    ))
}

async fn delete_watchlist(
    State(state): State<AppState>,
    Path((watchlist_id,)): Path<(i64,)>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Result<Json<DeleteResult>, (StatusCode, Json<serde_json::Value>)> {
    let deleted = sqlx::query_as::<_, (i64, String)>(
        r#"
        DELETE FROM stock_monitoring.watchlists
        WHERE id = $1 AND user_id = $2
        RETURNING id, name
        "#,
    )
    .bind(watchlist_id)
    .bind(&user_id.0)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    let Some((id, name)) = deleted else {
        return Err(not_found("Watchlist not found"));
    };

    write_audit(
        &state.pool,
        Some(&user_id.0),
        "watchlist.delete",
        Some("watchlist"),
        Some(&id.to_string()),
        Some(serde_json::json!({ "name": name })),
    )
    .await;

    Ok(Json(DeleteResult { ok: true }))
}

async fn update_watchlist(
    State(state): State<AppState>,
    Path((watchlist_id,)): Path<(i64,)>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
    Json(body): Json<UpdateWatchlistRequest>,
) -> Result<Json<WatchlistDto>, (StatusCode, Json<serde_json::Value>)> {
    let name = body.name.trim();
    if name.is_empty() || name.len() > 255 {
        return Err(bad_request("Invalid watchlist name"));
    }

    let updated = sqlx::query_as::<_, (i64, String, i64)>(
        r#"
        UPDATE stock_monitoring.watchlists w
        SET name = $1,
            updated_at = CURRENT_TIMESTAMP
        WHERE w.id = $2
          AND w.user_id = $3
        RETURNING
          w.id,
          w.name,
          (
            SELECT COALESCE(COUNT(i.id), 0)
            FROM stock_monitoring.watchlist_items i
            WHERE i.watchlist_id = w.id
          ) AS item_count
        "#,
    )
    .bind(name)
    .bind(watchlist_id)
    .bind(&user_id.0)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("duplicate key") || msg.contains("unique") {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Watchlist already exists for this user"
                })),
            )
        } else {
            internal_error(e)
        }
    })?;

    let Some((id, new_name, item_count)) = updated else {
        return Err(not_found("Watchlist not found"));
    };

    write_audit(
        &state.pool,
        Some(&user_id.0),
        "watchlist.update",
        Some("watchlist"),
        Some(&id.to_string()),
        Some(serde_json::json!({ "name": new_name })),
    )
    .await;

    Ok(Json(WatchlistDto {
        id,
        name: new_name,
        item_count,
    }))
}

async fn list_watchlist_items(
    State(state): State<AppState>,
    Path((watchlist_id,)): Path<(i64,)>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Result<Json<Vec<WatchlistItemDto>>, (StatusCode, Json<serde_json::Value>)> {
    ensure_watchlist_owned(&state.pool, watchlist_id, &user_id.0).await?;

    let rows = sqlx::query_as::<_, (i64, String, Option<String>)>(
        r#"
        SELECT i.id, i.symbol, i.mic
        FROM stock_monitoring.watchlist_items i
        WHERE i.watchlist_id = $1
        ORDER BY i.id DESC
        "#,
    )
    .bind(watchlist_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(|(id, symbol, mic)| WatchlistItemDto { id, symbol, mic })
        .collect::<Vec<_>>();
    Ok(Json(items))
}

async fn add_watchlist_item(
    State(state): State<AppState>,
    Path((watchlist_id,)): Path<(i64,)>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
    Json(body): Json<AddWatchlistItemRequest>,
) -> Result<(StatusCode, Json<WatchlistItemDto>), (StatusCode, Json<serde_json::Value>)> {
    ensure_watchlist_owned(&state.pool, watchlist_id, &user_id.0).await?;

    let symbol = normalize_symbol(&body.symbol)?;
    let mic = normalize_mic(body.mic.as_deref())?;

    let inserted = sqlx::query_as::<_, (i64, String, Option<String>)>(
        r#"
        INSERT INTO stock_monitoring.watchlist_items (watchlist_id, symbol, mic)
        VALUES ($1, $2, $3)
        RETURNING id, symbol, mic
        "#,
    )
    .bind(watchlist_id)
    .bind(&symbol)
    .bind(&mic)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("duplicate key") || msg.contains("unique") {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Symbol already exists in this watchlist"
                })),
            )
        } else {
            internal_error(e)
        }
    })?;

    write_audit(
        &state.pool,
        Some(&user_id.0),
        "watchlist_item.add",
        Some("watchlist_item"),
        Some(&inserted.0.to_string()),
        Some(serde_json::json!({
            "watchlist_id": watchlist_id,
            "symbol": inserted.1,
            "mic": inserted.2
        })),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(WatchlistItemDto {
            id: inserted.0,
            symbol: inserted.1,
            mic: inserted.2,
        }),
    ))
}

async fn delete_watchlist_item(
    State(state): State<AppState>,
    Path((watchlist_id, item_id)): Path<(i64, i64)>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Result<Json<DeleteResult>, (StatusCode, Json<serde_json::Value>)> {
    ensure_watchlist_owned(&state.pool, watchlist_id, &user_id.0).await?;

    let deleted = sqlx::query_as::<_, (i64, String, Option<String>)>(
        r#"
        DELETE FROM stock_monitoring.watchlist_items
        WHERE id = $1 AND watchlist_id = $2
        RETURNING id, symbol, mic
        "#,
    )
    .bind(item_id)
    .bind(watchlist_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    let Some((id, symbol, mic)) = deleted else {
        return Err(not_found("Watchlist item not found"));
    };

    write_audit(
        &state.pool,
        Some(&user_id.0),
        "watchlist_item.delete",
        Some("watchlist_item"),
        Some(&id.to_string()),
        Some(serde_json::json!({
            "watchlist_id": watchlist_id,
            "symbol": symbol,
            "mic": mic
        })),
    )
    .await;

    Ok(Json(DeleteResult { ok: true }))
}

async fn list_alerts(
    State(state): State<AppState>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Result<Json<Vec<AlertDto>>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(err) = evaluate_alerts_for_user(&state, &user_id.0).await {
        tracing::warn!("Alert evaluation skipped: {}", err);
    }

    let rows = sqlx::query_as::<_, (i64, String, Option<String>, String, f64, bool, bool)>(
        r#"
        SELECT
            id,
            symbol,
            mic,
            condition_type,
            condition_value::double precision,
            enabled,
            (notified_at IS NOT NULL) AS triggered
        FROM stock_monitoring.alerts
        WHERE user_id = $1
        ORDER BY updated_at DESC, id DESC
        "#,
    )
    .bind(&user_id.0)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let out = rows
        .into_iter()
        .map(
            |(id, symbol, mic, condition_type, condition_value, enabled, triggered)| AlertDto {
                id,
                symbol,
                mic,
                condition_type,
                condition_value,
                enabled,
                triggered,
            },
        )
        .collect::<Vec<_>>();
    Ok(Json(out))
}

async fn list_notifications(
    State(state): State<AppState>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
    Query(query): Query<NotificationsQuery>,
) -> Result<Json<Vec<NotificationDto>>, (StatusCode, Json<serde_json::Value>)> {
    let include_read = query.include_read.unwrap_or(false);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let rows = sqlx::query_as::<_, (i64, Option<i64>, String, bool, i64)>(
        r#"
        SELECT
            id,
            alert_id,
            message,
            (read_at IS NOT NULL) AS is_read,
            (EXTRACT(EPOCH FROM created_at) * 1000)::bigint AS created_at_ms
        FROM stock_monitoring.notifications
        WHERE user_id = $1
          AND ($2::boolean = true OR read_at IS NULL)
        ORDER BY created_at DESC, id DESC
        LIMIT $3
        "#,
    )
    .bind(&user_id.0)
    .bind(include_read)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let out = rows
        .into_iter()
        .map(|(id, alert_id, message, is_read, created_at_ms)| NotificationDto {
            id,
            alert_id,
            message,
            read: is_read,
            created_at_ms,
        })
        .collect::<Vec<_>>();
    Ok(Json(out))
}

async fn mark_notification_read(
    State(state): State<AppState>,
    Path((notification_id,)): Path<(i64,)>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Result<Json<MarkReadResult>, (StatusCode, Json<serde_json::Value>)> {
    let updated = sqlx::query(
        r#"
        UPDATE stock_monitoring.notifications
        SET read_at = CURRENT_TIMESTAMP
        WHERE id = $1
          AND user_id = $2
          AND read_at IS NULL
        "#,
    )
    .bind(notification_id)
    .bind(&user_id.0)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(MarkReadResult {
        ok: updated.rows_affected() > 0,
    }))
}

async fn mark_all_notifications_read(
    State(state): State<AppState>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Result<Json<MarkReadResult>, (StatusCode, Json<serde_json::Value>)> {
    let updated = sqlx::query(
        r#"
        UPDATE stock_monitoring.notifications
        SET read_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
          AND read_at IS NULL
        "#,
    )
    .bind(&user_id.0)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(MarkReadResult {
        ok: updated.rows_affected() > 0,
    }))
}

async fn evaluate_alerts_internal(
    State(state): State<AppState>,
) -> Result<Json<AlertEvaluationResult>, (StatusCode, Json<serde_json::Value>)> {
    let users = evaluate_alerts_all_users(&state).await.map_err(internal_error)?;
    Ok(Json(AlertEvaluationResult {
        ok: true,
        users_evaluated: users,
        evaluated_at_ms: current_time_ms(),
    }))
}

async fn ingest_quotes_internal(
    State(state): State<AppState>,
    Json(body): Json<IngestQuotesRequest>,
) -> Result<Json<IngestQuotesResult>, (StatusCode, Json<serde_json::Value>)> {
    let now_ms = current_time_ms();
    let observed_at_secs = current_time_secs();
    let mut ingested = 0usize;
    let mut intraday_items: Vec<(String, f64)> = Vec::new();
    let mut cache = state.quote_service.cache.write().await;
    for item in body.quotes {
        let symbol = item.symbol.trim().to_uppercase();
        if symbol.is_empty() || !item.price.is_finite() || item.price <= 0.0 {
            continue;
        }
        intraday_items.push((symbol.clone(), item.price));
        cache.insert(
            symbol.clone(),
            CachedQuote {
                symbol,
                price: item.price,
                currency: item.currency.filter(|s| !s.trim().is_empty()),
                fetched_at_ms: now_ms,
            },
        );
        ingested += 1;
    }
    drop(cache);

    if !intraday_items.is_empty() {
        let pool = state.pool.clone();
        let for_db = intraday_items.clone();
        tokio::spawn(async move {
            if let Err(e) = persist_intraday_quotes(&pool, &for_db, observed_at_secs).await {
                tracing::warn!("ingest persist intraday: {}", e);
            }
        });
        #[cfg(feature = "redis")]
        if let Some(redis) = state.redis.clone() {
            let items = intraday_items;
            let observed = observed_at_secs;
            tokio::spawn(async move {
                if let Err(e) = redis_push_intraday(redis, &items, observed).await {
                    tracing::warn!("ingest redis push intraday: {}", e);
                }
            });
        }
    }

    Ok(Json(IngestQuotesResult { ok: true, ingested }))
}

/// Finnhub webhook: acknowledge with 2xx immediately to prevent timeouts.
/// Verifies X-Finnhub-Secret when FINNHUB_WEBHOOK_SECRET is set.
/// Empty-body requests (e.g. Finnhub dashboard "Test") are accepted without secret so test returns 200.
async fn finnhub_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Body,
) -> StatusCode {
    if let Some(ref secret) = state.finnhub_webhook_secret {
        let incoming = headers
            .get("x-finnhub-secret")
            .and_then(|v| v.to_str().ok())
            .map(str::trim);
        let content_length = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        // Accept without secret: empty or small body (e.g. Finnhub dashboard test sends small JSON)
        let is_likely_test = content_length <= 512;
        if incoming.as_deref() != Some(secret.as_str()) && !is_likely_test {
            return StatusCode::UNAUTHORIZED;
        }
    }
    // Acknowledge before any processing to prevent Finnhub disabling the endpoint.
    tokio::spawn(async move {
        let _ = axum::body::to_bytes(body, usize::MAX).await;
        // Optionally log or process the event here.
    });
    StatusCode::OK
}

async fn evaluate_alerts_all_users(state: &AppState) -> anyhow::Result<usize> {
    let user_ids = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT user_id
        FROM stock_monitoring.alerts
        WHERE enabled = true
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    for user_id in &user_ids {
        if let Err(err) = evaluate_alerts_for_user(state, user_id).await {
            tracing::warn!("Alert evaluation failed for user {}: {}", user_id, err);
        }
    }
    Ok(user_ids.len())
}

async fn evaluate_alerts_for_user(state: &AppState, user_id: &str) -> anyhow::Result<()> {
    let alerts = sqlx::query_as::<_, (i64, String, String, f64, bool)>(
        r#"
        SELECT
            id,
            symbol,
            condition_type,
            condition_value::double precision,
            (notified_at IS NOT NULL) AS was_notified
        FROM stock_monitoring.alerts
        WHERE user_id = $1
          AND enabled = true
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    if alerts.is_empty() {
        return Ok(());
    }

    let mut symbols = alerts.iter().map(|(_, symbol, _, _, _)| symbol.clone()).collect::<Vec<_>>();
    symbols.sort();
    symbols.dedup();
    let quotes = fetch_provider_quotes(&state.quote_service, &symbols).await?;

    let mut price_by_symbol = HashMap::new();
    for quote in quotes {
        price_by_symbol.insert(quote.symbol.to_uppercase(), quote);
    }

    for (alert_id, symbol, condition_type, condition_value, was_notified) in alerts {
        let Some(quote) = price_by_symbol.get(&symbol.to_uppercase()) else {
            continue;
        };
        let condition_met = match condition_type.as_str() {
            "above" => quote.price >= condition_value,
            "below" => quote.price <= condition_value,
            _ => false,
        };

        if condition_met && !was_notified {
            let updated = sqlx::query(
                r#"
                UPDATE stock_monitoring.alerts
                SET notified_at = CURRENT_TIMESTAMP,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $1 AND user_id = $2 AND notified_at IS NULL
                "#,
            )
            .bind(alert_id)
            .bind(user_id)
            .execute(&state.pool)
            .await?;
            if updated.rows_affected() > 0 {
                let direction_label = if condition_type == "above" {
                    "above"
                } else {
                    "below"
                };
                let threshold = format!("{:.4}", condition_value);
                let current_price = format!("{:.4}", quote.price);
                let currency_label = quote
                    .currency
                    .as_deref()
                    .map(|v| format!(" {}", v))
                    .unwrap_or_default();
                let message = format!(
                    "Alert triggered for {}: price is {} {} (current {}{}).",
                    symbol, direction_label, threshold, current_price, currency_label
                );
                write_notification(
                    &state.pool,
                    user_id,
                    Some(alert_id),
                    &message,
                    Some(serde_json::json!({
                        "symbol": symbol,
                        "condition_type": condition_type,
                        "condition_value": condition_value,
                        "price": quote.price,
                        "currency": quote.currency
                    })),
                )
                .await;
                write_audit(
                    &state.pool,
                    Some(user_id),
                    "alert.trigger",
                    Some("alert"),
                    Some(&alert_id.to_string()),
                    Some(serde_json::json!({
                        "symbol": symbol,
                        "condition_type": condition_type,
                        "condition_value": condition_value,
                        "price": quote.price,
                        "currency": quote.currency
                    })),
                )
                .await;
            }
        } else if !condition_met && was_notified {
            let updated = sqlx::query(
                r#"
                UPDATE stock_monitoring.alerts
                SET notified_at = NULL,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $1 AND user_id = $2 AND notified_at IS NOT NULL
                "#,
            )
            .bind(alert_id)
            .bind(user_id)
            .execute(&state.pool)
            .await?;
            if updated.rows_affected() > 0 {
                write_audit(
                    &state.pool,
                    Some(user_id),
                    "alert.reset",
                    Some("alert"),
                    Some(&alert_id.to_string()),
                    Some(serde_json::json!({
                        "symbol": symbol,
                        "condition_type": condition_type,
                        "condition_value": condition_value,
                        "price": quote.price,
                        "currency": quote.currency
                    })),
                )
                .await;
            }
        }
    }

    Ok(())
}

async fn create_alert(
    State(state): State<AppState>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
    Json(body): Json<CreateAlertRequest>,
) -> Result<(StatusCode, Json<AlertDto>), (StatusCode, Json<serde_json::Value>)> {
    let symbol = normalize_symbol(&body.symbol)?;
    let mic = normalize_mic(body.mic.as_deref())?;
    let condition_type = normalize_condition_type(&body.condition_type)?;
    if !body.condition_value.is_finite() || body.condition_value <= 0.0 {
        return Err(bad_request("Invalid condition_value"));
    }
    let enabled = body.enabled.unwrap_or(true);

    let inserted = sqlx::query_as::<_, (i64, String, Option<String>, String, f64, bool, bool)>(
        r#"
        INSERT INTO stock_monitoring.alerts (user_id, symbol, mic, condition_type, condition_value, enabled)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, symbol, mic, condition_type, condition_value::double precision, enabled, (notified_at IS NOT NULL)
        "#,
    )
    .bind(&user_id.0)
    .bind(&symbol)
    .bind(&mic)
    .bind(&condition_type)
    .bind(body.condition_value)
    .bind(enabled)
    .fetch_one(&state.pool)
    .await
    .map_err(internal_error)?;

    write_audit(
        &state.pool,
        Some(&user_id.0),
        "alert.create",
        Some("alert"),
        Some(&inserted.0.to_string()),
        Some(serde_json::json!({
            "symbol": inserted.1,
            "mic": inserted.2,
            "condition_type": inserted.3,
            "condition_value": inserted.4,
            "enabled": inserted.5
        })),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(AlertDto {
            id: inserted.0,
            symbol: inserted.1,
            mic: inserted.2,
            condition_type: inserted.3,
            condition_value: inserted.4,
            enabled: inserted.5,
            triggered: inserted.6,
        }),
    ))
}

async fn update_alert(
    State(state): State<AppState>,
    Path((alert_id,)): Path<(i64,)>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
    Json(body): Json<UpdateAlertRequest>,
) -> Result<Json<AlertDto>, (StatusCode, Json<serde_json::Value>)> {
    let current = sqlx::query_as::<_, (String, Option<String>, String, f64, bool)>(
        r#"
        SELECT symbol, mic, condition_type, condition_value::double precision, enabled
        FROM stock_monitoring.alerts
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(alert_id)
    .bind(&user_id.0)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    let Some(existing) = current else {
        return Err(not_found("Alert not found"));
    };

    let symbol = match body.symbol.as_deref() {
        Some(v) => normalize_symbol(v)?,
        None => existing.0,
    };
    let mic = match body.mic {
        Some(ref v) => normalize_mic(Some(v))?,
        None => existing.1,
    };
    let condition_type = match body.condition_type.as_deref() {
        Some(v) => normalize_condition_type(v)?,
        None => existing.2,
    };
    let condition_value = match body.condition_value {
        Some(v) => {
            if !v.is_finite() || v <= 0.0 {
                return Err(bad_request("Invalid condition_value"));
            }
            v
        }
        None => existing.3,
    };
    let enabled = body.enabled.unwrap_or(existing.4);

    let updated = sqlx::query_as::<_, (i64, String, Option<String>, String, f64, bool, bool)>(
        r#"
        UPDATE stock_monitoring.alerts
        SET symbol = $1,
            mic = $2,
            condition_type = $3,
            condition_value = $4,
            enabled = $5,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $6 AND user_id = $7
        RETURNING id, symbol, mic, condition_type, condition_value::double precision, enabled, (notified_at IS NOT NULL)
        "#,
    )
    .bind(&symbol)
    .bind(&mic)
    .bind(&condition_type)
    .bind(condition_value)
    .bind(enabled)
    .bind(alert_id)
    .bind(&user_id.0)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    let Some(updated) = updated else {
        return Err(not_found("Alert not found"));
    };

    write_audit(
        &state.pool,
        Some(&user_id.0),
        "alert.update",
        Some("alert"),
        Some(&updated.0.to_string()),
        Some(serde_json::json!({
            "symbol": updated.1,
            "mic": updated.2,
            "condition_type": updated.3,
            "condition_value": updated.4,
            "enabled": updated.5
        })),
    )
    .await;

    Ok(Json(AlertDto {
        id: updated.0,
        symbol: updated.1,
        mic: updated.2,
        condition_type: updated.3,
        condition_value: updated.4,
        enabled: updated.5,
        triggered: updated.6,
    }))
}

async fn delete_alert(
    State(state): State<AppState>,
    Path((alert_id,)): Path<(i64,)>,
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Result<Json<DeleteResult>, (StatusCode, Json<serde_json::Value>)> {
    let deleted = sqlx::query_as::<_, (i64, String, String, f64)>(
        r#"
        DELETE FROM stock_monitoring.alerts
        WHERE id = $1 AND user_id = $2
        RETURNING id, symbol, condition_type, condition_value::double precision
        "#,
    )
    .bind(alert_id)
    .bind(&user_id.0)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    let Some((id, symbol, condition_type, condition_value)) = deleted else {
        return Err(not_found("Alert not found"));
    };

    write_audit(
        &state.pool,
        Some(&user_id.0),
        "alert.delete",
        Some("alert"),
        Some(&id.to_string()),
        Some(serde_json::json!({
            "symbol": symbol,
            "condition_type": condition_type,
            "condition_value": condition_value
        })),
    )
    .await;

    Ok(Json(DeleteResult { ok: true }))
}

async fn ensure_watchlist_owned(
    pool: &sqlx::PgPool,
    watchlist_id: i64,
    user_id: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM stock_monitoring.watchlists WHERE id = $1 AND user_id = $2)",
    )
    .bind(watchlist_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(internal_error)?;

    if !exists {
        return Err(not_found("Watchlist not found"));
    }
    Ok(())
}

fn normalize_symbol(raw: &str) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let symbol = raw.trim().to_uppercase();
    if symbol.is_empty()
        || symbol.len() > 32
        || !symbol
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(bad_request("Invalid symbol format"));
    }
    Ok(symbol)
}

fn normalize_mic(raw: Option<&str>) -> Result<Option<String>, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let mic = value.to_uppercase();
    // EU-focused allowed MICs for MVP.
    const ALLOWED_MICS: &[&str] = &[
        "XPAR", "XAMS", "XBRU", "XLIS", "XDUB", "XOSL", "XETR", "XFRA", "XLON", "XSWX", "XMIL",
        "XMAD", "XSTO", "XHEL", "XCSE", "XICE",
    ];
    if mic.len() > 8 || !mic.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(bad_request("Invalid MIC format"));
    }
    if !ALLOWED_MICS.contains(&mic.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Unsupported MIC for EU MVP",
                "detail": format!("MIC {} is not in allowed EU list", mic)
            })),
        ));
    }
    Ok(Some(mic))
}

fn normalize_condition_type(raw: &str) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let value = raw.trim().to_lowercase();
    match value.as_str() {
        "above" | "below" => Ok(value),
        _ => Err(bad_request("Unsupported condition_type")),
    }
}

async fn write_audit(
    pool: &sqlx::PgPool,
    user_id: Option<&str>,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    details: Option<serde_json::Value>,
) {
    let _ = sqlx::query(
        r#"
        INSERT INTO stock_monitoring.audit_log (user_id, action, resource_type, resource_id, details)
        VALUES ($1, $2, $3, $4, $5::jsonb)
        "#,
    )
    .bind(user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(details.unwrap_or_else(|| serde_json::json!({})).to_string())
    .execute(pool)
    .await;
}

async fn write_notification(
    pool: &sqlx::PgPool,
    user_id: &str,
    alert_id: Option<i64>,
    message: &str,
    details: Option<serde_json::Value>,
) {
    let _ = sqlx::query(
        r#"
        INSERT INTO stock_monitoring.notifications (user_id, alert_id, message, details)
        VALUES ($1, $2, $3, $4::jsonb)
        "#,
    )
    .bind(user_id)
    .bind(alert_id)
    .bind(message)
    .bind(details.unwrap_or_else(|| serde_json::json!({})).to_string())
    .execute(pool)
    .await;
}

fn bad_request(message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": message
        })),
    )
}

fn not_found(message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": message
        })),
    )
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "Internal server error",
            "detail": err.to_string()
        })),
    )
}

async fn me(
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
    display_name: Option<axum::extract::Extension<UserDisplayName>>,
    email: Option<axum::extract::Extension<UserEmail>>,
) -> Json<serde_json::Value> {
    let resolved_email = email.and_then(|axum::extract::Extension(v)| v.0);
    let resolved_display_name = display_name
        .map(|axum::extract::Extension(v)| v.0)
        .filter(|v| !v.trim().is_empty())
        .filter(|v| v != &user_id.0)
        .filter(|v| !looks_like_user_id(v))
        .or_else(|| {
            resolved_email
                .as_deref()
                .and_then(|v| v.split('@').next())
                .map(String::from)
        });
    Json(serde_json::json!({
        "user_id": user_id.0,
        "display_name": resolved_display_name,
        "email": resolved_email
    }))
}

#[cfg(test)]
mod tests {
    use super::{infer_currency_from_symbol, map_to_alpha_vantage_symbol};

    #[test]
    fn infer_currency_from_symbol_eur_exchanges() {
        assert_eq!(infer_currency_from_symbol("SAP.DE"), Some("EUR".to_string()));
        assert_eq!(infer_currency_from_symbol("ASML.AS"), Some("EUR".to_string()));
        assert_eq!(infer_currency_from_symbol("AIR.PA"), Some("EUR".to_string()));
    }

    #[test]
    fn infer_currency_from_symbol_gbp_sek_sto() {
        assert_eq!(infer_currency_from_symbol("SHEL.L"), Some("GBP".to_string()));
        assert_eq!(infer_currency_from_symbol("VOLV-B.ST"), Some("SEK".to_string()));
        assert_eq!(infer_currency_from_symbol("ERIC-B.ST"), Some("SEK".to_string()));
    }

    #[test]
    fn infer_currency_from_symbol_us_style_returns_usd() {
        assert_eq!(infer_currency_from_symbol("AAPL"), Some("USD".to_string()));
        assert_eq!(infer_currency_from_symbol("MSFT"), Some("USD".to_string()));
        assert_eq!(infer_currency_from_symbol("MSFT.US"), Some("USD".to_string()));
    }

    #[test]
    fn infer_currency_from_symbol_unknown_returns_none() {
        assert_eq!(infer_currency_from_symbol("ABCDEF"), None);
        assert_eq!(infer_currency_from_symbol("X.XX"), None);
    }

    #[test]
    fn map_to_alpha_vantage_stockholm() {
        assert_eq!(map_to_alpha_vantage_symbol("VOLV-B.ST"), "VOLV-B.STO");
        assert_eq!(map_to_alpha_vantage_symbol("eric-b.st"), "ERIC-B.STO");
    }

    #[test]
    fn map_to_alpha_vantage_other_unchanged() {
        assert_eq!(map_to_alpha_vantage_symbol("SAP.DE"), "SAP.DE");
        assert_eq!(map_to_alpha_vantage_symbol("ASML.AS"), "ASML.AS");
    }
}
