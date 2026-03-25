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
    #[serde(default)]
    pub entry_open: Option<f64>,
    #[serde(default)]
    pub exit_close: Option<f64>,
    #[serde(default)]
    pub notional_usd: Option<f64>,
    #[serde(default)]
    pub shares: Option<f64>,
    #[serde(default)]
    pub pnl_usd_gross: Option<f64>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub pnl_usd_net: Option<f64>,
    #[serde(default)]
    pub skipped: bool,
    #[serde(default)]
    pub contrib_gross: f64,
    #[serde(default)]
    pub cost_ln: f64,
    #[serde(default)]
    pub contrib_net: f64,
    pub contrib: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperExecution {
    pub decision_as_of_ts: u64,
    pub execution_as_of_ts: u64,
    pub executed_at_ts: u64,
    #[serde(default)]
    pub pnl_ln_gross: f64,
    #[serde(default)]
    pub cost_total_ln: f64,
    #[serde(default)]
    pub pnl_ln_net: f64,
    pub pnl_ln: f64,
    #[serde(default)]
    pub capital_usd: Option<f64>,
    #[serde(default)]
    pub gross_long: Option<f64>,
    #[serde(default)]
    pub gross_short: Option<f64>,
    #[serde(default)]
    pub pnl_usd_gross: Option<f64>,
    #[serde(default)]
    pub cost_usd_total: Option<f64>,
    #[serde(default)]
    pub pnl_usd_net: Option<f64>,
    #[serde(default)]
    pub pnl_return_gross: f64,
    #[serde(default)]
    pub pnl_return_net: f64,
    pub pnl_return: f64,
    pub legs: Vec<PaperLegPnl>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperTradeRunResult {
    pub executed_count: usize,
    pub stored_decision: bool,
    pub stored_decision_ts: Option<u64>,
    pub latest_model_publish_signals: bool,
    pub latest_model_publish_reason: Option<String>,
    pub latest_model_as_of_ts: Option<u64>,
    pub pending_after: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperTradeBackfillResult {
    pub requested_days: usize,
    pub inserted_count: usize,
    pub skipped_existing_count: usize,
    pub source_rows: usize,
}

const PENDING_SET_KEY: &str = "paper:daily:pending";
const DECISION_KEY_PREFIX: &str = "paper:daily:decision:";
const EXECUTION_KEY_PREFIX: &str = "paper:daily:execution:";
const RECENT_EXECUTIONS_LIST_KEY: &str = "paper:daily:recent_executions";
const LAST_EXECUTION_TS_KEY: &str = "paper:daily:last_execution_ts";
const CUM_PNL_LN_KEY: &str = "paper:daily:cum_pnl_ln";
const CUM_PNL_LN_GROSS_KEY: &str = "paper:daily:cum_pnl_ln_gross";
const CUM_PNL_LN_NET_KEY: &str = "paper:daily:cum_pnl_ln_net";
const EXECUTION_COUNT_KEY: &str = "paper:daily:execution_count";
const RECENT_EXECUTIONS_LIMIT: isize = 20;

const COST_SLIPPAGE_BPS_PER_SIDE_DEFAULT: f64 = 5.0;
const COST_COMMISSION_BPS_PER_SIDE_DEFAULT: f64 = 1.0;
const COST_SHORT_BORROW_BPS_DAILY_DEFAULT: f64 = 2.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperCostAssumptions {
    pub slippage_bps_per_side: f64,
    pub commission_bps_per_side: f64,
    pub short_borrow_bps_daily: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSizingAssumptions {
    pub capital_usd: f64,
    pub gross_long: f64,
    pub gross_short: f64,
    pub min_notional_usd: f64,
    pub share_rounding: String, // none | whole | lot
    pub lot_size: f64,
}

fn read_env_f64(name: &str) -> Option<f64> {
    std::env::var(name)
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn read_env_bool(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|s| {
        let v = s.trim().to_ascii_lowercase();
        v == "1" || v == "true" || v == "yes" || v == "y" || v == "on"
    })
}

fn sizing_assumptions_from_env() -> PaperSizingAssumptions {
    let capital_usd = read_env_f64("PAPER_CAPITAL_USD").unwrap_or(10_000.0).max(0.0);
    let gross_long = read_env_f64("PAPER_GROSS_LONG").unwrap_or(0.5).max(0.0);
    let gross_short = read_env_f64("PAPER_GROSS_SHORT").unwrap_or(0.5).max(0.0);
    let min_notional_usd = read_env_f64("PAPER_MIN_NOTIONAL_USD").unwrap_or(25.0).max(0.0);
    let lot_size = read_env_f64("PAPER_LOT_SIZE").unwrap_or(1.0).max(1.0);
    // Back-compat: PAPER_ROUND_SHARES=true => whole shares
    let legacy_round = read_env_bool("PAPER_ROUND_SHARES").unwrap_or(false);
    let share_rounding = std::env::var("PAPER_SHARE_ROUNDING")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| if legacy_round { "whole".to_string() } else { "none".to_string() });
    let share_rounding = match share_rounding.as_str() {
        "none" | "raw" => "none",
        "whole" | "shares" | "ints" | "int" => "whole",
        "lot" | "lots" => "lot",
        _ => "none",
    }
    .to_string();
    PaperSizingAssumptions {
        capital_usd,
        gross_long,
        gross_short,
        min_notional_usd,
        share_rounding,
        lot_size,
    }
}

fn round_shares(raw: f64, sizing: &PaperSizingAssumptions) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 0.0;
    }
    match sizing.share_rounding.as_str() {
        "whole" => raw.floor().max(0.0),
        "lot" => {
            let lot = sizing.lot_size.max(1.0);
            ((raw / lot).floor() * lot).max(0.0)
        }
        _ => raw,
    }
}

fn cost_assumptions_from_env() -> PaperCostAssumptions {
    PaperCostAssumptions {
        slippage_bps_per_side: read_env_f64("PAPER_COST_SLIPPAGE_BPS_PER_SIDE")
            .unwrap_or(COST_SLIPPAGE_BPS_PER_SIDE_DEFAULT)
            .max(0.0),
        commission_bps_per_side: read_env_f64("PAPER_COST_COMMISSION_BPS_PER_SIDE")
            .unwrap_or(COST_COMMISSION_BPS_PER_SIDE_DEFAULT)
            .max(0.0),
        short_borrow_bps_daily: read_env_f64("PAPER_COST_SHORT_BORROW_BPS_DAILY")
            .unwrap_or(COST_SHORT_BORROW_BPS_DAILY_DEFAULT)
            .max(0.0),
    }
}

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

fn bps_to_ln_cost(bps: f64) -> f64 {
    (bps * 1e-4).max(0.0)
}

fn normalize_legacy_execution(mut e: PaperExecution) -> PaperExecution {
    if e.pnl_ln_gross == 0.0 && e.pnl_ln_net == 0.0 && e.pnl_ln != 0.0 {
        e.pnl_ln_gross = e.pnl_ln;
        e.pnl_ln_net = e.pnl_ln;
        e.pnl_return_gross = e.pnl_return;
        e.pnl_return_net = e.pnl_return;
    }
    if e.pnl_return_gross == 0.0 && e.pnl_return_net == 0.0 && e.pnl_return != 0.0 {
        e.pnl_return_gross = e.pnl_return;
        e.pnl_return_net = e.pnl_return;
    }
    for leg in &mut e.legs {
        if leg.contrib_gross == 0.0 && leg.contrib_net == 0.0 && leg.contrib != 0.0 {
            leg.contrib_gross = leg.contrib;
            leg.contrib_net = leg.contrib;
        }
    }
    e
}

fn rank_and_weight_by_score(
    rows: &[daily_strategy::ResearchRow],
    quantile: f64,
    short_enabled: bool,
) -> Vec<PaperSignal> {
    if rows.is_empty() {
        return vec![];
    }
    let n = rows.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        rows[a]
            .score
            .partial_cmp(&rows[b].score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let k = ((n as f64 * quantile).ceil() as usize).max(1).min(n);
    let long_idx: Vec<usize> = order.iter().rev().take(k).copied().collect();
    let short_idx: Vec<usize> = order.iter().take(k).copied().collect();
    let inv_vol = |i: usize| 1.0 / rows[i].vol20.max(1e-8);
    let long_sum: f64 = long_idx.iter().map(|&i| inv_vol(i)).sum();
    let short_sum: f64 = short_idx.iter().map(|&i| inv_vol(i)).sum();
    let mut out = Vec::new();
    for &i in &long_idx {
        out.push(PaperSignal {
            symbol: rows[i].symbol.clone(),
            side: "LONG".to_string(),
            weight: if long_sum > 1e-12 {
                inv_vol(i) / long_sum
            } else {
                1.0 / long_idx.len() as f64
            },
            score: rows[i].score,
        });
    }
    if short_enabled {
        for &i in &short_idx {
            out.push(PaperSignal {
                symbol: rows[i].symbol.clone(),
                side: "SHORT".to_string(),
                weight: if short_sum > 1e-12 {
                    inv_vol(i) / short_sum
                } else {
                    1.0 / short_idx.len() as f64
                },
                score: rows[i].score,
            });
        }
    }
    out
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
    let mut conn = redis.clone();
    let cost = cost_assumptions_from_env();
    let sizing = sizing_assumptions_from_env();
    let raw: Option<String> = conn.get(decision_key(decision_ts)).await.ok();
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
    let mut pnl_ln_gross = 0.0_f64;
    let mut cost_total_ln = 0.0_f64;
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
        let r_simple = (next_bar.close / next_bar.open) - 1.0;
        let contrib_gross = match sig.side.as_str() {
            "LONG" => sig.weight * y,
            "SHORT" => -sig.weight * y,
            _ => 0.0,
        };
        let mut leg_cost_ln = 2.0
            * (bps_to_ln_cost(cost.slippage_bps_per_side)
                + bps_to_ln_cost(cost.commission_bps_per_side))
            * sig.weight;
        if sig.side == "SHORT" {
            leg_cost_ln += bps_to_ln_cost(cost.short_borrow_bps_daily) * sig.weight;
        }
        let contrib_net = contrib_gross - leg_cost_ln;
        pnl_ln_gross += contrib_gross;
        cost_total_ln += leg_cost_ln;

        // Dollar sizing (approx): allocate within each side using weights.
        let (gross_side, side_sign) = if sig.side == "SHORT" {
            (sizing.gross_short, -1.0)
        } else {
            (sizing.gross_long, 1.0)
        };
        let notional_target = sizing.capital_usd * gross_side * sig.weight;
        let raw_shares = if next_bar.open > 0.0 {
            notional_target / next_bar.open
        } else {
            0.0
        };
        let shares = round_shares(raw_shares, &sizing);
        let notional_used = shares * next_bar.open;
        let skipped = notional_used < sizing.min_notional_usd;
        let pnl_usd_gross = if skipped { 0.0 } else { side_sign * notional_used * r_simple };
        let cost_usd = if skipped { 0.0 } else { notional_used * leg_cost_ln };
        let pnl_usd_net = pnl_usd_gross - cost_usd;
        legs.push(PaperLegPnl {
            symbol: sig.symbol.clone(),
            side: sig.side.clone(),
            weight: sig.weight,
            y_oc_next: y,
            entry_open: Some(next_bar.open),
            exit_close: Some(next_bar.close),
            notional_usd: Some(if skipped { 0.0 } else { notional_used }),
            shares: Some(shares),
            pnl_usd_gross: Some(pnl_usd_gross),
            cost_usd: Some(cost_usd),
            pnl_usd_net: Some(pnl_usd_net),
            skipped,
            contrib_gross,
            cost_ln: leg_cost_ln,
            contrib_net,
            contrib: contrib_net,
        });
    }

    let pnl_ln_net = pnl_ln_gross - cost_total_ln;
    let pnl_return_gross = (pnl_ln_gross.exp() - 1.0).max(-1.0);
    let pnl_return_net = (pnl_ln_net.exp() - 1.0).max(-1.0);
    let pnl_usd_gross = legs.iter().filter_map(|l| l.pnl_usd_gross).sum::<f64>();
    let cost_usd_total = legs.iter().filter_map(|l| l.cost_usd).sum::<f64>();
    let pnl_usd_net = legs.iter().filter_map(|l| l.pnl_usd_net).sum::<f64>();
    let execution = PaperExecution {
        decision_as_of_ts: decision.decision_as_of_ts,
        execution_as_of_ts: exec_ts,
        executed_at_ts: now_ts(),
        pnl_ln_gross,
        cost_total_ln,
        pnl_ln_net,
        pnl_ln: pnl_ln_net,
        capital_usd: Some(sizing.capital_usd),
        gross_long: Some(sizing.gross_long),
        gross_short: Some(sizing.gross_short),
        pnl_usd_gross: Some(pnl_usd_gross),
        cost_usd_total: Some(cost_usd_total),
        pnl_usd_net: Some(pnl_usd_net),
        pnl_return_gross,
        pnl_return_net,
        pnl_return: pnl_return_net,
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
    let mut conn = redis.clone();
    let pending: Vec<String> = conn.smembers(PENDING_SET_KEY).await.unwrap_or_default();
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
                let _: () = conn.set(exec_key, raw).await?;
                let _: () = conn
                    .lpush(RECENT_EXECUTIONS_LIST_KEY, execution.decision_as_of_ts)
                    .await?;
                let _: () = conn
                    .ltrim(
                        RECENT_EXECUTIONS_LIST_KEY,
                        0,
                        RECENT_EXECUTIONS_LIMIT.saturating_sub(1),
                    )
                    .await?;
                let _: () = conn
                    .set(LAST_EXECUTION_TS_KEY, execution.decision_as_of_ts)
                    .await?;
                let prev_cum_legacy: f64 = conn.get(CUM_PNL_LN_KEY).await.unwrap_or(0.0);
                let _: () = conn
                    .set(CUM_PNL_LN_KEY, prev_cum_legacy + execution.pnl_ln_net)
                    .await?;
                let prev_cum_gross: f64 = conn.get(CUM_PNL_LN_GROSS_KEY).await.unwrap_or(0.0);
                let _: () = conn
                    .set(CUM_PNL_LN_GROSS_KEY, prev_cum_gross + execution.pnl_ln_gross)
                    .await?;
                let prev_cum_net: f64 = conn.get(CUM_PNL_LN_NET_KEY).await.unwrap_or(0.0);
                let _: () = conn
                    .set(CUM_PNL_LN_NET_KEY, prev_cum_net + execution.pnl_ln_net)
                    .await?;
                let prev_cnt: u64 = conn.get(EXECUTION_COUNT_KEY).await.unwrap_or(0);
                let _: () = conn.set(EXECUTION_COUNT_KEY, prev_cnt + 1).await?;
                let _: () = conn
                    .srem(PENDING_SET_KEY, execution.decision_as_of_ts)
                    .await?;
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

    let latest_model_publish_signals = resp.publish_signals;
    let latest_model_publish_reason = resp.reason.clone();
    let latest_model_as_of_ts = resp.as_of_ts;

    if resp.publish_signals && resp.as_of_ts.is_some() && !resp.signals.is_empty() {
        let ts = latest_model_as_of_ts.unwrap();
        let key = decision_key(ts);
        let exists: bool = conn.exists(&key).await.unwrap_or(false);
        if !exists {
            let decision = to_paper_decision(&resp);
            let raw = serde_json::to_string(&decision)?;
            let _: () = conn.set(key, raw).await?;
            let _: () = conn.sadd(PENDING_SET_KEY, ts).await?;
            stored_decision = true;
            stored_decision_ts = Some(ts);
        }
    }

    let pending_after: usize = conn.scard(PENDING_SET_KEY).await.unwrap_or(0usize);

    Ok(PaperTradeRunResult {
        executed_count,
        stored_decision,
        stored_decision_ts,
        latest_model_publish_signals,
        latest_model_publish_reason,
        latest_model_as_of_ts,
        pending_after,
    })
}

pub async fn paper_trade_status(
    redis: &redis::aio::ConnectionManager,
) -> Result<serde_json::Value> {
    let mut conn = redis.clone();
    let cost = cost_assumptions_from_env();
    let sizing = sizing_assumptions_from_env();
    let last_ts: Option<u64> = conn
        .get(LAST_EXECUTION_TS_KEY)
        .await
        .ok();
    let pending_after: usize = conn.scard(PENDING_SET_KEY).await.unwrap_or(0usize);
    let cumulative_pnl_ln: f64 = conn.get(CUM_PNL_LN_KEY).await.unwrap_or(0.0);
    let cumulative_pnl_ln_gross: f64 = conn.get(CUM_PNL_LN_GROSS_KEY).await.unwrap_or(0.0);
    let cumulative_pnl_ln_net: f64 = conn.get(CUM_PNL_LN_NET_KEY).await.unwrap_or(cumulative_pnl_ln);
    let execution_count: u64 = conn.get(EXECUTION_COUNT_KEY).await.unwrap_or(0);
    let cumulative_pnl_return = cumulative_pnl_ln.exp() - 1.0;
    let cumulative_pnl_return_gross = cumulative_pnl_ln_gross.exp() - 1.0;
    let cumulative_pnl_return_net = cumulative_pnl_ln_net.exp() - 1.0;

    if let Some(ts) = last_ts {
        let raw: Option<String> = conn.get(execution_key(ts)).await.ok();
        if let Some(raw) = raw {
            let exec: PaperExecution = normalize_legacy_execution(serde_json::from_str(&raw)?);
            return Ok(serde_json::json!({
                "pending_after": pending_after,
                "execution_count": execution_count,
                "cost_assumptions": cost,
                "sizing_assumptions": sizing,
                "cumulative_pnl_ln": cumulative_pnl_ln,
                "cumulative_pnl_return": cumulative_pnl_return,
                "cumulative_pnl_ln_gross": cumulative_pnl_ln_gross,
                "cumulative_pnl_return_gross": cumulative_pnl_return_gross,
                "cumulative_pnl_ln_net": cumulative_pnl_ln_net,
                "cumulative_pnl_return_net": cumulative_pnl_return_net,
                "last_execution": exec,
            }));
        }
    }

    Ok(serde_json::json!({
        "pending_after": pending_after,
        "execution_count": execution_count,
        "cost_assumptions": cost,
        "sizing_assumptions": sizing,
        "cumulative_pnl_ln": cumulative_pnl_ln,
        "cumulative_pnl_return": cumulative_pnl_return,
        "cumulative_pnl_ln_gross": cumulative_pnl_ln_gross,
        "cumulative_pnl_return_gross": cumulative_pnl_return_gross,
        "cumulative_pnl_ln_net": cumulative_pnl_ln_net,
        "cumulative_pnl_return_net": cumulative_pnl_return_net,
        "last_execution": null,
    }))
}

pub async fn paper_trade_records(
    redis: &redis::aio::ConnectionManager,
    limit: isize,
) -> Result<Vec<PaperExecution>> {
    let mut conn = redis.clone();
    let llimit: i64 = limit.max(1).min(50) as i64;
    let recent: Vec<u64> = conn
        .lrange(
            RECENT_EXECUTIONS_LIST_KEY,
            0,
            (llimit as isize).saturating_sub(1),
        )
        .await
        .unwrap_or_default();
    let mut out = Vec::new();
    for ts in recent {
        let raw: Option<String> = conn.get(execution_key(ts)).await.ok();
        if let Some(raw) = raw {
            if let Ok(exec) = serde_json::from_str::<PaperExecution>(&raw) {
                let exec = normalize_legacy_execution(exec);
                out.push(exec);
            }
        }
    }
    Ok(out)
}

pub async fn paper_trade_backfill(
    client: &IbkrOAuthClient,
    resolved_conids: &HashMap<String, u64>,
    redis: &redis::aio::ConnectionManager,
    days: usize,
    quantile: f64,
    short_enabled: bool,
    overwrite: bool,
) -> Result<PaperTradeBackfillResult> {
    let cost = cost_assumptions_from_env();
    let sizing = sizing_assumptions_from_env();
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
    let research = daily_strategy::compute_phase2_research(
        client,
        &pairs,
        quantile,
        short_enabled,
        250,
        60,
        0, // return all rows
    )
    .await;
    let rows = research.rows_sample;
    if rows.is_empty() {
        return Ok(PaperTradeBackfillResult {
            requested_days: days,
            inserted_count: 0,
            skipped_existing_count: 0,
            source_rows: 0,
        });
    }

    let mut by_ts: HashMap<u64, Vec<daily_strategy::ResearchRow>> = HashMap::new();
    for r in rows.clone() {
        by_ts.entry(r.ts).or_default().push(r);
    }
    let mut ts_sorted: Vec<u64> = by_ts.keys().copied().collect();
    ts_sorted.sort_unstable();
    if ts_sorted.is_empty() {
        return Ok(PaperTradeBackfillResult {
            requested_days: days,
            inserted_count: 0,
            skipped_existing_count: 0,
            source_rows: rows.len(),
        });
    }

    let keep = days.max(1).min(ts_sorted.len());
    let selected = ts_sorted[ts_sorted.len() - keep..].to_vec();

    let mut conn = redis.clone();
    if overwrite {
        let _: () = conn.del(RECENT_EXECUTIONS_LIST_KEY).await.unwrap_or(());
        let _: () = conn.del(LAST_EXECUTION_TS_KEY).await.unwrap_or(());
        let _: () = conn.del(CUM_PNL_LN_KEY).await.unwrap_or(());
        let _: () = conn.del(CUM_PNL_LN_GROSS_KEY).await.unwrap_or(());
        let _: () = conn.del(CUM_PNL_LN_NET_KEY).await.unwrap_or(());
        let _: () = conn.del(EXECUTION_COUNT_KEY).await.unwrap_or(());
    }
    let mut inserted_count = 0usize;
    let mut skipped_existing_count = 0usize;
    for (idx, ts) in selected.iter().enumerate() {
        let decision_ts = *ts;
        let exec_ts = if idx + 1 < selected.len() {
            selected[idx + 1]
        } else {
            // last selected day has unknown t+1 in this window; skip
            continue;
        };
        let key = execution_key(decision_ts);
        let exists: bool = conn.exists(&key).await.unwrap_or(false);
        if exists && !overwrite {
            skipped_existing_count += 1;
            continue;
        }
        if exists && overwrite {
            let _: () = conn.del(&key).await.unwrap_or(());
        }
        let Some(day_rows) = by_ts.get(&decision_ts) else {
            continue;
        };
        let sigs = rank_and_weight_by_score(day_rows, quantile, short_enabled);
        if sigs.is_empty() {
            continue;
        }
        let mut legs = Vec::new();
        let mut pnl_ln_gross = 0.0;
        let mut cost_total_ln = 0.0;
        for s in &sigs {
            if let Some(r) = day_rows.iter().find(|r| r.symbol == s.symbol) {
                let contrib_gross = if s.side == "LONG" {
                    s.weight * r.y_oc_next
                } else {
                    -s.weight * r.y_oc_next
                };
                let r_simple = r.y_oc_next.exp() - 1.0;
                let mut leg_cost_ln = 2.0
                    * (bps_to_ln_cost(cost.slippage_bps_per_side)
                        + bps_to_ln_cost(cost.commission_bps_per_side))
                    * s.weight;
                if s.side == "SHORT" {
                    leg_cost_ln += bps_to_ln_cost(cost.short_borrow_bps_daily) * s.weight;
                }
                let contrib_net = contrib_gross - leg_cost_ln;
                pnl_ln_gross += contrib_gross;
                cost_total_ln += leg_cost_ln;

                let (gross_side, side_sign) = if s.side == "SHORT" {
                    (sizing.gross_short, -1.0)
                } else {
                    (sizing.gross_long, 1.0)
                };
                let notional = sizing.capital_usd * gross_side * s.weight;
                let pnl_usd_gross = side_sign * notional * r_simple;
                let cost_usd = notional * leg_cost_ln;
                let pnl_usd_net = pnl_usd_gross - cost_usd;
                legs.push(PaperLegPnl {
                    symbol: s.symbol.clone(),
                    side: s.side.clone(),
                    weight: s.weight,
                    y_oc_next: r.y_oc_next,
                    entry_open: None,
                    exit_close: None,
                    notional_usd: Some(notional),
                    shares: None,
                    pnl_usd_gross: Some(pnl_usd_gross),
                    cost_usd: Some(cost_usd),
                    pnl_usd_net: Some(pnl_usd_net),
                    skipped: notional < sizing.min_notional_usd,
                    contrib_gross,
                    cost_ln: leg_cost_ln,
                    contrib_net,
                    contrib: contrib_net,
                });
            }
        }
        if legs.is_empty() {
            continue;
        }
        let pnl_ln_net = pnl_ln_gross - cost_total_ln;
        let pnl_usd_gross = legs.iter().filter_map(|l| l.pnl_usd_gross).sum::<f64>();
        let cost_usd_total = legs.iter().filter_map(|l| l.cost_usd).sum::<f64>();
        let pnl_usd_net = legs.iter().filter_map(|l| l.pnl_usd_net).sum::<f64>();
        let exec = PaperExecution {
            decision_as_of_ts: decision_ts,
            execution_as_of_ts: exec_ts,
            executed_at_ts: now_ts(),
            pnl_ln_gross,
            cost_total_ln,
            pnl_ln_net,
            pnl_ln: pnl_ln_net,
            capital_usd: Some(sizing.capital_usd),
            gross_long: Some(sizing.gross_long),
            gross_short: Some(sizing.gross_short),
            pnl_usd_gross: Some(pnl_usd_gross),
            cost_usd_total: Some(cost_usd_total),
            pnl_usd_net: Some(pnl_usd_net),
            pnl_return_gross: pnl_ln_gross.exp() - 1.0,
            pnl_return_net: pnl_ln_net.exp() - 1.0,
            pnl_return: pnl_ln_net.exp() - 1.0,
            legs,
        };
        let raw = serde_json::to_string(&exec)?;
        let _: () = conn.set(execution_key(decision_ts), raw).await?;
        let _: () = conn.lpush(RECENT_EXECUTIONS_LIST_KEY, decision_ts).await?;
        let _: () = conn
            .ltrim(
                RECENT_EXECUTIONS_LIST_KEY,
                0,
                RECENT_EXECUTIONS_LIMIT.saturating_sub(1),
            )
            .await?;
        let _: () = conn.set(LAST_EXECUTION_TS_KEY, decision_ts).await?;
        let prev_cum_legacy: f64 = conn.get(CUM_PNL_LN_KEY).await.unwrap_or(0.0);
        let _: () = conn.set(CUM_PNL_LN_KEY, prev_cum_legacy + pnl_ln_net).await?;
        let prev_cum_gross: f64 = conn.get(CUM_PNL_LN_GROSS_KEY).await.unwrap_or(0.0);
        let _: () = conn
            .set(CUM_PNL_LN_GROSS_KEY, prev_cum_gross + pnl_ln_gross)
            .await?;
        let prev_cum_net: f64 = conn.get(CUM_PNL_LN_NET_KEY).await.unwrap_or(0.0);
        let _: () = conn.set(CUM_PNL_LN_NET_KEY, prev_cum_net + pnl_ln_net).await?;
        let prev_cnt: u64 = conn.get(EXECUTION_COUNT_KEY).await.unwrap_or(0);
        let _: () = conn.set(EXECUTION_COUNT_KEY, prev_cnt + 1).await?;
        inserted_count += 1;
    }

    Ok(PaperTradeBackfillResult {
        requested_days: days,
        inserted_count,
        skipped_existing_count,
        source_rows: rows.len(),
    })
}

