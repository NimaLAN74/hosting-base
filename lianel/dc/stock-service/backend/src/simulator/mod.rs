use crate::app::AppState;
use crate::daily_strategy;
use crate::hybrid_selector;
use crate::watchlist;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod exchanges;

const RUN_INDEX_KEY: &str = "sim:runs:index";

#[derive(Debug, Clone, Deserialize)]
pub struct SimRunRequest {
    #[serde(default = "default_days")]
    pub days: usize,
    #[serde(default = "default_top")]
    pub top: usize,
    #[serde(default = "default_quantile")]
    pub quantile: f64,
    #[serde(default = "default_short_enabled")]
    pub short_enabled: bool,
    #[serde(default = "default_initial_capital")]
    pub initial_capital_usd: f64,
    #[serde(default = "default_reinvest")]
    pub reinvest_profit: bool,
    #[serde(default = "default_replay_delay_ms")]
    pub replay_delay_ms: u64,
    #[serde(default = "default_readiness_min_days")]
    pub readiness_min_days: usize,
    #[serde(default = "default_max_cycles")]
    pub max_cycles: usize,
    #[serde(default = "default_live_market_data")]
    pub live_market_data: bool,
    #[serde(default = "default_edge_cost_buffer_bps")]
    pub edge_cost_buffer_bps: f64,
    #[serde(default = "default_min_signal_abs_return_bps")]
    pub min_signal_abs_return_bps: f64,
    #[serde(default = "default_min_hold_seconds")]
    pub min_hold_seconds: u64,
    #[serde(default = "default_symbol_cooldown_seconds")]
    pub symbol_cooldown_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimRunMeta {
    pub run_id: String,
    pub status: String,
    pub created_at_ts: u64,
    pub started_at_ts: Option<u64>,
    pub finished_at_ts: Option<u64>,
    pub days_requested: usize,
    pub symbols_count: usize,
    pub exchanges: Vec<String>,
    pub initial_capital_usd: f64,
    pub ending_equity_usd: Option<f64>,
    pub pnl_usd: Option<f64>,
    #[serde(default)]
    pub cycles_completed: usize,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub readiness_score: Option<f64>,
    #[serde(default)]
    pub readiness_passed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimEvent {
    pub event_id: String,
    pub run_id: String,
    pub ts: u64,
    pub kind: String,
    pub exchange: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimDecisionTrace {
    pub decision_id: String,
    pub run_id: String,
    pub decision_ts: u64,
    pub symbol: String,
    pub exchange: String,
    pub side: String,
    pub weight: f64,
    pub hybrid_score: f64,
    pub features: serde_json::Value,
    pub rationale: Vec<String>,
    #[serde(default)]
    pub short_explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimFillLedgerRow {
    pub decision_id: String,
    pub run_id: String,
    pub exec_ts: u64,
    pub symbol: String,
    pub exchange: String,
    pub side: String,
    pub qty_notional_usd: f64,
    pub open_px: f64,
    pub close_px: f64,
    #[serde(default)]
    pub buy_px: f64,
    #[serde(default)]
    pub sell_px: f64,
    #[serde(default)]
    pub buy_ts: u64,
    #[serde(default)]
    pub sell_ts: u64,
    #[serde(default)]
    pub buy_session_time_utc: String,
    #[serde(default)]
    pub sell_session_time_utc: String,
    #[serde(default)]
    pub market_data_source: String,
    pub ret_simple: f64,
    pub fee_usd: f64,
    #[serde(default)]
    pub ibkr_commission_usd: f64,
    #[serde(default)]
    pub exchange_fee_usd: f64,
    #[serde(default)]
    pub clearing_fee_usd: f64,
    #[serde(default)]
    pub regulatory_fee_usd: f64,
    #[serde(default)]
    pub fx_fee_usd: f64,
    #[serde(default)]
    pub tax_usd: f64,
    pub slippage_usd: f64,
    pub pnl_usd: f64,
    pub latency_ms: u64,
    #[serde(default)]
    pub order_id: Option<String>,
    #[serde(default = "default_fill_ratio")]
    pub fill_ratio: f64,
    #[serde(default)]
    pub borrow_fee_usd: f64,
    #[serde(default)]
    pub market_impact_usd: f64,
    #[serde(default)]
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimOrderLedgerRow {
    pub order_id: String,
    pub decision_id: String,
    pub run_id: String,
    pub ts: u64,
    #[serde(default)]
    pub wall_clock_ts: u64,
    pub symbol: String,
    pub exchange: String,
    pub side: String,
    pub order_type: String,
    pub tif: String,
    pub qty_notional_usd: f64,
    pub intended_px: f64,
    pub status: String,
    pub filled_notional_usd: f64,
    pub remaining_notional_usd: f64,
    pub venue_latency_ms: u64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimPortfolioPoint {
    pub ts: u64,
    pub equity_usd: f64,
    pub pnl_cum_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimRiskSnapshot {
    pub run_id: String,
    pub ts: u64,
    pub equity_usd: f64,
    pub cash_usd: f64,
    pub gross_exposure_usd: f64,
    pub leverage: f64,
    pub drawdown: f64,
    pub var_95_1d: f64,
    pub concentration_hhi: f64,
    pub benchmark_equity_usd: f64,
    pub relative_return: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimReadinessReport {
    pub run_id: String,
    pub status: String,
    pub min_days_required: usize,
    pub evaluated_days: usize,
    pub score: f64,
    pub pass: bool,
    pub criteria: serde_json::Value,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimBiasFinding {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimExplainResponse {
    pub run: SimRunMeta,
    pub decision: SimDecisionTrace,
    pub fills: Vec<SimFillLedgerRow>,
    pub market_context: serde_json::Value,
}

fn default_days() -> usize {
    7
}
fn default_top() -> usize {
    16
}
fn default_quantile() -> f64 {
    0.2
}
fn default_short_enabled() -> bool {
    true
}
fn default_initial_capital() -> f64 {
    100.0
}
fn default_reinvest() -> bool {
    true
}
fn default_replay_delay_ms() -> u64 {
    250
}
fn default_fill_ratio() -> f64 {
    1.0
}
fn default_readiness_min_days() -> usize {
    126
}
fn default_max_cycles() -> usize {
    2_000
}
fn default_live_market_data() -> bool {
    true
}
fn default_edge_cost_buffer_bps() -> f64 {
    6.0
}
fn default_min_signal_abs_return_bps() -> f64 {
    8.0
}
fn default_min_hold_seconds() -> u64 {
    300
}
fn default_symbol_cooldown_seconds() -> u64 {
    180
}

fn hhmm_utc_from_ts(ts: u64) -> String {
    use chrono::{TimeZone, Timelike, Utc};
    let dt = Utc
        .timestamp_opt(ts as i64, 0)
        .single()
        .unwrap_or_else(Utc::now);
    format!("{:02}:{:02}", dt.hour(), dt.minute())
}

fn parse_hhmm_utc(v: &str) -> Option<(u32, u32)> {
    let mut parts = v.split(':');
    let h = parts.next()?.trim().parse::<u32>().ok()?;
    let m = parts.next()?.trim().parse::<u32>().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

fn is_exchange_open_now(exchange: &exchanges::ExchangeAdapter, now_ts: u64) -> bool {
    use chrono::{Datelike, TimeZone, Timelike, Utc, Weekday};
    let dt = Utc
        .timestamp_opt(now_ts as i64, 0)
        .single()
        .unwrap_or_else(Utc::now);
    match dt.weekday() {
        Weekday::Sat | Weekday::Sun => return false,
        _ => {}
    }
    let (oh, om) = match parse_hhmm_utc(exchange.session_open_utc) {
        Some(v) => v,
        None => return true,
    };
    let (ch, cm) = match parse_hhmm_utc(exchange.session_close_utc) {
        Some(v) => v,
        None => return true,
    };
    let now_mins = dt.hour() * 60 + dt.minute();
    let open_mins = oh * 60 + om;
    let close_mins = ch * 60 + cm;
    now_mins >= open_mins && now_mins <= close_mins
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn generate_run_id() -> String {
    let t = now_ts();
    let r = rand::random::<u32>() % 1_000_000;
    format!("sim-{t}-{r:06}")
}

fn run_meta_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:meta")
}
fn run_events_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:events")
}
fn run_decisions_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:decision_trace")
}
fn run_fills_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:fills")
}
fn run_curve_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:portfolio_curve")
}
fn run_bias_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:bias_findings")
}
fn run_orders_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:orders")
}
fn run_risk_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:risk_snapshots")
}
fn run_readiness_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:readiness")
}
fn run_control_key(run_id: &str) -> String {
    format!("sim:run:{run_id}:control")
}

async fn append_json<T: Serialize>(
    redis: &redis::aio::ConnectionManager,
    key: &str,
    value: &T,
) -> Result<(), String> {
    let raw = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let mut conn = redis.clone();
    let _: () = conn.rpush(key, raw).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn push_event(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
    kind: &str,
    exchange: Option<String>,
    payload: serde_json::Value,
) -> Result<(), String> {
    let e = SimEvent {
        event_id: format!("ev-{}-{}", now_ts(), rand::random::<u32>()),
        run_id: run_id.to_string(),
        ts: now_ts(),
        kind: kind.to_string(),
        exchange,
        payload,
    };
    append_json(redis, &run_events_key(run_id), &e).await
}

async fn set_meta(redis: &redis::aio::ConnectionManager, meta: &SimRunMeta) -> Result<(), String> {
    let mut conn = redis.clone();
    let raw = serde_json::to_string(meta).map_err(|e| e.to_string())?;
    let _: () = conn
        .set(run_meta_key(&meta.run_id), raw)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_run_meta(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
) -> Result<Option<SimRunMeta>, String> {
    let mut conn = redis.clone();
    let raw: Option<String> = conn.get(run_meta_key(run_id)).await.map_err(|e| e.to_string())?;
    match raw {
        Some(v) => serde_json::from_str::<SimRunMeta>(&v)
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

pub async fn list_runs(
    redis: &redis::aio::ConnectionManager,
    limit: usize,
) -> Result<Vec<SimRunMeta>, String> {
    let mut conn = redis.clone();
    let ids: Vec<String> = conn
        .lrange(RUN_INDEX_KEY, 0, (limit.max(1) as isize) - 1)
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for id in ids {
        if let Some(meta) = get_run_meta(redis, &id).await? {
            out.push(meta);
        }
    }
    Ok(out)
}

pub async fn get_timeline(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
    limit: usize,
) -> Result<Vec<SimEvent>, String> {
    let mut conn = redis.clone();
    let n = limit.max(1) as isize;
    let rows: Vec<String> = conn
        .lrange(run_events_key(run_id), -n, -1)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .filter_map(|r| serde_json::from_str::<SimEvent>(&r).ok())
        .collect())
}

pub async fn get_by_exchange(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
    exchange: &str,
    limit: usize,
) -> Result<Vec<SimEvent>, String> {
    let ex = exchange.trim().to_ascii_uppercase();
    let timeline = get_timeline(redis, run_id, limit.max(1_000)).await?;
    Ok(timeline
        .into_iter()
        .filter(|e| e.exchange.as_deref().map(|x| x == ex).unwrap_or(false))
        .take(limit.max(1))
        .collect())
}

pub async fn get_bias_report(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
) -> Result<Vec<SimBiasFinding>, String> {
    let mut conn = redis.clone();
    let rows: Vec<String> = conn
        .lrange(run_bias_key(run_id), 0, 500)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .filter_map(|r| serde_json::from_str::<SimBiasFinding>(&r).ok())
        .collect())
}

pub async fn get_order_ledger(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
    limit: usize,
) -> Result<Vec<SimOrderLedgerRow>, String> {
    let mut conn = redis.clone();
    let n = limit.max(1) as isize;
    let rows: Vec<String> = conn
        .lrange(run_orders_key(run_id), -n, -1)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .filter_map(|r| serde_json::from_str::<SimOrderLedgerRow>(&r).ok())
        .collect())
}

pub async fn get_risk_snapshots(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
    limit: usize,
) -> Result<Vec<SimRiskSnapshot>, String> {
    let mut conn = redis.clone();
    let n = limit.max(1) as isize;
    let rows: Vec<String> = conn
        .lrange(run_risk_key(run_id), -n, -1)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .filter_map(|r| serde_json::from_str::<SimRiskSnapshot>(&r).ok())
        .collect())
}

pub async fn get_readiness_report(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
) -> Result<Option<SimReadinessReport>, String> {
    let mut conn = redis.clone();
    let raw: Option<String> = conn.get(run_readiness_key(run_id)).await.map_err(|e| e.to_string())?;
    match raw {
        Some(v) => serde_json::from_str::<SimReadinessReport>(&v)
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

pub async fn set_control_action(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
    action: &str,
) -> Result<(), String> {
    let mut conn = redis.clone();
    let _: () = conn
        .set(run_control_key(run_id), action.to_ascii_lowercase())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn explain_decision(
    redis: &redis::aio::ConnectionManager,
    run_id: &str,
    decision_id: &str,
) -> Result<Option<SimExplainResponse>, String> {
    let run = match get_run_meta(redis, run_id).await? {
        Some(v) => v,
        None => return Ok(None),
    };

    let mut conn = redis.clone();
    let decision_rows: Vec<String> = conn
        .lrange(run_decisions_key(run_id), 0, 10_000)
        .await
        .map_err(|e| e.to_string())?;
    let fill_rows: Vec<String> = conn
        .lrange(run_fills_key(run_id), 0, 10_000)
        .await
        .map_err(|e| e.to_string())?;

    let decision = decision_rows
        .into_iter()
        .filter_map(|r| serde_json::from_str::<SimDecisionTrace>(&r).ok())
        .find(|d| d.decision_id == decision_id);

    let Some(decision) = decision else {
        return Ok(None);
    };
    let fills: Vec<SimFillLedgerRow> = fill_rows
        .into_iter()
        .filter_map(|r| serde_json::from_str::<SimFillLedgerRow>(&r).ok())
        .filter(|f| f.decision_id == decision_id)
        .collect();

    let market_context = json!({
        "exchange": decision.exchange,
        "feature_inputs": decision.features,
        "rationale": decision.rationale,
    });

    Ok(Some(SimExplainResponse {
        run,
        decision,
        fills,
        market_context,
    }))
}

fn compute_hhi(weights: &[f64]) -> f64 {
    weights.iter().map(|w| w * w).sum::<f64>()
}

fn compute_readiness(
    run_id: &str,
    min_days_required: usize,
    risk_series: &[SimRiskSnapshot],
    daily_pnls: &[f64],
    data_gaps: usize,
    explain_coverage: f64,
) -> SimReadinessReport {
    let evaluated_days = daily_pnls.len();
    let avg_daily_pnl = if daily_pnls.is_empty() {
        0.0
    } else {
        daily_pnls.iter().sum::<f64>() / daily_pnls.len() as f64
    };
    let max_drawdown = risk_series
        .iter()
        .map(|r| r.drawdown)
        .fold(0.0_f64, f64::max);
    let max_leverage = risk_series
        .iter()
        .map(|r| r.leverage)
        .fold(0.0_f64, f64::max);
    let max_var = risk_series
        .iter()
        .map(|r| r.var_95_1d)
        .fold(0.0_f64, f64::max);
    let avg_relative = if risk_series.is_empty() {
        0.0
    } else {
        risk_series.iter().map(|r| r.relative_return).sum::<f64>() / risk_series.len() as f64
    };
    let gate_days = evaluated_days >= min_days_required;
    let gate_drawdown = max_drawdown <= 0.25;
    let gate_var = max_var <= 0.06;
    let gate_leverage = max_leverage <= 1.8;
    let gate_perf = avg_daily_pnl > 0.0 && avg_relative > 0.0;
    let gap_ratio = if evaluated_days == 0 {
        1.0
    } else {
        data_gaps as f64 / evaluated_days as f64
    };
    let gate_data = gap_ratio <= 0.10;
    let gate_explain = explain_coverage >= 0.98;
    let gates = [gate_days, gate_drawdown, gate_var, gate_leverage, gate_perf, gate_data, gate_explain];
    let score = gates.iter().filter(|g| **g).count() as f64 / gates.len() as f64;
    let pass = gates.iter().all(|g| *g);
    let recommendation = if pass {
        "READY_FOR_REAL_MONEY"
    } else if gate_perf && gate_data && gate_explain {
        "PROMISING_BUT_RISK_TUNING_REQUIRED"
    } else {
        "NOT_READY_CONTINUE_SIMULATION"
    };
    SimReadinessReport {
        run_id: run_id.to_string(),
        status: if pass { "pass" } else { "fail" }.to_string(),
        min_days_required,
        evaluated_days,
        score,
        pass,
        criteria: json!({
            "gate_days": gate_days,
            "gate_drawdown": gate_drawdown,
            "gate_var_95_1d": gate_var,
            "gate_leverage": gate_leverage,
            "gate_performance_vs_benchmark": gate_perf,
            "gate_data_quality": gate_data,
            "gate_explainability": gate_explain,
            "avg_daily_pnl": avg_daily_pnl,
            "avg_relative_return": avg_relative,
            "max_drawdown": max_drawdown,
            "max_var_95_1d": max_var,
            "max_leverage": max_leverage,
            "data_gap_ratio": gap_ratio,
            "explainability_coverage": explain_coverage
        }),
        recommendation: recommendation.to_string(),
    }
}

pub async fn start_run(state: AppState, mut req: SimRunRequest) -> Result<SimRunMeta, String> {
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| "Redis not configured (simulator requires redis)".to_string())?
        .clone();
    let client = state
        .ibkr_client
        .as_ref()
        .ok_or_else(|| "IBKR not configured".to_string())?
        .clone();

    // Allow short replay windows when aligned market data is sparse.
    // Continuous six-month evidence still comes from readiness_min_days/max_cycles.
    req.days = req.days.max(7).min(365);
    req.top = req.top.max(6).min(40);
    req.quantile = req.quantile.clamp(0.05, 0.45);
    req.initial_capital_usd = req.initial_capital_usd.max(25.0);
    // User requirement: never evaluate readiness below six months.
    req.readiness_min_days = req.readiness_min_days.max(126).min(365 * 3);
    req.max_cycles = req
        .max_cycles
        .max(req.readiness_min_days)
        .min(if req.live_market_data { 1_000_000 } else { 20_000 });
    if req.live_market_data {
        // In live mode, one cycle should represent a real-time step, not a tight loop.
        req.replay_delay_ms = req.replay_delay_ms.max(60_000);
    }
    req.edge_cost_buffer_bps = req.edge_cost_buffer_bps.clamp(0.0, 150.0);
    req.min_signal_abs_return_bps = req.min_signal_abs_return_bps.clamp(0.0, 200.0);
    req.min_hold_seconds = req.min_hold_seconds.clamp(0, 86_400);
    req.symbol_cooldown_seconds = req.symbol_cooldown_seconds.clamp(0, 86_400);

    let run_id = generate_run_id();
    let created_at = now_ts();

    let (pairs, quotes) = {
        let g = state.watchlist_cache.read().await;
        let pairs = watchlist::active_symbol_conid_pairs(&g);
        (pairs, g.quotes.clone())
    };
    if pairs.len() < 6 {
        return Err("need at least 6 symbols with conids to run simulator".to_string());
    }

    let selection = hybrid_selector::build_hybrid_selection(
        &client,
        Some(&redis),
        &pairs,
        &quotes,
        req.quantile,
        req.short_enabled,
        120,
        req.top,
    )
    .await;

    let selected_symbols: Vec<String> = selection
        .selected
        .iter()
        .map(|c| c.symbol.clone())
        .collect();
    if selected_symbols.len() < 4 {
        return Err("selection produced too few symbols".to_string());
    }

    let mut bars_by_symbol: HashMap<String, Vec<crate::ibkr::HistoryBar>> = HashMap::new();
    let mut replay_symbols: Vec<String> = selected_symbols.clone();
    let mut replay_ts: Vec<u64> = Vec::new();
    let effective_days: usize;
    if req.live_market_data {
        effective_days = req.days;
    } else {
        let mut history_fetch_failures = 0usize;
        for (sym, conid) in &pairs {
            if !selected_symbols.iter().any(|s| s == sym) {
                continue;
            }
            let mut bars = None;
            for attempt in 1..=3 {
                match client.fetch_history(*conid, "1y", "1d").await {
                    Ok(result) => {
                        bars = Some(result);
                        break;
                    }
                    Err(e) => {
                        if attempt == 3 {
                            history_fetch_failures += 1;
                            tracing::warn!(
                                "simulator: history fetch failed for {} (conid={}): {}",
                                sym,
                                conid,
                                e
                            );
                        } else {
                            tokio::time::sleep(std::time::Duration::from_millis(250 * attempt as u64))
                                .await;
                        }
                    }
                }
            }
            let Some(bars) = bars else {
                continue;
            };
            let bars = daily_strategy::sort_dedupe_bars(bars);
            if bars.len() >= req.days + 3 {
                bars_by_symbol.insert(sym.clone(), bars);
            }
            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        }
        if bars_by_symbol.len() < 4 {
            if bars_by_symbol.is_empty() && history_fetch_failures > 0 {
                return Err(format!(
                    "history failed for all selected symbols ({})",
                    history_fetch_failures
                ));
            }
            return Err("not enough symbols with stable daily history for replay".to_string());
        }
        let aligned_ts = daily_strategy::aligned_timestamps(&bars_by_symbol);
        let replay_base_ts: Vec<u64> = if aligned_ts.len() >= 3 {
            aligned_ts
        } else {
            // Fallback: when strict all-symbol overlap is too short, replay on the densest symbol's
            // timeline and allow per-symbol bar misses during scoring/execution.
            let maybe_ref = bars_by_symbol
                .values()
                .max_by_key(|bars| bars.len())
                .map(|bars| {
                    let mut ts = bars.iter().map(|b| b.t).collect::<Vec<_>>();
                    ts.sort_unstable();
                    ts.dedup();
                    ts
                });
            match maybe_ref {
                Some(ts) if ts.len() >= 3 => ts,
                _ => return Err("not enough aligned days to run requested simulation horizon".to_string()),
            }
        };
        // Degrade gracefully when overlap is limited instead of hard-failing run startup.
        let d = req.days.min(replay_base_ts.len().saturating_sub(1)).max(2);
        replay_ts = replay_base_ts[replay_base_ts.len() - (d + 1)..].to_vec();
        effective_days = d;
        replay_symbols = bars_by_symbol.keys().cloned().collect();
    }

    let mut exch_set = std::collections::BTreeSet::<String>::new();
    for sym in &replay_symbols {
        exch_set.insert(exchanges::infer_exchange(sym).code.to_string());
    }
    let exchanges = exch_set.into_iter().collect::<Vec<_>>();

    let mut meta = SimRunMeta {
        run_id: run_id.clone(),
        status: "queued".to_string(),
        created_at_ts: created_at,
        started_at_ts: None,
        finished_at_ts: None,
        days_requested: effective_days,
        symbols_count: replay_symbols.len(),
        exchanges,
        initial_capital_usd: req.initial_capital_usd,
        ending_equity_usd: None,
        pnl_usd: None,
        cycles_completed: 0,
        stop_reason: None,
        readiness_score: None,
        readiness_passed: None,
    };
    set_meta(&redis, &meta).await?;
    {
        let mut conn = redis.clone();
        let _: () = conn
            .lpush(RUN_INDEX_KEY, &run_id)
            .await
            .map_err(|e| e.to_string())?;
        let _: () = conn
            .ltrim(RUN_INDEX_KEY, 0, 199)
            .await
            .map_err(|e| e.to_string())?;
    }

    let selected_scores: HashMap<String, (f64, Vec<String>)> = selection
        .selected
        .iter()
        .map(|c| (c.symbol.clone(), (c.hybrid_score, c.reasons.clone())))
        .collect();
    let watchlist_cache = state.watchlist_cache.clone();

    let run_id_cloned = run_id.clone();
    let meta_for_task = meta.clone();
    tokio::spawn(async move {
        let mut meta = meta_for_task;
        let mut equity = req.initial_capital_usd;
        let mut benchmark_equity = req.initial_capital_usd;
        let mut cash_usd = req.initial_capital_usd;
        let mut peak_equity = req.initial_capital_usd.max(1.0);
        let mut findings: Vec<SimBiasFinding> = Vec::new();
        let mut exchange_leg_count: HashMap<String, usize> = HashMap::new();
        let mut missing_price_rows = 0usize;
        let mut total_rows = 0usize;
        let mut daily_pnls: Vec<f64> = Vec::new();
        let mut risk_snapshots: Vec<SimRiskSnapshot> = Vec::new();
        let mut total_decisions = 0usize;
        let mut explained_decisions = 0usize;
        let mut live_prev_prices: HashMap<String, f64> = HashMap::new();
        let mut live_last_decision_ts = now_ts().saturating_sub(1);
        let mut last_fill_ts_by_symbol: HashMap<String, u64> = HashMap::new();
        let mut cooldown_until_by_symbol: HashMap<String, u64> = HashMap::new();

        meta.status = "running".to_string();
        meta.started_at_ts = Some(now_ts());
        let _ = set_meta(&redis, &meta).await;
        let _ = push_event(
            &redis,
            &run_id_cloned,
            "RunStarted",
            None,
            json!({
                "days_requested": req.days,
                "days_effective": effective_days,
                "symbols": replay_symbols.len(),
                "initial_capital_usd": req.initial_capital_usd,
                "live_market_data": req.live_market_data
            }),
        )
        .await;

        let mut day_idx = 1usize;
        let mut cycle_idx = 0usize;
        let mut stop_reason = "MAX_CYCLES_REACHED".to_string();
        while cycle_idx < req.max_cycles {
            let control: Option<String> = {
                let mut conn = redis.clone();
                conn.get(run_control_key(&run_id_cloned)).await.ok()
            };
            if let Some(action) = control {
                if action == "stop" {
                    stop_reason = "MANUAL_STOP".to_string();
                    break;
                }
                if action == "pause" {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
            }
            let (exec_ts, decision_ts) = if req.live_market_data {
                let exec = now_ts();
                let decision = live_last_decision_ts;
                live_last_decision_ts = exec;
                (exec, decision)
            } else {
                if replay_ts.len() < 2 {
                    stop_reason = "NO_REPLAY_WINDOW".to_string();
                    break;
                }
                if day_idx >= replay_ts.len() {
                    day_idx = 1;
                }
                let exec = replay_ts[day_idx];
                let decision = replay_ts[day_idx - 1];
                (exec, decision)
            };

            let mut decision_rows: Vec<SimDecisionTrace> = Vec::new();
            let mut fills: Vec<SimFillLedgerRow> = Vec::new();
            let mut orders: Vec<SimOrderLedgerRow> = Vec::new();
            let symbols: Vec<String> = replay_symbols.clone();
            let symbol_count = symbols.len();
            let quote_snapshot = if req.live_market_data {
                let g = watchlist_cache.read().await;
                Some(g.quotes.clone())
            } else {
                None
            };
            let per_leg_notional = if symbol_count == 0 {
                0.0
            } else {
                equity.max(0.0) / symbol_count as f64
            };

            let _ = push_event(
                &redis,
                &run_id_cloned,
                "MarketSnapshotSeen",
                None,
                json!({"decision_ts": decision_ts, "exec_ts": exec_ts, "symbol_count": symbol_count}),
            )
            .await;

            for sym in symbols {
                let exchange = exchanges::infer_exchange(&sym);
                let (open_px, close_px, ret_prev, is_halt, market_data_source) = if req.live_market_data {
                    if !is_exchange_open_now(&exchange, exec_ts) {
                        continue;
                    }
                    let quote = quote_snapshot
                        .as_ref()
                        .and_then(|q| q.get(&sym))
                        .and_then(|q| q.price)
                        .filter(|p| p.is_finite() && *p > 0.0);
                    let Some(px_now) = quote else {
                        missing_price_rows += 1;
                        total_rows += 1;
                        continue;
                    };
                    let px_prev = live_prev_prices.get(&sym).copied().unwrap_or(px_now);
                    live_prev_prices.insert(sym.clone(), px_now);
                    let ret_prev = if px_prev > 0.0 { (px_now / px_prev).ln() } else { 0.0 };
                    let is_halt = (px_now - px_prev).abs() <= f64::EPSILON;
                    (px_prev, px_now, ret_prev, is_halt, "IBKR_LIVE_WATCHLIST_QUOTE".to_string())
                } else {
                    let Some(bars) = bars_by_symbol.get(&sym) else {
                        continue;
                    };
                    let map: HashMap<u64, &crate::ibkr::HistoryBar> = bars.iter().map(|b| (b.t, b)).collect();
                    let prev = map.get(&decision_ts).copied();
                    let prev2 = if day_idx >= 2 {
                        map.get(&replay_ts[day_idx - 2]).copied()
                    } else {
                        None
                    };
                    let next = map.get(&exec_ts).copied();
                    let Some(next_bar) = next else {
                        continue;
                    };
                    let ret_prev = match (prev, prev2) {
                        (Some(p1), Some(p0)) if p1.close > 0.0 && p0.close > 0.0 => (p1.close / p0.close).ln(),
                        _ => 0.0,
                    };
                    let is_halt = (next_bar.high - next_bar.low).abs() <= f64::EPSILON;
                    (next_bar.open, next_bar.close, ret_prev, is_halt, "IBKR_HISTORY_REAL_BAR".to_string())
                };
                total_rows += 1;
                if open_px <= 0.0 || close_px <= 0.0 {
                    missing_price_rows += 1;
                    continue;
                }

                let side = if ret_prev >= 0.0 { "LONG" } else { "SHORT" }.to_string();
                let signal_abs_return = ret_prev.exp_m1().abs();
                let estimated_cost_bps = exchange.ibkr_commission_bps
                    + exchange.exchange_fee_bps
                    + exchange.clearing_fee_bps
                    + exchange.regulatory_fee_bps
                    + exchange.fx_fee_bps
                    + exchange.tax_bps
                    + exchange.spread_bps
                    + exchange.slippage_bps
                    + exchange.market_impact_bps
                    + if side == "SHORT" { exchange.borrow_fee_bps } else { 0.0 };
                let required_edge = (estimated_cost_bps + req.edge_cost_buffer_bps) / 10_000.0;
                let min_signal = req.min_signal_abs_return_bps / 10_000.0;
                if is_halt || signal_abs_return < min_signal || signal_abs_return <= required_edge {
                    let skip_reason = if is_halt {
                        "skip_halt_or_stale_quote"
                    } else if signal_abs_return < min_signal {
                        "skip_signal_too_small"
                    } else {
                        "skip_edge_below_cost_floor"
                    };
                    let _ = push_event(
                        &redis,
                        &run_id_cloned,
                        "TradeSkipped",
                        Some(exchange.code.to_string()),
                        json!({
                            "symbol": sym,
                            "side": side,
                            "ret_prev_ln": ret_prev,
                            "signal_abs_return": signal_abs_return,
                            "estimated_cost_bps": estimated_cost_bps,
                            "edge_cost_buffer_bps": req.edge_cost_buffer_bps,
                            "required_edge": required_edge,
                            "min_signal_abs_return_bps": req.min_signal_abs_return_bps,
                            "reason": skip_reason
                        }),
                    )
                    .await;
                    continue;
                }
                if req.min_hold_seconds > 0 {
                    if let Some(last_fill_ts) = last_fill_ts_by_symbol.get(&sym).copied() {
                        let elapsed = exec_ts.saturating_sub(last_fill_ts);
                        if elapsed < req.min_hold_seconds {
                            let _ = push_event(
                                &redis,
                                &run_id_cloned,
                                "TradeSkipped",
                                Some(exchange.code.to_string()),
                                json!({
                                    "symbol": sym,
                                    "side": side,
                                    "reason": "skip_min_hold_window",
                                    "last_fill_ts": last_fill_ts,
                                    "elapsed_seconds": elapsed,
                                    "required_min_hold_seconds": req.min_hold_seconds
                                }),
                            )
                            .await;
                            continue;
                        }
                    }
                }
                if let Some(cooldown_until) = cooldown_until_by_symbol.get(&sym).copied() {
                    if exec_ts < cooldown_until {
                        let _ = push_event(
                            &redis,
                            &run_id_cloned,
                            "TradeSkipped",
                            Some(exchange.code.to_string()),
                            json!({
                                "symbol": sym,
                                "side": side,
                                "reason": "skip_symbol_cooldown",
                                "cooldown_until_ts": cooldown_until,
                                "remaining_seconds": cooldown_until.saturating_sub(exec_ts)
                            }),
                        )
                        .await;
                        continue;
                    }
                }
                *exchange_leg_count.entry(exchange.code.to_string()).or_insert(0) += 1;
                let (hybrid_score, rationale) = selected_scores
                    .get(&sym)
                    .cloned()
                    .unwrap_or((0.0, vec!["not-selected-fallback".to_string()]));
                let decision_id = format!("d-{}-{}", decision_ts, sym);
                let mut rationale_extended = rationale.clone();
                rationale_extended.push(format!("session_open_utc={}", exchange.session_open_utc));
                rationale_extended.push(format!("session_close_utc={}", exchange.session_close_utc));
                let trace = SimDecisionTrace {
                    decision_id: decision_id.clone(),
                    run_id: run_id_cloned.clone(),
                    decision_ts,
                    symbol: sym.clone(),
                    exchange: exchange.code.to_string(),
                    side: side.clone(),
                    weight: if symbol_count == 0 {
                        0.0
                    } else {
                        1.0 / symbol_count as f64
                    },
                    hybrid_score,
                    features: json!({
                        "ret_prev_ln": ret_prev,
                        "next_open": open_px,
                        "next_close": close_px,
                        "spread_bps": exchange.spread_bps,
                        "depth_notional_usd": exchange.depth_notional_usd,
                        "auction_window": exchange.auction_window,
                        "borrow_available": exchange.short_borrow_available,
                        "is_halt": is_halt,
                        "signal_abs_return": signal_abs_return,
                        "estimated_cost_bps": estimated_cost_bps,
                        "required_edge": required_edge,
                        "live_market_data": req.live_market_data,
                    }),
                    rationale: rationale_extended,
                    short_explanation: format!(
                        "{} {} using real market prices (buy {:.4} at {} UTC, sell {:.4} at {} UTC) because momentum {:.4} and hybrid score {:.4} favored {} on {}.",
                        if side == "LONG" { "Buy" } else { "Sell short" },
                        sym,
                        open_px,
                        if req.live_market_data { hhmm_utc_from_ts(exec_ts) } else { exchange.session_open_utc.to_string() },
                        close_px,
                        if req.live_market_data { hhmm_utc_from_ts(exec_ts) } else { exchange.session_close_utc.to_string() },
                        ret_prev,
                        hybrid_score,
                        if side == "LONG" { "upside" } else { "downside" },
                        exchange.code
                    ),
                };
                decision_rows.push(trace.clone());
                total_decisions += 1;
                explained_decisions += 1;

                let order_id = format!("o-{}-{}", exec_ts, sym);
                let order_type = if exchange.auction_window { "MOO" } else { "LIMIT" };
                let mut order_reasons = vec![
                    format!("hybrid_score={:.6}", hybrid_score),
                    format!("ret_prev_ln={:.6}", ret_prev),
                    format!("exchange={}", exchange.code),
                    format!("borrow_available={}", exchange.short_borrow_available),
                ];
                if is_halt {
                    order_reasons.push("simulated_halt_spread_widened".to_string());
                }
                let short_blocked = side == "SHORT" && !exchange.short_borrow_available;
                let fill_ratio = if short_blocked { 0.0 } else if per_leg_notional > exchange.depth_notional_usd {
                    (exchange.depth_notional_usd / per_leg_notional).clamp(0.1, 1.0)
                } else {
                    1.0
                };
                let submitted = SimOrderLedgerRow {
                    order_id: order_id.clone(),
                    decision_id: decision_id.clone(),
                    run_id: run_id_cloned.clone(),
                    ts: decision_ts,
                    wall_clock_ts: now_ts(),
                    symbol: sym.clone(),
                    exchange: exchange.code.to_string(),
                    side: side.clone(),
                    order_type: order_type.to_string(),
                    tif: "DAY".to_string(),
                    qty_notional_usd: per_leg_notional,
                    intended_px: open_px,
                    status: "submitted".to_string(),
                    filled_notional_usd: 0.0,
                    remaining_notional_usd: per_leg_notional,
                    venue_latency_ms: exchange.latency_ms,
                    reasons: order_reasons.clone(),
                };
                orders.push(submitted);

                let ret_simple = (close_px / open_px) - 1.0;
                let direction = if side == "LONG" { 1.0 } else { -1.0 };
                let filled_notional = per_leg_notional * fill_ratio;
                let remaining_notional = (per_leg_notional - filled_notional).max(0.0);
                let ibkr_commission_usd = filled_notional * (exchange.ibkr_commission_bps / 10_000.0);
                let exchange_fee_usd = filled_notional * (exchange.exchange_fee_bps / 10_000.0);
                let clearing_fee_usd = filled_notional * (exchange.clearing_fee_bps / 10_000.0);
                let regulatory_fee_usd = filled_notional * (exchange.regulatory_fee_bps / 10_000.0);
                let fx_fee_usd = filled_notional * (exchange.fx_fee_bps / 10_000.0);
                let tax_usd = filled_notional * (exchange.tax_bps / 10_000.0);
                let fee_usd = ibkr_commission_usd
                    + exchange_fee_usd
                    + clearing_fee_usd
                    + regulatory_fee_usd
                    + fx_fee_usd
                    + tax_usd;
                let spread_cost_usd = filled_notional * (exchange.spread_bps / 10_000.0);
                let slippage_usd = filled_notional * (exchange.slippage_bps / 10_000.0);
                let impact_usd = filled_notional * (exchange.market_impact_bps / 10_000.0);
                let borrow_fee_usd = if side == "SHORT" {
                    filled_notional * (exchange.borrow_fee_bps / 10_000.0)
                } else {
                    0.0
                };
                let pnl_usd = (direction * filled_notional * ret_simple)
                    - fee_usd
                    - slippage_usd
                    - spread_cost_usd
                    - impact_usd
                    - borrow_fee_usd;
                let total_cost_usd = fee_usd + slippage_usd + spread_cost_usd + impact_usd + borrow_fee_usd;
                let status = if short_blocked {
                    "rejected"
                } else if fill_ratio < 0.999 {
                    "partially_filled"
                } else {
                    "filled"
                };
                orders.push(SimOrderLedgerRow {
                    order_id: order_id.clone(),
                    decision_id: decision_id.clone(),
                    run_id: run_id_cloned.clone(),
                    ts: exec_ts,
                    wall_clock_ts: now_ts(),
                    symbol: sym.clone(),
                    exchange: exchange.code.to_string(),
                    side: side.clone(),
                    order_type: order_type.to_string(),
                    tif: "DAY".to_string(),
                    qty_notional_usd: per_leg_notional,
                    intended_px: open_px,
                    status: status.to_string(),
                    filled_notional_usd: filled_notional,
                    remaining_notional_usd: remaining_notional,
                    venue_latency_ms: exchange.latency_ms + exchange.auction_extra_latency_ms,
                    reasons: order_reasons,
                });
                fills.push(SimFillLedgerRow {
                    decision_id: decision_id.clone(),
                    run_id: run_id_cloned.clone(),
                    exec_ts,
                    symbol: sym.clone(),
                    exchange: exchange.code.to_string(),
                    side,
                    qty_notional_usd: filled_notional,
                    open_px,
                    close_px,
                    buy_px: open_px,
                    sell_px: close_px,
                    buy_ts: exec_ts,
                    sell_ts: exec_ts,
                    buy_session_time_utc: if req.live_market_data {
                        hhmm_utc_from_ts(exec_ts)
                    } else {
                        exchange.session_open_utc.to_string()
                    },
                    sell_session_time_utc: if req.live_market_data {
                        hhmm_utc_from_ts(exec_ts)
                    } else {
                        exchange.session_close_utc.to_string()
                    },
                    market_data_source,
                    ret_simple,
                    fee_usd,
                    ibkr_commission_usd,
                    exchange_fee_usd,
                    clearing_fee_usd,
                    regulatory_fee_usd,
                    fx_fee_usd,
                    tax_usd,
                    slippage_usd,
                    pnl_usd,
                    latency_ms: exchange.latency_ms,
                    order_id: Some(order_id),
                    fill_ratio,
                    borrow_fee_usd,
                    market_impact_usd: impact_usd + spread_cost_usd,
                    total_cost_usd,
                });
                if filled_notional > 0.0 {
                    last_fill_ts_by_symbol.insert(sym.clone(), exec_ts);
                    if pnl_usd < 0.0 && req.symbol_cooldown_seconds > 0 {
                        cooldown_until_by_symbol
                            .insert(sym.clone(), exec_ts.saturating_add(req.symbol_cooldown_seconds));
                    }
                }
            }

            for d in &decision_rows {
                let _ = append_json(&redis, &run_decisions_key(&run_id_cloned), d).await;
                let _ = push_event(
                    &redis,
                    &run_id_cloned,
                    "DecisionCreated",
                    Some(d.exchange.clone()),
                    json!({
                        "decision_id": d.decision_id,
                        "symbol": d.symbol,
                        "side": d.side,
                        "weight": d.weight,
                        "hybrid_score": d.hybrid_score,
                        "features": d.features,
                        "short_explanation": d.short_explanation,
                    }),
                )
                .await;
            }
            for o in &orders {
                let _ = append_json(&redis, &run_orders_key(&run_id_cloned), o).await;
                let event_kind = match o.status.as_str() {
                    "submitted" => "OrderSubmitted",
                    "partially_filled" => "OrderPartiallyFilled",
                    "filled" => "OrderFilled",
                    "rejected" => "OrderRejected",
                    _ => "OrderUpdated",
                };
                let _ = push_event(
                    &redis,
                    &run_id_cloned,
                    event_kind,
                    Some(o.exchange.clone()),
                    json!({
                        "order_id": o.order_id,
                        "decision_id": o.decision_id,
                        "symbol": o.symbol,
                        "side": o.side,
                        "status": o.status,
                        "filled_notional_usd": o.filled_notional_usd,
                        "remaining_notional_usd": o.remaining_notional_usd,
                        "reasons": o.reasons,
                    }),
                )
                .await;
            }
            let mut pnl_day = 0.0;
            let mut benchmark_day = 0.0;
            let mut leg_returns: Vec<f64> = Vec::new();
            for f in &fills {
                pnl_day += f.pnl_usd;
                let _ = append_json(&redis, &run_fills_key(&run_id_cloned), f).await;
                leg_returns.push(f.ret_simple.abs());
                benchmark_day += (f.qty_notional_usd * f.ret_simple) / (symbol_count.max(1) as f64);
                let _ = push_event(
                    &redis,
                    &run_id_cloned,
                    "OrderFilled",
                    Some(f.exchange.clone()),
                    json!({
                        "decision_id": f.decision_id,
                        "symbol": f.symbol,
                        "pnl_usd": f.pnl_usd,
                        "buy_px": f.buy_px,
                        "sell_px": f.sell_px,
                        "buy_ts": f.buy_ts,
                        "sell_ts": f.sell_ts,
                        "buy_session_time_utc": f.buy_session_time_utc,
                        "sell_session_time_utc": f.sell_session_time_utc,
                        "market_data_source": f.market_data_source,
                        "fee_usd": f.fee_usd,
                        "ibkr_commission_usd": f.ibkr_commission_usd,
                        "exchange_fee_usd": f.exchange_fee_usd,
                        "clearing_fee_usd": f.clearing_fee_usd,
                        "regulatory_fee_usd": f.regulatory_fee_usd,
                        "fx_fee_usd": f.fx_fee_usd,
                        "tax_usd": f.tax_usd,
                        "slippage_usd": f.slippage_usd,
                        "borrow_fee_usd": f.borrow_fee_usd,
                        "market_impact_usd": f.market_impact_usd,
                        "total_cost_usd": f.total_cost_usd,
                        "fill_ratio": f.fill_ratio,
                        "order_id": f.order_id,
                        "latency_ms": f.latency_ms,
                    }),
                )
                .await;
            }
            if req.reinvest_profit {
                equity += pnl_day;
                if equity < 0.0 {
                    equity = 0.0;
                }
            }
            benchmark_equity += benchmark_day;
            cash_usd = equity;
            peak_equity = peak_equity.max(equity);
            daily_pnls.push(pnl_day);
            let drawdown = if peak_equity > 0.0 {
                (peak_equity - equity).max(0.0) / peak_equity
            } else {
                0.0
            };
            let var_95_1d = daily_pnls
                .iter()
                .rev()
                .take(60)
                .map(|v| -v / req.initial_capital_usd.max(1.0))
                .fold(0.0_f64, f64::max);
            let avg_abs_ret = if leg_returns.is_empty() {
                0.0
            } else {
                leg_returns.iter().sum::<f64>() / leg_returns.len() as f64
            };
            let gross_exposure = per_leg_notional * symbol_count as f64;
            let weights = if symbol_count == 0 {
                vec![1.0]
            } else {
                vec![1.0 / symbol_count as f64; symbol_count]
            };
            let risk = SimRiskSnapshot {
                run_id: run_id_cloned.clone(),
                ts: exec_ts,
                equity_usd: equity,
                cash_usd,
                gross_exposure_usd: gross_exposure,
                leverage: gross_exposure / equity.max(1.0),
                drawdown,
                var_95_1d: (var_95_1d + avg_abs_ret * 0.1).clamp(0.0, 1.0),
                concentration_hhi: compute_hhi(&weights),
                benchmark_equity_usd: benchmark_equity,
                relative_return: (equity - benchmark_equity) / req.initial_capital_usd.max(1.0),
            };
            risk_snapshots.push(risk.clone());
            let _ = append_json(&redis, &run_risk_key(&run_id_cloned), &risk).await;
            let curve = SimPortfolioPoint {
                ts: exec_ts,
                equity_usd: equity,
                pnl_cum_usd: equity - req.initial_capital_usd,
            };
            let _ = append_json(&redis, &run_curve_key(&run_id_cloned), &curve).await;
            let _ = push_event(
                &redis,
                &run_id_cloned,
                "PortfolioValued",
                None,
                json!({
                    "ts": exec_ts,
                    "equity_usd": curve.equity_usd,
                    "pnl_cum_usd": curve.pnl_cum_usd,
                }),
            )
            .await;
            let _ = push_event(
                &redis,
                &run_id_cloned,
                "RiskSnapshot",
                None,
                json!({
                    "drawdown": risk.drawdown,
                    "var_95_1d": risk.var_95_1d,
                    "leverage": risk.leverage,
                    "concentration_hhi": risk.concentration_hhi,
                    "relative_return": risk.relative_return
                }),
            )
            .await;
            if risk.drawdown >= 0.55 || risk.leverage >= 3.0 || risk.var_95_1d >= 0.2 {
                let _ = push_event(
                    &redis,
                    &run_id_cloned,
                    "RiskThresholdBreached",
                    None,
                    json!({
                        "drawdown": risk.drawdown,
                        "leverage": risk.leverage,
                        "var_95_1d": risk.var_95_1d
                    }),
                )
                .await;
            }
            cycle_idx += 1;
            meta.cycles_completed = cycle_idx;
            let explain_coverage = if total_decisions == 0 {
                1.0
            } else {
                explained_decisions as f64 / total_decisions as f64
            };
            let readiness = compute_readiness(
                &run_id_cloned,
                req.readiness_min_days,
                &risk_snapshots,
                &daily_pnls,
                missing_price_rows,
                explain_coverage,
            );
            meta.readiness_score = Some(readiness.score);
            meta.readiness_passed = Some(readiness.pass);
            // Live snapshot for UI polling (status endpoint reads Redis meta only at start/end otherwise).
            meta.ending_equity_usd = Some(equity);
            meta.pnl_usd = Some(equity - req.initial_capital_usd);
            let _ = {
                let mut conn = redis.clone();
                let raw = serde_json::to_string(&readiness).unwrap_or_else(|_| "{}".to_string());
                conn.set::<_, _, ()>(run_readiness_key(&run_id_cloned), raw).await
            };
            let _ = set_meta(&redis, &meta).await;
            // Do not auto-stop on readiness pass; continue simulation until bankroll is depleted
            // (or manually stopped), while continuously tracking readiness.
            if equity <= 0.0 {
                stop_reason = "BANKRUPT".to_string();
                break;
            }
            if req.replay_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(req.replay_delay_ms)).await;
            }
            day_idx += 1;
        }

        if total_rows > 0 {
            let missing_ratio = missing_price_rows as f64 / total_rows as f64;
            if missing_ratio > 0.10 {
                findings.push(SimBiasFinding {
                    severity: "medium".to_string(),
                    code: "missing_price_data".to_string(),
                    message: "High ratio of missing/invalid execution prices in replay".to_string(),
                    details: json!({"missing_ratio": missing_ratio, "missing_rows": missing_price_rows, "total_rows": total_rows}),
                });
            }
        }
        let total_legs = exchange_leg_count.values().sum::<usize>().max(1);
        for (ex, n) in &exchange_leg_count {
            let share = *n as f64 / total_legs as f64;
            if share > 0.60 {
                findings.push(SimBiasFinding {
                    severity: "low".to_string(),
                    code: "exchange_concentration".to_string(),
                    message: "Decision concentration is heavily skewed to one exchange".to_string(),
                    details: json!({"exchange": ex, "share": share}),
                });
            }
        }

        for finding in &findings {
            let _ = append_json(&redis, &run_bias_key(&run_id_cloned), finding).await;
            let _ = push_event(
                &redis,
                &run_id_cloned,
                "DataQualityFlagged",
                None,
                json!({
                    "code": finding.code,
                    "severity": finding.severity,
                    "message": finding.message,
                    "details": finding.details
                }),
            )
            .await;
        }

        meta.status = "completed".to_string();
        meta.finished_at_ts = Some(now_ts());
        meta.ending_equity_usd = Some(equity);
        meta.pnl_usd = Some(equity - req.initial_capital_usd);
        meta.stop_reason = Some(stop_reason.clone());
        let _ = set_meta(&redis, &meta).await;
        let _ = push_event(
            &redis,
            &run_id_cloned,
            "RunCompleted",
            None,
            json!({
                "ending_equity_usd": equity,
                "pnl_usd": equity - req.initial_capital_usd,
                "stop_reason": stop_reason,
                "cycles_completed": meta.cycles_completed,
                "readiness_score": meta.readiness_score,
                "readiness_passed": meta.readiness_passed
            }),
        )
        .await;
    });

    Ok(meta)
}
