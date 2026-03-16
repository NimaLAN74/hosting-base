//! Cache today's intraday price points in Redis so restarts/deployments don't lose them.
//! Set REDIS_URL to enable. Key: stock:today:{symbol}, value: list of "{unix_ts_sec}:{price}".

use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

const KEY_PREFIX: &str = "stock:today:";
const MAX_POINTS_PER_SYMBOL: isize = 600; // ~10h at 1/min

/// Push one (timestamp_sec, price) for symbol. No-op if redis is None.
pub async fn push_price(
    redis: Option<&redis::aio::ConnectionManager>,
    symbol: &str,
    ts_sec: u64,
    price: f64,
) {
    let Some(conn) = redis else {
        return;
    };
    let key = format!("{KEY_PREFIX}{symbol}");
    let value = format!("{}:{}", ts_sec, price);
    let _: Result<(), redis::RedisError> = conn.rpush(&key, &value).await;
    let _: Result<(), redis::RedisError> = conn.ltrim(&key, -MAX_POINTS_PER_SYMBOL, -1).await;
}

/// Return today's points for symbol (ts_sec, price), sorted by time. Empty if Redis unavailable or no data.
pub async fn get_today(
    redis: Option<&redis::aio::ConnectionManager>,
    symbol: &str,
) -> Vec<(u64, f64)> {
    let Some(conn) = redis else {
        return vec![];
    };
    let key = format!("{KEY_PREFIX}{symbol}");
    let raw: Result<Vec<String>, redis::RedisError> = conn.lrange(&key, 0, -1).await;
    let list = match raw {
        Ok(l) => l,
        Err(_) => return vec![],
    };
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let today_start = (now_secs / 86400) * 86400;

    let mut points: Vec<(u64, f64)> = list
        .into_iter()
        .filter_map(|s| {
            let mut parts = s.splitn(2, ':');
            let ts = parts.next()?.parse::<u64>().ok()?;
            let price = parts.next()?.parse::<f64>().ok()?;
            if ts >= today_start {
                Some((ts, price))
            } else {
                None
            }
        })
        .collect();
    points.sort_by_key(|p| p.0);
    points
}

/// Build Redis connection manager from URL. Returns None if url is empty or connection fails.
pub async fn connect_redis(url: &str) -> Option<redis::aio::ConnectionManager> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    let client = match redis::Client::open(url) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Redis client create failed: {}", e);
            return None;
        }
    };
    match redis::aio::ConnectionManager::new(client).await {
        Ok(m) => {
            tracing::info!("Redis connected for today cache");
            Some(m)
        }
        Err(e) => {
            tracing::warn!("Redis connection failed: {}", e);
            None
        }
    }
}
