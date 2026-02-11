//! Axum router and handlers. Exposed for integration tests (oneshot) and main binary.

use axum::{
    extract::{Query, State},
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

use crate::auth::{KeycloakJwtValidator, UserId};
use crate::config::AppConfig;

#[derive(Clone)]
pub struct QuoteService {
    pub provider: String,
    pub cache_ttl: Duration,
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

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub validator: Arc<KeycloakJwtValidator>,
    pub quote_service: QuoteService,
}

/// Build the application router (used by main and by API integration tests).
pub fn create_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/quotes", get(quotes));

    let protected = Router::new()
        .route("/api/v1/me", get(me))
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
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(String::from));
    let token = match token {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Missing authorization token"})),
            )
                .into_response()
        }
    };
    match state.validator.validate_bearer(&token).await {
        Ok(sub) => {
            req.extensions_mut().insert(UserId(sub));
            next.run(req).await
        }
        Err(e) => {
            tracing::debug!("JWT validation failed: {}", e);
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid or expired token"})),
            )
                .into_response()
        }
    }
}

async fn health() -> &'static str {
    "ok"
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
        match fetch_provider_quotes(&state.quote_service.http, &missing).await {
            Ok(items) => {
                let mut write_cache = state.quote_service.cache.write().await;
                for item in items {
                    let quote = CachedQuote {
                        symbol: item.symbol.clone(),
                        price: item.price,
                        currency: item.currency,
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
    http: &reqwest::Client,
    symbols: &[String],
) -> anyhow::Result<Vec<CachedQuote>> {
    let mut all = Vec::new();

    // First attempt: Yahoo bulk endpoint.
    match fetch_yahoo_quotes(http, symbols).await {
        Ok(mut quotes) => {
            all.append(&mut quotes);
        }
        Err(err) => {
            tracing::warn!("Yahoo quote fetch failed: {}", err);
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

    if all.is_empty() {
        anyhow::bail!("Provider fetch returned no data (Yahoo/Stooq)");
    }
    Ok(all)
}

async fn fetch_yahoo_quotes(
    http: &reqwest::Client,
    symbols: &[String],
) -> anyhow::Result<Vec<CachedQuote>> {
    let url = "https://query1.finance.yahoo.com/v7/finance/quote";
    let joined = symbols.join(",");
    let resp = http
        .get(url)
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
                currency: item.currency,
                fetched_at_ms: current_time_ms(),
            });
        }
    }
    Ok(out)
}

async fn fetch_stooq_quote(http: &reqwest::Client, symbol: &str) -> Option<CachedQuote> {
    let stooq_symbol = to_stooq_symbol(symbol);
    let url = format!("https://stooq.com/q/l/?s={}&i=d", stooq_symbol.to_lowercase());
    let text = http.get(url).send().await.ok()?.error_for_status().ok()?.text().await.ok()?;
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
        currency: None,
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

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

async fn me(
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "user_id": user_id.0 }))
}
