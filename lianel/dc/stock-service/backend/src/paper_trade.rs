//! Paper trading loop for daily model signals.
//!
//! Contract:
//! - At decision day `t`, if `publish_signals=true`, we store the model signals for that day.
//! - On the next loop run (typically the next close), we try to execute all stored decisions `t`
//!   using `y(t) = ln(C[t+1]/O[t+1])` computed from IBKR daily bars.
//! - We store execution records (PnL) in Redis and remove the decision from the pending set.

use crate::daily_strategy;
use crate::ibkr::{HistoryBar, IbkrOAuthClient};
use crate::watchlist;
use anyhow::{anyhow, Context, Result};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSignal {
    pub symbol: String,
    pub side: String, // LONG | SHORT
    pub weight: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperModelHealth {
    pub feature_row_count: usize,
    pub nan_or_inf_count: usize,
    pub coef_norm: f64,
    pub train_window_days: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperDecision {
    pub decision_as_of_ts: u64,
    pub created_at_ts: u64,
    pub model: String,
    pub label: String,
    pub quantile: f64,
    pub short_enabled: bool,
    pub publish_signals: bool,
    pub publish_reason: Option<String>,
    pub model_health: Option<PaperModelHealth>,
    pub signals: Vec<PaperSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperLegPnl {
    pub symbol: String,
    pub side: String,
    pub weight: f64,
    pub y_oc_next: f64,
    pub contrib: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperExecution {
    pub decision_as_of_ts: u64,
    pub execution_as_of_ts: u64,
    pub executed_at_ts: u64,
    pub pnl_ln: f64,
    pub pnl_return: f64,
    pub legs: Vec<PaperLegPnl>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperTradeRunResult {
    pub executed_count: usize,
    pub stored_decision: bool,
    pub stored_decision_ts: Option<u64>,
    pub pending_after: usize,
}

const PENDING_SET_KEY: &str = "paper:daily:pending";
const DECISION_KEY_PREFIX: &str = "paper:daily:decision:";
const EXECUTION_KEY_PREFIX: &str = "paper:daily:execution:";
const RECENT_EXECUTIONS_LIST_KEY: &str = "paper:daily:recent_executions";
const LAST_EXECUTION_TS_KEY: &str = "paper:daily:last_execution_ts";
const RECENT_EXECUTIONS_LIMIT: i64 = 20;

fn now_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn decision_key(ts: u64) -> String {
    format!("{}{}", DECISION_KEY_PREFIX, ts)
}

fn execution_key(ts: u64) -> String {
    format!("{}{}", EXECUTION_KEY_PREFIX, ts)
}

fn day_bar_price_ok(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

fn to_paper_decision(resp: &daily_strategy::Phase2ModelSignalsResponse) -> PaperDecision {
    let model_health = resp.model_health.as_ref().map(|h| PaperModelHealth {
        feature_row_count: h.feature_row_count,
        nan_or_inf_count: h.nan_or_inf_count,
        coef_norm: h.coef_norm,
        train_window_days: h.train_window_days,
    });

    PaperDecision {
        decision_as_of_ts: resp.as_of_ts.unwrap_or_default(),
        created_at_ts: now_ts(),
        model: resp.model.to_string(),
        label: resp.label.to_string(),
        quantile: resp.quantile,
        short_enabled: resp.short_enabled,
        publish_signals: resp.publish_signals,
        publish_reason: resp.reason.clone(),
        model_health,
        signals: resp
            .signals
            .iter()
            .map(|s| PaperSignal {
                symbol: s.symbol.clone(),
                side: s.side.to_string(),
                weight: s.weight,
                score: s.score,
            })
            .collect(),
    }
}

fn build_series_map(bars: &[HistoryBar]) -> HashMap<u64, &HistoryBar> {
    bars.iter().map(|b| (b.t, b)).collect()
}

fn find_exec_day_ts(decision_as_of_ts: u64, aligned_ts: &[u64]) -> Option<u64> {
    let i = aligned_ts.iter().position(|&t| t == decision_as_of_ts)?;
    let next_i = i + 1;
    aligned_ts.get(next_i).copied()
}

async fn execute_decision_for_ts(
    client: &IbkrOAuthClient,
    resolved_conids: &HashMap<String, u64>,
    redis: &redis::aio::ConnectionManager,
    decision_ts: u64,
) -> Result<Option<PaperExecution>> {
    let raw: Option<String> = redis.get(decision_key(decision_ts)).await.ok();
    let Some(raw) = raw else {
        return Ok(None);
    };
    let decision: PaperDecision = serde_json::from_str(&raw).context("decode paper decision JSON")?;

    // Empty decisions are allowed but never executed.
    if decision.signals.is_empty() {
        return Ok(None);
    }

    let mut bars_by_symbol: HashMap<String, Vec<HistoryBar>> = HashMap::new();
    let mut symbols_seen: HashSet<String> = HashSet::new();
    for sig in &decision.signals {
        if !symbols_seen.insert(sig.symbol.clone()) {
            continue;
        }
        let n = sig.symbol.trim().to_ascii_uppercase();
        let conid = resolved_conids
            .get(&n)
            .copied()
            .or_else(|| watchlist::get_conid_for_symbol(&n))
            .ok_or_else(|| anyhow!("missing conid for symbol {}", sig.symbol))?;
        let bars = client.fetch_history(conid, "1y", "1d").await?;
        bars_by_symbol.insert(sig.symbol.clone(), daily_strategy::sort_dedupe_bars(bars));
        tokio::time::sleep(std::time::Duration::from_millis(75)).await;
    }

    let aligned_ts = daily_strategy::aligned_timestamps(&bars_by_symbol);
    let Some(exec_ts) = find_exec_day_ts(decision.decision_as_of_ts, &aligned_ts) else {
        // Execution prices not available yet (or intersection changed due to missing bars).
        return Ok(None);
    };

    // Compute per-leg pnl using y_oc_next at `exec_ts`.
    let mut legs: Vec<PaperLegPnl> = Vec::new();
    let mut pnl_ln = 0.0_f64;
    for sig in &decision.signals {
        let Some(bars) = bars_by_symbol.get(&sig.symbol) else {
            return Ok(None);
        };
        let map = build_series_map(bars);
        let Some(next_bar) = map.get(&exec_ts) else {
            return Ok(None);
        };
        if !day_bar_price_ok(next_bar.open) || !day_bar_price_ok(next_bar.close) {
            return Ok(None);
        }
        let y = (next_bar.close / next_bar.open).ln();
        let contrib = match sig.side.as_str() {
            "LONG" => sig.weight * y,
            "SHORT" => -sig.weight * y,
            _ => 0.0,
        };
        pnl_ln += contrib;
        legs.push(PaperLegPnl {
            symbol: sig.symbol.clone(),
            side: sig.side.clone(),
            weight: sig.weight,
            y_oc_next: y,
            contrib,
        });
    }

    let pnl_return = (pnl_ln.exp() - 1.0).max(-1.0); // avoid absurd negatives
    let execution = PaperExecution {
        decision_as_of_ts: decision.decision_as_of_ts,
        execution_as_of_ts: exec_ts,
        executed_at_ts: now_ts(),
        pnl_ln,
        pnl_return,
        legs,
    };
    Ok(Some(execution))
}

pub async fn paper_trade_run(
    client: &IbkrOAuthClient,
    resolved_conids: &HashMap<String, u64>,
    redis: &redis::aio::ConnectionManager,
    quantile: f64,
    short_enabled: bool,
    train_days: usize,
) -> Result<PaperTradeRunResult> {
    let pending: Vec<String> = redis.smembers(PENDING_SET_KEY).await.unwrap_or_default();
    let pending_ts: Vec<u64> = pending
        .iter()
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();

    let mut executed_count = 0usize;
    for decision_ts in pending_ts {
        match execute_decision_for_ts(client, resolved_conids, redis, decision_ts).await? {
            Some(execution) => {
                let exec_key = execution_key(execution.decision_as_of_ts);
                let raw = serde_json::to_string(&execution)?;
                let _: () = redis.set(exec_key, raw).await?;
                let _: () = redis
                    .lpush(RECENT_EXECUTIONS_LIST_KEY, execution.decision_as_of_ts)
                    .await?;
                let _: () = redis
                    .ltrim(
                        RECENT_EXECUTIONS_LIST_KEY,
                        0,
                        RECENT_EXECUTIONS_LIMIT.saturating_sub(1),
                    )
                    .await?;
                let _: () = redis
                    .set(LAST_EXECUTION_TS_KEY, execution.decision_as_of_ts)
                    .await?;
                let _: () = redis.srem(PENDING_SET_KEY, execution.decision_as_of_ts).await?;
                executed_count += 1;
            }
            None => {}
        }
    }

    // Store latest published decision, if we haven't already.
    let pairs: Vec<(String, u64)> = watchlist::DEFAULT_SYMBOLS
        .iter()
        .filter_map(|s| {
            let n = s.trim().to_ascii_uppercase();
            resolved_conids
                .get(&n)
                .copied()
                .or_else(|| watchlist::get_conid_for_symbol(&n))
                .map(|c| (s.to_string(), c))
        })
        .collect();
    let resp = daily_strategy::compute_phase2_model_signals(
        client,
        &pairs,
        quantile,
        short_enabled,
        train_days.max(60),
    )
    .await;

    let mut stored_decision = false;
    let mut stored_decision_ts = None;
    if resp.publish_signals && resp.as_of_ts.is_some() && !resp.signals.is_empty() {
        let ts = resp.as_of_ts.unwrap();
        let key = decision_key(ts);
        let exists: bool = redis.exists(&key).await.unwrap_or(false);
        if !exists {
            let decision = to_paper_decision(&resp);
            let raw = serde_json::to_string(&decision)?;
            let _: () = redis.set(key, raw).await?;
            let _: () = redis.sadd(PENDING_SET_KEY, ts).await?;
            stored_decision = true;
            stored_decision_ts = Some(ts);
        }
    }

    let pending_after: usize = redis
        .scard(PENDING_SET_KEY)
        .await
        .unwrap_or(0usize);

    Ok(PaperTradeRunResult {
        executed_count,
        stored_decision,
        stored_decision_ts,
        pending_after,
    })
}

pub async fn paper_trade_status(
    redis: &redis::aio::ConnectionManager,
) -> Result<serde_json::Value> {
    let last_ts: Option<u64> = redis
        .get(LAST_EXECUTION_TS_KEY)
        .await
        .ok();
    let pending_after: usize = redis.scard(PENDING_SET_KEY).await.unwrap_or(0usize);

    if let Some(ts) = last_ts {
        let raw: Option<String> = redis
            .get(execution_key(ts))
            .await
            .ok();
        if let Some(raw) = raw {
            let exec: PaperExecution = serde_json::from_str(&raw)?;
            return Ok(serde_json::json!({
                "pending_after": pending_after,
                "last_execution": exec,
            }));
        }
    }

    Ok(serde_json::json!({
        "pending_after": pending_after,
        "last_execution": null,
    }))
}

pub async fn paper_trade_records(
    redis: &redis::aio::ConnectionManager,
    limit: isize,
) -> Result<Vec<PaperExecution>> {
    let llimit: i64 = limit.max(1).min(50) as i64;
    let recent: Vec<u64> = redis
        .lrange(RECENT_EXECUTIONS_LIST_KEY, 0, llimit.saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut out = Vec::new();
    for ts in recent {
        let raw: Option<String> = redis.get(execution_key(ts)).await.ok();
        if let Some(raw) = raw {
            if let Ok(exec) = serde_json::from_str::<PaperExecution>(&raw) {
                out.push(exec);
            }
        }
    }
    Ok(out)
}

