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
    pub ret_simple: f64,
    pub fee_usd: f64,
    pub slippage_usd: f64,
    pub pnl_usd: f64,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimPortfolioPoint {
    pub ts: u64,
    pub equity_usd: f64,
    pub pnl_cum_usd: f64,
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
    let rows: Vec<String> = conn
        .lrange(run_events_key(run_id), 0, (limit.max(1) as isize) - 1)
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

    req.days = req.days.max(5).min(30);
    req.top = req.top.max(6).min(40);
    req.quantile = req.quantile.clamp(0.05, 0.45);
    req.initial_capital_usd = req.initial_capital_usd.max(25.0);

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
    for (sym, conid) in &pairs {
        if !selected_symbols.iter().any(|s| s == sym) {
            continue;
        }
        let bars = client
            .fetch_history(*conid, "1y", "1d")
            .await
            .map_err(|e| e.to_string())?;
        let bars = daily_strategy::sort_dedupe_bars(bars);
        if bars.len() >= req.days + 3 {
            bars_by_symbol.insert(sym.clone(), bars);
        }
    }
    if bars_by_symbol.len() < 4 {
        return Err("not enough symbols with stable daily history for replay".to_string());
    }

    let aligned_ts = daily_strategy::aligned_timestamps(&bars_by_symbol);
    if aligned_ts.len() < req.days + 2 {
        return Err("not enough aligned days to run requested simulation horizon".to_string());
    }
    let replay_ts = aligned_ts[aligned_ts.len() - (req.days + 1)..].to_vec();

    let mut exch_set = std::collections::BTreeSet::<String>::new();
    for sym in bars_by_symbol.keys() {
        exch_set.insert(exchanges::infer_exchange(sym).code.to_string());
    }
    let exchanges = exch_set.into_iter().collect::<Vec<_>>();

    let mut meta = SimRunMeta {
        run_id: run_id.clone(),
        status: "queued".to_string(),
        created_at_ts: created_at,
        started_at_ts: None,
        finished_at_ts: None,
        days_requested: req.days,
        symbols_count: bars_by_symbol.len(),
        exchanges,
        initial_capital_usd: req.initial_capital_usd,
        ending_equity_usd: None,
        pnl_usd: None,
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

    let run_id_cloned = run_id.clone();
    tokio::spawn(async move {
        let mut equity = req.initial_capital_usd;
        let mut findings: Vec<SimBiasFinding> = Vec::new();
        let mut exchange_leg_count: HashMap<String, usize> = HashMap::new();
        let mut missing_price_rows = 0usize;
        let mut total_rows = 0usize;

        meta.status = "running".to_string();
        meta.started_at_ts = Some(now_ts());
        let _ = set_meta(&redis, &meta).await;
        let _ = push_event(
            &redis,
            &run_id_cloned,
            "RunStarted",
            None,
            json!({"days": req.days, "symbols": bars_by_symbol.len(), "initial_capital_usd": req.initial_capital_usd}),
        )
        .await;

        for day_idx in 1..replay_ts.len() {
            let exec_ts = replay_ts[day_idx];
            let decision_ts = replay_ts[day_idx - 1];

            let mut decision_rows: Vec<SimDecisionTrace> = Vec::new();
            let mut fills: Vec<SimFillLedgerRow> = Vec::new();
            let symbols: Vec<String> = bars_by_symbol.keys().cloned().collect();
            let symbol_count = symbols.len();
            let per_leg_notional = if symbol_count == 0 {
                0.0
            } else {
                equity / symbol_count as f64
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
                total_rows += 1;
                if next_bar.open <= 0.0 || next_bar.close <= 0.0 {
                    missing_price_rows += 1;
                    continue;
                }

                let ret_prev = match (prev, prev2) {
                    (Some(p1), Some(p0)) if p1.close > 0.0 && p0.close > 0.0 => (p1.close / p0.close).ln(),
                    _ => 0.0,
                };
                let side = if ret_prev >= 0.0 { "LONG" } else { "SHORT" }.to_string();
                let exchange = exchanges::infer_exchange(&sym);
                *exchange_leg_count.entry(exchange.code.to_string()).or_insert(0) += 1;
                let (hybrid_score, rationale) = selected_scores
                    .get(&sym)
                    .cloned()
                    .unwrap_or((0.0, vec!["not-selected-fallback".to_string()]));
                let decision_id = format!("d-{}-{}", decision_ts, sym);
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
                        "next_open": next_bar.open,
                        "next_close": next_bar.close,
                    }),
                    rationale,
                };
                decision_rows.push(trace.clone());

                let ret_simple = (next_bar.close / next_bar.open) - 1.0;
                let direction = if side == "LONG" { 1.0 } else { -1.0 };
                let fee_usd = per_leg_notional * (exchange.fee_bps / 10_000.0);
                let slippage_usd = per_leg_notional * (exchange.slippage_bps / 10_000.0);
                let pnl_usd = (direction * per_leg_notional * ret_simple) - fee_usd - slippage_usd;
                fills.push(SimFillLedgerRow {
                    decision_id: decision_id.clone(),
                    run_id: run_id_cloned.clone(),
                    exec_ts,
                    symbol: sym.clone(),
                    exchange: exchange.code.to_string(),
                    side,
                    qty_notional_usd: per_leg_notional,
                    open_px: next_bar.open,
                    close_px: next_bar.close,
                    ret_simple,
                    fee_usd,
                    slippage_usd,
                    pnl_usd,
                    latency_ms: exchange.latency_ms,
                });
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
                    }),
                )
                .await;
            }
            let mut pnl_day = 0.0;
            for f in &fills {
                pnl_day += f.pnl_usd;
                let _ = append_json(&redis, &run_fills_key(&run_id_cloned), f).await;
                let _ = push_event(
                    &redis,
                    &run_id_cloned,
                    "OrderFilled",
                    Some(f.exchange.clone()),
                    json!({
                        "decision_id": f.decision_id,
                        "symbol": f.symbol,
                        "pnl_usd": f.pnl_usd,
                        "fee_usd": f.fee_usd,
                        "slippage_usd": f.slippage_usd,
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
            if req.replay_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(req.replay_delay_ms)).await;
            }
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
        let _ = set_meta(&redis, &meta).await;
        let _ = push_event(
            &redis,
            &run_id_cloned,
            "RunCompleted",
            None,
            json!({"ending_equity_usd": equity, "pnl_usd": equity - req.initial_capital_usd}),
        )
        .await;
    });

    Ok(meta)
}
