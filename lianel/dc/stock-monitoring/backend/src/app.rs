//! Axum router and handlers. Exposed for integration tests (oneshot) and main binary.

use axum::{
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware,
    response::IntoResponse,
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

use crate::auth::{KeycloakJwtValidator, UserId};

#[derive(Clone)]
pub struct QuoteService {
    pub provider: String,
    pub cache_ttl: Duration,
    pub data_provider_api_key: Option<String>,
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
        .route("/internal/alerts/evaluate", post(evaluate_alerts_internal));

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
struct AlertEvaluationResult {
    ok: bool,
    users_evaluated: usize,
    evaluated_at_ms: u64,
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
                for item in items {
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
    quote_service: &QuoteService,
    symbols: &[String],
) -> anyhow::Result<Vec<CachedQuote>> {
    let mut all = Vec::new();
    let http = &quote_service.http;

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
        anyhow::bail!("Provider fetch returned no data (Yahoo/Stooq)");
    }
    Ok(all)
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

fn map_to_alpha_vantage_symbol(symbol: &str) -> String {
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

fn infer_currency_from_symbol(symbol: &str) -> Option<String> {
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
    None
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
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
