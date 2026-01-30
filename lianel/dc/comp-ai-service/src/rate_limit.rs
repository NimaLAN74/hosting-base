//! In-memory per-IP rate limiting for API routes.
//! Key is taken from X-Real-IP or X-Forwarded-For (first value) when behind a proxy.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tower::{Layer, Service};

/// Per-key (e.g. IP) rate limit: fixed window, max `limit` requests per `window`.
#[derive(Clone)]
pub struct RateLimitState {
    inner: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
    limit: u32,
    window: Duration,
}

impl RateLimitState {
    pub fn new(limit: u32, window_secs: u64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            limit,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Returns Ok(()) if under limit, Err(StatusCode) if over limit.
    pub async fn check(&self, key: &str) -> Result<(), StatusCode> {
        if self.limit == 0 {
            return Ok(());
        }
        let now = Instant::now();
        let mut guard = self.inner.write().await;
        let entry = guard.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) >= self.window {
            *entry = (1, now);
            return Ok(());
        }
        entry.0 += 1;
        if entry.0 > self.limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        Ok(())
    }
}

/// Tower layer that applies rate limiting using client IP (or X-Real-IP / X-Forwarded-For).
#[derive(Clone)]
pub struct RateLimitLayer {
    state: RateLimitState,
}

impl RateLimitLayer {
    pub fn new(state: RateLimitState) -> Self {
        Self { state }
    }
}

fn client_key_from_request(req: &Request) -> String {
    let headers = req.headers();
    if let Some(v) = headers.get("X-Real-IP") {
        if let Ok(s) = v.to_str() {
            return s.trim().to_string();
        }
    }
    if let Some(v) = headers.get("X-Forwarded-For") {
        if let Ok(s) = v.to_str() {
            return s.split(',').next().map(str::trim).unwrap_or("").to_string();
        }
    }
    "unknown".to_string()
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    state: RateLimitState,
}

impl<S> Service<Request> for RateLimitMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let state = self.state.clone();
        let key = client_key_from_request(&req);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if let Err(code) = state.check(&key).await {
                return Ok(
                    (
                        code,
                        [(header::RETRY_AFTER, state.window.as_secs().to_string())],
                        axum::Json(serde_json::json!({
                            "error": "Too many requests",
                            "retry_after_secs": state.window.as_secs()
                        })),
                    )
                        .into_response(),
                );
            }
            inner.call(req).await
        })
    }
}
