use crate::daily_strategy;
use crate::ibkr::IbkrOAuthClient;
use crate::today_cache;
use crate::watchlist::WatchlistQuote;
use redis::aio::ConnectionManager;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct HybridWeights {
    pub daily: f64,
    pub intraday: f64,
    pub under50: f64,
    pub risk: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HybridCandidate {
    pub symbol: String,
    pub price: Option<f64>,
    pub daily_score: f64,
    pub intraday_score: f64,
    pub under50_bonus: f64,
    pub risk_penalty: f64,
    pub hybrid_score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HybridSelectionResponse {
    pub model: &'static str,
    pub model_version: &'static str,
    pub as_of_ts: u64,
    pub model_as_of_ts: Option<u64>,
    pub universe_size: usize,
    pub selected_size: usize,
    pub price_priority_usd: f64,
    pub weights: HybridWeights,
    pub selected: Vec<HybridCandidate>,
    pub top_candidates: Vec<HybridCandidate>,
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn env_f64(name: &str, default_v: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(default_v)
}

fn zscore_map(raw: &HashMap<String, f64>) -> HashMap<String, f64> {
    if raw.is_empty() {
        return HashMap::new();
    }
    let vals: Vec<f64> = raw.values().copied().collect();
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    let var = vals
        .iter()
        .map(|v| {
            let d = *v - mean;
            d * d
        })
        .sum::<f64>()
        / vals.len() as f64;
    let sd = var.sqrt();
    if sd < 1e-12 {
        return raw.keys().map(|k| (k.clone(), 0.0)).collect();
    }
    raw.iter().map(|(k, v)| (k.clone(), (*v - mean) / sd)).collect()
}

fn under50_bonus(price: Option<f64>, threshold: f64) -> f64 {
    let Some(p) = price else {
        return -0.2;
    };
    if p <= 0.0 || !p.is_finite() {
        return -0.2;
    }
    if p <= threshold {
        return 1.0;
    }
    // Soft decay: keep some score up to 3x threshold, then 0.
    let max_p = threshold * 3.0;
    if p >= max_p {
        return 0.0;
    }
    ((max_p - p) / (max_p - threshold)).clamp(0.0, 1.0)
}

pub async fn build_hybrid_selection(
    ibkr_client: &IbkrOAuthClient,
    redis: Option<&ConnectionManager>,
    symbol_conids: &[(String, u64)],
    quotes: &HashMap<String, WatchlistQuote>,
    quantile: f64,
    short_enabled: bool,
    train_days: usize,
    top_n: usize,
) -> HybridSelectionResponse {
    let model = daily_strategy::compute_phase2_model_signals(
        ibkr_client,
        symbol_conids,
        quantile.clamp(0.05, 0.45),
        short_enabled,
        train_days.max(60),
    )
    .await;

    let threshold = env_f64("STOCK_PRICE_PRIORITY_THRESHOLD_USD", 50.0).max(1.0);
    let weights = HybridWeights {
        daily: env_f64("STOCK_HYBRID_WEIGHT_DAILY", 0.60),
        intraday: env_f64("STOCK_HYBRID_WEIGHT_INTRADAY", 0.25),
        under50: env_f64("STOCK_HYBRID_WEIGHT_UNDER50", 0.20),
        risk: env_f64("STOCK_HYBRID_WEIGHT_RISK", 0.15).abs(),
    };

    let mut daily_raw: HashMap<String, f64> = HashMap::new();
    let mut risk_raw: HashMap<String, f64> = HashMap::new();
    for f in &model.features {
        daily_raw.insert(f.symbol.clone(), f.score);
        risk_raw.insert(f.symbol.clone(), f.vol20.abs());
    }
    let daily_z = zscore_map(&daily_raw);
    let risk_z = zscore_map(&risk_raw);

    let mut intraday_raw: HashMap<String, f64> = HashMap::new();
    for (sym, _) in symbol_conids {
        let mut ret = 0.0_f64;
        if let Some(conn) = redis {
            let pts = today_cache::get_today(Some(conn), sym).await;
            if pts.len() >= 2 {
                let first = pts.first().map(|p| p.1).unwrap_or(0.0);
                let last = pts.last().map(|p| p.1).unwrap_or(0.0);
                if first > 0.0 && last > 0.0 {
                    ret = (last / first).ln();
                }
            }
        }
        intraday_raw.insert(sym.clone(), ret);
    }
    let intraday_z = zscore_map(&intraday_raw);

    let mut candidates: Vec<HybridCandidate> = symbol_conids
        .iter()
        .map(|(sym, _)| {
            let price = quotes.get(sym).and_then(|q| q.price);
            let d = daily_z.get(sym).copied().unwrap_or(0.0);
            let i = intraday_z.get(sym).copied().unwrap_or(0.0);
            let r = risk_z.get(sym).copied().unwrap_or(0.0).max(0.0);
            let p = under50_bonus(price, threshold);
            let hybrid = (weights.daily * d) + (weights.intraday * i) + (weights.under50 * p)
                - (weights.risk * r);

            let mut reasons = Vec::new();
            if d > 0.5 {
                reasons.push("strong daily alpha".to_string());
            }
            if i > 0.5 {
                reasons.push("positive intraday overlay".to_string());
            }
            if p > 0.8 {
                reasons.push(format!("price-priority (<= ${threshold:.0})"));
            }
            if reasons.is_empty() {
                reasons.push("balanced score contribution".to_string());
            }

            HybridCandidate {
                symbol: sym.clone(),
                price,
                daily_score: d,
                intraday_score: i,
                under50_bonus: p,
                risk_penalty: r,
                hybrid_score: hybrid,
                reasons,
            }
        })
        .collect();

    candidates.sort_by(|a, b| {
        b.hybrid_score
            .partial_cmp(&a.hybrid_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let selected_n = top_n.max(3).min(candidates.len().max(3));
    let selected = candidates.iter().take(selected_n).cloned().collect::<Vec<_>>();
    let top_candidates = candidates.iter().take(30).cloned().collect::<Vec<_>>();

    HybridSelectionResponse {
        model: "hybrid_daily_intraday_priority",
        model_version: "v1",
        as_of_ts: now_ts(),
        model_as_of_ts: model.as_of_ts,
        universe_size: symbol_conids.len(),
        selected_size: selected.len(),
        price_priority_usd: threshold,
        weights,
        selected,
        top_candidates,
    }
}

