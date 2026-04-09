//! Phase-1 daily momentum strategy: decision at close of day `t`, execution next session
//! open → close (`t+1`). Label for research: `y(t) = ln(C_{t+1}/O_{t+1})`.
//!
//! Features (no lookahead): 5d/20d momentum on closes, 20d realized vol of daily log returns.
//! IBKR history bars do not include volume — volume surprise is omitted until we add a feed.

use crate::ibkr::{HistoryBar, IbkrOAuthClient};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

fn day_start_ts_utc(ts: u64) -> u64 {
    use chrono::{Datelike, TimeZone, Utc};
    let dt = Utc.timestamp_opt(ts as i64, 0).single().unwrap_or_else(Utc::now);
    Utc.with_ymd_and_hms(dt.year(), dt.month(), dt.day(), 0, 0, 0)
        .single()
        .map(|d| d.timestamp().max(0) as u64)
        .unwrap_or(ts)
}

/// Top/bottom fraction of the cross-section for long/short (20% each → 2 names on a 10-name list).
pub const DEFAULT_QUANTILE: f64 = 0.2;

#[derive(Clone, Debug, Serialize)]
pub struct SymbolFeatures {
    pub symbol: String,
    pub mom5: f64,
    pub mom20: f64,
    pub vol20: f64,
    pub rank_mom5_cs: f64,
    pub rank_mom20_cs: f64,
    pub rank_vol20_cs: f64,
    pub vol_regime: f64,
    /// Cross-sectional composite: z(mom5)+z(mom20)-z(vol20)
    pub score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct DailySignal {
    pub symbol: String,
    /// LONG | SHORT | FLAT
    pub side: &'static str,
    pub weight: f64,
    pub score: f64,
}

#[derive(Debug, Serialize)]
pub struct DailySignalsResponse {
    pub strategy: &'static str,
    pub execution: &'static str,
    pub label: &'static str,
    /// Unix seconds of last aligned bar; omitted / null in JSON when `data_available` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_of_ts: Option<u64>,
    /// False when IBKR returned empty/partial history (e.g. holiday), overlap too short, or features cannot be formed.
    pub data_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Counts for debugging when data is missing (always present).
    pub symbols_with_history: usize,
    pub overlapping_days: usize,
    pub universe: Vec<String>,
    pub quantile: f64,
    pub short_enabled: bool,
    pub features: Vec<SymbolFeatures>,
    pub signals: Vec<DailySignal>,
    pub backtest: Option<BacktestSummary>,
}

#[derive(Debug, Serialize)]
pub struct BacktestSummary {
    pub trading_days: usize,
    pub mean_daily: f64,
    pub vol_daily: f64,
    pub sharpe_252: f64,
    pub cumulative_return: f64,
    pub max_drawdown: f64,
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn std_sample(xs: &[f64], m: f64) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let v: f64 = xs
        .iter()
        .map(|x| {
            let d = x - m;
            d * d
        })
        .sum::<f64>()
        / (xs.len() as f64 - 1.0);
    v.sqrt()
}

fn z_scores(vals: &[f64]) -> Vec<f64> {
    let m = mean(vals);
    let sd = std_sample(vals, m);
    if sd < 1e-12 {
        return vec![0.0; vals.len()];
    }
    vals.iter().map(|x| (x - m) / sd).collect()
}

fn percentile_ranks(vals: &[f64]) -> Vec<f64> {
    let n = vals.len();
    if n <= 1 {
        return vec![0.5; n];
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        vals[a]
            .partial_cmp(&vals[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut ranks = vec![0.5_f64; n];
    let denom = (n - 1) as f64;
    for (k, idx) in order.into_iter().enumerate() {
        ranks[idx] = k as f64 / denom;
    }
    ranks
}

fn median(vals: &[f64]) -> f64 {
    if vals.is_empty() {
        return 0.0;
    }
    let mut v = vals.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// Sort by `t` ascending and keep one bar per timestamp (last wins).
pub fn sort_dedupe_bars(mut bars: Vec<HistoryBar>) -> Vec<HistoryBar> {
    bars.sort_by_key(|b| b.t);
    let mut out: Vec<HistoryBar> = Vec::with_capacity(bars.len());
    for b in bars {
        if let Some(last) = out.last_mut() {
            if last.t == b.t {
                *last = b;
            } else {
                out.push(b);
            }
        } else {
            out.push(b);
        }
    }
    out
}

/// Intersection of bar timestamps across symbols (all must have that day).
pub fn aligned_timestamps(bars_by_symbol: &HashMap<String, Vec<HistoryBar>>) -> Vec<u64> {
    let mut sets: Option<HashSet<u64>> = None;
    for bars in bars_by_symbol.values() {
        // IBKR daily bars can have symbol-specific timestamps (exchange timezones / feed quirks).
        // Align by UTC calendar day (00:00:00 UTC) instead of exact `t` seconds.
        let ts: HashSet<u64> = bars.iter().map(|b| day_start_ts_utc(b.t)).collect();
        sets = Some(match sets {
            None => ts,
            Some(s) => s.intersection(&ts).copied().collect(),
        });
    }
    let mut v: Vec<u64> = sets.unwrap_or_default().into_iter().collect();
    v.sort_unstable();
    v
}

/// Per-day log returns aligned to `closes` (len = n), rets[i] = ln(C_i/C_{i-1}), i>=1.
fn log_returns_from_closes(closes: &[f64]) -> Vec<f64> {
    let mut r = Vec::with_capacity(closes.len().saturating_sub(1));
    for i in 1..closes.len() {
        let c0 = closes[i - 1];
        let c1 = closes[i];
        if c0 > 0.0 && c1 > 0.0 {
            r.push((c1 / c0).ln());
        } else {
            r.push(0.0);
        }
    }
    r
}

/// Features at end of day index `i` (requires i >= 20).
fn features_at_index(closes: &[f64], rets: &[f64], i: usize) -> Option<(f64, f64, f64)> {
    if i < 20 || i >= closes.len() {
        return None;
    }
    let c = closes[i];
    if c <= 0.0 {
        return None;
    }
    let c5 = closes[i - 5];
    let c20 = closes[i - 20];
    if c5 <= 0.0 || c20 <= 0.0 {
        return None;
    }
    let mom5 = (c / c5).ln();
    let mom20 = (c / c20).ln();
    // 20 daily returns ending at i: rets[i-19]..=rets[i]
    if i < 19 || rets.len() <= i {
        return None;
    }
    let window = &rets[i - 19..=i];
    let m = mean(window);
    let vol20 = std_sample(window, m);
    Some((mom5, mom20, vol20.max(1e-12)))
}

/// Forward label matching next-session open→close: ln(C_{i+1}/O_{i+1}).
fn forward_oc_return(o_next: f64, c_next: f64) -> Option<f64> {
    if o_next > 0.0 && c_next > 0.0 {
        Some((c_next / o_next).ln())
    } else {
        None
    }
}

/// Build closes/opens aligned to common timeline `ts` for one symbol.
fn series_on_timestamps(bars: &[HistoryBar], ts: &[u64]) -> Option<(Vec<f64>, Vec<f64>)> {
    let map: HashMap<u64, &HistoryBar> = bars
        .iter()
        .map(|b| (day_start_ts_utc(b.t), b))
        .collect();
    let mut closes = Vec::with_capacity(ts.len());
    let mut opens = Vec::with_capacity(ts.len());
    for t in ts {
        let b = map.get(t)?;
        if b.close > 0.0 && b.open > 0.0 {
            closes.push(b.close);
            opens.push(b.open);
        } else {
            return None;
        }
    }
    Some((opens, closes))
}

#[derive(Clone)]
struct PreparedSymbol {
    symbol: String,
    opens: Vec<f64>,
    closes: Vec<f64>,
    rets: Vec<f64>,
}

fn prepare_symbol(symbol: String, bars: Vec<HistoryBar>, ts: &[u64]) -> Option<PreparedSymbol> {
    let bars = sort_dedupe_bars(bars);
    let (opens, closes) = series_on_timestamps(&bars, ts)?;
    let rets = log_returns_from_closes(&closes);
    Some(PreparedSymbol {
        symbol,
        opens,
        closes,
        rets,
    })
}

#[derive(Clone)]
struct DayFeatureRow {
    symbol: String,
    mom5: f64,
    mom20: f64,
    vol20: f64,
    rank_mom5_cs: f64,
    rank_mom20_cs: f64,
    rank_vol20_cs: f64,
    vol_regime: f64,
    score: f64,
}

/// Cross-sectional composite scores at day index `i`.
fn scores_at_day(prepared: &[PreparedSymbol], i: usize) -> Option<Vec<DayFeatureRow>> {
    let mut rows = Vec::new();
    for p in prepared {
        let f = features_at_index(&p.closes, &p.rets, i)?;
        rows.push((p.symbol.clone(), f.0, f.1, f.2));
    }
    if rows.is_empty() {
        return None;
    }
    let mom5: Vec<f64> = rows.iter().map(|(_, m5, _, _)| *m5).collect();
    let mom20: Vec<f64> = rows.iter().map(|(_, _, m20, _)| *m20).collect();
    let vols: Vec<f64> = rows.iter().map(|(_, _, _, v)| *v).collect();
    let z5 = z_scores(&mom5);
    let z20 = z_scores(&mom20);
    let zv = z_scores(&vols);
    let rank5 = percentile_ranks(&mom5);
    let rank20 = percentile_ranks(&mom20);
    let rankv = percentile_ranks(&vols);
    let vol_median = median(&vols).max(1e-12);
    let mut out = Vec::new();
    for k in 0..rows.len() {
        let score = z5[k] + z20[k] - zv[k] + (rank5[k] - rankv[k]);
        out.push(DayFeatureRow {
            symbol: rows[k].0.clone(),
            mom5: rows[k].1,
            mom20: rows[k].2,
            vol20: rows[k].3,
            rank_mom5_cs: rank5[k],
            rank_mom20_cs: rank20[k],
            rank_vol20_cs: rankv[k],
            vol_regime: rows[k].3 / vol_median,
            score,
        });
    }
    Some(out)
}

/// Find latest day index that can form a full cross-sectional feature row.
/// This allows fallback from a partial/holiday latest bar to the most recent
/// fully closed aligned day.
fn latest_usable_decision_day(
    prepared: &[PreparedSymbol],
    n_days: usize,
) -> Option<(usize, Vec<DayFeatureRow>)> {
    if n_days <= 20 {
        return None;
    }
    for i in (20..n_days).rev() {
        if let Some(sc) = scores_at_day(prepared, i) {
            return Some((i, sc));
        }
    }
    None
}

/// Rank by score; long top `quantile`, short bottom `quantile`; vol-parity weights within side.
fn signals_from_scores(
    scores: &[DayFeatureRow],
    quantile: f64,
    short_enabled: bool,
) -> (Vec<SymbolFeatures>, Vec<DailySignal>) {
    let n = scores.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        scores[a]
            .score
            .partial_cmp(&scores[b].score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let k = ((n as f64 * quantile).ceil() as usize).max(1).min(n);

    let long_idx: Vec<usize> = order.iter().rev().take(k).copied().collect();
    let short_idx: Vec<usize> = order.iter().take(k).copied().collect();

    let inv_vol = |i: usize| 1.0 / scores[i].vol20.max(1e-8);
    let long_w_raw: f64 = long_idx.iter().map(|&i| inv_vol(i)).sum();
    let short_w_raw: f64 = short_idx.iter().map(|&i| inv_vol(i)).sum();

    let mut features: Vec<SymbolFeatures> = scores
        .iter()
        .map(|row| SymbolFeatures {
            symbol: row.symbol.clone(),
            mom5: row.mom5,
            mom20: row.mom20,
            vol20: row.vol20,
            rank_mom5_cs: row.rank_mom5_cs,
            rank_mom20_cs: row.rank_mom20_cs,
            rank_vol20_cs: row.rank_vol20_cs,
            vol_regime: row.vol_regime,
            score: row.score,
        })
        .collect();

    features.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    let mut signals: Vec<DailySignal> = Vec::new();
    for &i in &long_idx {
        let w = if long_w_raw > 0.0 {
            inv_vol(i) / long_w_raw
        } else {
            1.0 / long_idx.len() as f64
        };
        signals.push(DailySignal {
            symbol: scores[i].symbol.clone(),
            side: "LONG",
            weight: w,
            score: scores[i].score,
        });
    }
    if short_enabled {
        for &i in &short_idx {
            let w = if short_w_raw > 0.0 {
                inv_vol(i) / short_w_raw
            } else {
                1.0 / short_idx.len() as f64
            };
            signals.push(DailySignal {
                symbol: scores[i].symbol.clone(),
                side: "SHORT",
                weight: w,
                score: scores[i].score,
            });
        }
    }

    signals.sort_by(|a, b| a.side.cmp(&b.side).then_with(|| a.symbol.cmp(&b.symbol)));

    (features, signals)
}

fn empty_daily_response(
    universe: Vec<String>,
    quantile: f64,
    short_enabled: bool,
    symbols_with_history: usize,
    overlapping_days: usize,
    reason: String,
) -> DailySignalsResponse {
    DailySignalsResponse {
        strategy: "cross_sectional_momentum_vol_target",
        execution: "enter_next_open_exit_next_close",
        label: "ln(C[t+1]/O[t+1])",
        as_of_ts: None,
        data_available: false,
        reason: Some(reason),
        symbols_with_history,
        overlapping_days,
        universe,
        quantile,
        short_enabled,
        features: vec![],
        signals: vec![],
        backtest: None,
    }
}

fn run_backtest(
    prepared: &[PreparedSymbol],
    quantile: f64,
    short_enabled: bool,
) -> Option<BacktestSummary> {
    let n = prepared.first()?.closes.len();
    if n < 22 {
        return None;
    }
    let mut daily_pnls: Vec<f64> = Vec::new();
    for i in 20..n - 1 {
        let sc = scores_at_day(prepared, i)?;
        let (feats, sigs) = signals_from_scores(&sc, quantile, short_enabled);
        let _ = feats;

        let mut pnl = 0.0;
        for s in &sigs {
            let sym = &s.symbol;
            let p = prepared.iter().find(|x| x.symbol == *sym)?;
            let y = forward_oc_return(p.opens[i + 1], p.closes[i + 1])?;
            let contrib = match s.side {
                "LONG" => s.weight * y,
                "SHORT" => -s.weight * y,
                _ => 0.0,
            };
            pnl += contrib;
        }
        daily_pnls.push(pnl);
    }
    if daily_pnls.is_empty() {
        return None;
    }
    let m = mean(&daily_pnls);
    let sd = std_sample(&daily_pnls, m);
    let sharpe = if sd > 1e-12 {
        (m / sd) * (252_f64).sqrt()
    } else {
        0.0
    };
    let mut cum = 0.0;
    let mut peak = 0.0_f64;
    let mut max_dd = 0.0_f64;
    for &x in &daily_pnls {
        cum += x;
        peak = peak.max(cum);
        max_dd = max_dd.min(cum - peak);
    }
    Some(BacktestSummary {
        trading_days: daily_pnls.len(),
        mean_daily: m,
        vol_daily: sd,
        sharpe_252: sharpe,
        cumulative_return: daily_pnls.iter().sum(),
        max_drawdown: max_dd,
    })
}

/// Build daily-signals payload. On holidays or empty IBKR history, returns **200-safe** empty state
/// (`data_available: false`) instead of failing.
pub async fn compute_daily_signals(
    client: &IbkrOAuthClient,
    symbol_conids: &[(String, u64)],
    quantile: f64,
    short_enabled: bool,
    include_backtest: bool,
) -> DailySignalsResponse {
    let requested_universe: Vec<String> = symbol_conids.iter().map(|(s, _)| s.clone()).collect();

    let mut bars_by_symbol: HashMap<String, Vec<HistoryBar>> = HashMap::new();
    for (sym, conid) in symbol_conids {
        match client.fetch_history(*conid, "1y", "1d").await {
            Ok(bars) => {
                if bars.len() >= 30 {
                    bars_by_symbol.insert(sym.clone(), bars);
                } else {
                    tracing::warn!(
                        "daily-signals: history too short for {} ({} bars); skipping",
                        sym,
                        bars.len()
                    );
                }
            }
            Err(e) => {
                tracing::warn!("daily-signals: history failed for {}: {}", sym, e);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(75)).await;
    }

    let symbols_with_history = bars_by_symbol.len();
    if symbols_with_history < 3 {
        return empty_daily_response(
            requested_universe,
            quantile,
            short_enabled,
            symbols_with_history,
            0,
            "insufficient_symbols_with_history".to_string(),
        );
    }

    // Self-heal data gaps:
    // IBKR can return partial/shifted history per symbol (feed gaps, symbol-specific holidays, etc.),
    // which can collapse the strict intersection to 0. Drop the weakest-history symbols until we
    // regain enough overlapping days to form 20d features.
    let mut dropped_symbols: Vec<String> = Vec::new();
    let mut ts = aligned_timestamps(&bars_by_symbol);
    while ts.len() < 22 && bars_by_symbol.len() >= 3 {
        let mut by_len: Vec<(String, usize)> = bars_by_symbol
            .iter()
            .map(|(s, b)| (s.clone(), b.len()))
            .collect();
        by_len.sort_by_key(|(_, n)| *n);
        let drop = by_len.first().map(|(s, _)| s.clone());
        let Some(sym) = drop else { break };
        bars_by_symbol.remove(&sym);
        dropped_symbols.push(sym);
        ts = aligned_timestamps(&bars_by_symbol);
    }

    let overlapping_days = ts.len();
    if bars_by_symbol.len() < 3 {
        return empty_daily_response(
            requested_universe,
            quantile,
            short_enabled,
            symbols_with_history,
            overlapping_days,
            format!(
                "insufficient_symbols_after_gap_filter(dropped={})",
                dropped_symbols.len()
            ),
        );
    }
    if ts.is_empty() {
        return empty_daily_response(
            requested_universe,
            quantile,
            short_enabled,
            symbols_with_history,
            0,
            format!(
                "no_overlapping_trading_days(dropped={})",
                dropped_symbols.len()
            ),
        );
    }
    if ts.len() < 22 {
        return empty_daily_response(
            requested_universe,
            quantile,
            short_enabled,
            symbols_with_history,
            overlapping_days,
            format!(
                "insufficient_overlapping_history_for_features(days={}, dropped={})",
                ts.len(),
                dropped_symbols.len()
            ),
        );
    }

    let mut prepared: Vec<PreparedSymbol> = Vec::new();
    for (sym, bars) in bars_by_symbol {
        if let Some(p) = prepare_symbol(sym, bars, &ts) {
            prepared.push(p);
        }
    }
    if prepared.len() < 3 {
        return empty_daily_response(
            requested_universe,
            quantile,
            short_enabled,
            symbols_with_history,
            overlapping_days,
            "not_enough_symbols_after_alignment".to_string(),
        );
    }

    let Some((decision_i, sc)) = latest_usable_decision_day(&prepared, ts.len()) else {
        tracing::info!(
            "daily-signals: no usable decision day found in aligned window"
        );
        return empty_daily_response(
            requested_universe,
            quantile,
            short_enabled,
            symbols_with_history,
            overlapping_days,
            "feature_computation_unavailable".to_string(),
        );
    };

    let (features, signals) = signals_from_scores(&sc, quantile, short_enabled);

    let backtest = if include_backtest {
        run_backtest(&prepared, quantile, short_enabled)
    } else {
        None
    };

    let universe: Vec<String> = prepared.iter().map(|p| p.symbol.clone()).collect();
    DailySignalsResponse {
        strategy: "cross_sectional_momentum_vol_target",
        execution: "enter_next_open_exit_next_close",
        label: "ln(C[t+1]/O[t+1])",
        as_of_ts: Some(ts[decision_i]),
        data_available: true,
        reason: None,
        symbols_with_history,
        overlapping_days,
        universe,
        quantile,
        short_enabled,
        features,
        signals,
        backtest,
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ResearchRow {
    pub ts: u64,
    pub symbol: String,
    pub mom5: f64,
    pub mom20: f64,
    pub vol20: f64,
    pub rank_mom5_cs: f64,
    pub rank_mom20_cs: f64,
    pub rank_vol20_cs: f64,
    pub vol_regime: f64,
    pub score: f64,
    /// Label aligned with execution: ln(C[t+1] / O[t+1])
    pub y_oc_next: f64,
}

#[derive(Debug, Serialize)]
pub struct WalkForwardSummary {
    pub folds: usize,
    pub trading_days: usize,
    pub mean_daily: f64,
    pub vol_daily: f64,
    pub sharpe_252: f64,
    pub cumulative_return: f64,
    pub max_drawdown: f64,
}

#[derive(Debug, Serialize)]
pub struct Phase2ResearchResponse {
    pub model_scaffold: &'static str,
    pub label: &'static str,
    pub data_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub symbols_with_history: usize,
    pub overlapping_days: usize,
    pub train_days: usize,
    pub test_days: usize,
    pub quantile: f64,
    pub short_enabled: bool,
    pub row_count: usize,
    pub rows_sample: Vec<ResearchRow>,
    pub walkforward: Option<WalkForwardSummary>,
}

#[derive(Debug, Serialize)]
pub struct LinearModelCoefficients {
    pub intercept: f64,
    pub mom5: f64,
    pub mom20: f64,
    pub vol20: f64,
    pub rank_mom5_cs: f64,
    pub rank_mom20_cs: f64,
    pub rank_vol20_cs: f64,
    pub vol_regime: f64,
}

#[derive(Debug, Serialize)]
pub struct Phase2ModelSignalsResponse {
    pub model: &'static str,
    pub label: &'static str,
    pub data_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_of_ts: Option<u64>,
    pub symbols_with_history: usize,
    pub overlapping_days: usize,
    pub training_rows: usize,
    pub quantile: f64,
    pub short_enabled: bool,
    pub publish_signals: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coefficients: Option<LinearModelCoefficients>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_health: Option<ModelHealth>,
    pub features: Vec<SymbolFeatures>,
    pub signals: Vec<DailySignal>,
}

#[derive(Debug, Serialize)]
pub struct ModelHealth {
    pub feature_row_count: usize,
    pub nan_or_inf_count: usize,
    pub coef_norm: f64,
    pub train_window_days: usize,
}

const MODEL_MIN_FEATURE_ROWS: usize = 5;
const MODEL_MIN_UNIVERSE_SIZE: usize = 5;
const MODEL_MAX_WEIGHT_PER_SYMBOL: f64 = 0.35;
const MODEL_MAX_COEF_NORM: f64 = 5.0;

fn coef_l2_norm(beta: &[f64]) -> f64 {
    beta.iter().skip(1).map(|x| x * x).sum::<f64>().sqrt()
}

fn count_non_finite_values(rows: &[ResearchRow], beta: &[f64], day_scores: &[DayFeatureRow]) -> usize {
    let mut c = 0usize;
    for b in beta {
        if !b.is_finite() {
            c += 1;
        }
    }
    for r in rows {
        let vals = [
            r.mom5,
            r.mom20,
            r.vol20,
            r.rank_mom5_cs,
            r.rank_mom20_cs,
            r.rank_vol20_cs,
            r.vol_regime,
            r.score,
            r.y_oc_next,
        ];
        c += vals.iter().filter(|v| !v.is_finite()).count();
    }
    for r in day_scores {
        let vals = [
            r.mom5,
            r.mom20,
            r.vol20,
            r.rank_mom5_cs,
            r.rank_mom20_cs,
            r.rank_vol20_cs,
            r.vol_regime,
            r.score,
        ];
        c += vals.iter().filter(|v| !v.is_finite()).count();
    }
    c
}

fn solve_linear_system(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = a.len();
    if n == 0 || b.len() != n || a.iter().any(|row| row.len() != n) {
        return None;
    }
    // Gaussian elimination with partial pivoting.
    for col in 0..n {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..n {
            if a[r][col].abs() > best {
                best = a[r][col].abs();
                pivot = r;
            }
        }
        if best < 1e-12 {
            return None;
        }
        if pivot != col {
            a.swap(col, pivot);
            b.swap(col, pivot);
        }

        let diag = a[col][col];
        for j in col..n {
            a[col][j] /= diag;
        }
        b[col] /= diag;

        for r in 0..n {
            if r == col {
                continue;
            }
            let factor = a[r][col];
            if factor.abs() < 1e-20 {
                continue;
            }
            for j in col..n {
                a[r][j] -= factor * a[col][j];
            }
            b[r] -= factor * b[col];
        }
    }
    Some(b)
}

fn fit_linear_model(rows: &[ResearchRow], ridge: f64) -> Option<Vec<f64>> {
    if rows.len() < 20 {
        return None;
    }
    // X = [1, mom5, mom20, vol20, rank_mom5_cs, rank_mom20_cs, rank_vol20_cs, vol_regime]
    let dim = 8_usize;
    let mut xtx = vec![vec![0.0_f64; dim]; dim];
    let mut xty = vec![0.0_f64; dim];
    for r in rows {
        let x = [
            1.0,
            r.mom5,
            r.mom20,
            r.vol20,
            r.rank_mom5_cs,
            r.rank_mom20_cs,
            r.rank_vol20_cs,
            r.vol_regime,
        ];
        for i in 0..dim {
            xty[i] += x[i] * r.y_oc_next;
            for j in 0..dim {
                xtx[i][j] += x[i] * x[j];
            }
        }
    }
    // L2 regularization (ridge), skip intercept term.
    for i in 1..dim {
        xtx[i][i] += ridge.max(0.0);
    }
    solve_linear_system(xtx, xty)
}

fn walkforward_score_proxy(
    prepared: &[PreparedSymbol],
    quantile: f64,
    short_enabled: bool,
    train_days: usize,
    test_days: usize,
) -> Option<WalkForwardSummary> {
    let n = prepared.first()?.closes.len();
    if n < (train_days + 2) || test_days == 0 {
        return None;
    }

    let mut pnls = Vec::<f64>::new();
    let mut folds = 0usize;
    let mut start = train_days.max(20);
    while start < n - 1 {
        let end = (start + test_days).min(n - 1);
        let mut fold_has_day = false;
        for i in start..end {
            let sc = match scores_at_day(prepared, i) {
                Some(v) => v,
                None => continue,
            };
            let (_, sigs) = signals_from_scores(&sc, quantile, short_enabled);
            if sigs.is_empty() {
                continue;
            }
            let mut pnl = 0.0;
            let mut used = false;
            for s in &sigs {
                let p = match prepared.iter().find(|x| x.symbol == s.symbol) {
                    Some(v) => v,
                    None => continue,
                };
                let y = match forward_oc_return(p.opens[i + 1], p.closes[i + 1]) {
                    Some(v) => v,
                    None => continue,
                };
                used = true;
                pnl += if s.side == "SHORT" { -s.weight * y } else { s.weight * y };
            }
            if used {
                pnls.push(pnl);
                fold_has_day = true;
            }
        }
        if fold_has_day {
            folds += 1;
        }
        start += test_days;
    }

    if pnls.is_empty() {
        return None;
    }
    let m = mean(&pnls);
    let sd = std_sample(&pnls, m);
    let sharpe = if sd > 1e-12 {
        (m / sd) * (252_f64).sqrt()
    } else {
        0.0
    };
    let mut cum = 0.0;
    let mut peak = 0.0_f64;
    let mut max_dd = 0.0_f64;
    for &x in &pnls {
        cum += x;
        peak = peak.max(cum);
        max_dd = max_dd.min(cum - peak);
    }
    Some(WalkForwardSummary {
        folds,
        trading_days: pnls.len(),
        mean_daily: m,
        vol_daily: sd,
        sharpe_252: sharpe,
        cumulative_return: pnls.iter().sum(),
        max_drawdown: max_dd,
    })
}

pub async fn compute_phase2_research(
    client: &IbkrOAuthClient,
    symbol_conids: &[(String, u64)],
    quantile: f64,
    short_enabled: bool,
    train_days: usize,
    test_days: usize,
    sample_rows: usize,
) -> Phase2ResearchResponse {
    let mut bars_by_symbol: HashMap<String, Vec<HistoryBar>> = HashMap::new();
    for (sym, conid) in symbol_conids {
        match client.fetch_history(*conid, "1y", "1d").await {
            Ok(bars) => {
                bars_by_symbol.insert(sym.clone(), bars);
            }
            Err(e) => tracing::warn!("phase2-research: history failed for {}: {}", sym, e),
        }
        tokio::time::sleep(std::time::Duration::from_millis(75)).await;
    }
    let symbols_with_history = bars_by_symbol.len();
    if symbols_with_history < 3 {
        return Phase2ResearchResponse {
            model_scaffold: "phase2_proxy_score_walkforward",
            label: "ln(C[t+1]/O[t+1])",
            data_available: false,
            reason: Some("insufficient_symbols_with_history".to_string()),
            symbols_with_history,
            overlapping_days: 0,
            train_days,
            test_days,
            quantile,
            short_enabled,
            row_count: 0,
            rows_sample: vec![],
            walkforward: None,
        };
    }

    let ts = aligned_timestamps(&bars_by_symbol);
    let overlapping_days = ts.len();
    if ts.len() < 22 {
        return Phase2ResearchResponse {
            model_scaffold: "phase2_proxy_score_walkforward",
            label: "ln(C[t+1]/O[t+1])",
            data_available: false,
            reason: Some("insufficient_overlapping_history_for_features".to_string()),
            symbols_with_history,
            overlapping_days,
            train_days,
            test_days,
            quantile,
            short_enabled,
            row_count: 0,
            rows_sample: vec![],
            walkforward: None,
        };
    }

    let mut prepared: Vec<PreparedSymbol> = Vec::new();
    for (sym, bars) in bars_by_symbol {
        if let Some(p) = prepare_symbol(sym, bars, &ts) {
            prepared.push(p);
        }
    }
    if prepared.len() < 3 {
        return Phase2ResearchResponse {
            model_scaffold: "phase2_proxy_score_walkforward",
            label: "ln(C[t+1]/O[t+1])",
            data_available: false,
            reason: Some("not_enough_symbols_after_alignment".to_string()),
            symbols_with_history,
            overlapping_days,
            train_days,
            test_days,
            quantile,
            short_enabled,
            row_count: 0,
            rows_sample: vec![],
            walkforward: None,
        };
    }

    let mut rows = Vec::<ResearchRow>::new();
    let n = ts.len();
    for i in 20..(n - 1) {
        let day_scores = match scores_at_day(&prepared, i) {
            Some(v) => v,
            None => continue,
        };
        for row in day_scores {
            let p = match prepared.iter().find(|x| x.symbol == row.symbol) {
                Some(v) => v,
                None => continue,
            };
            let y = match forward_oc_return(p.opens[i + 1], p.closes[i + 1]) {
                Some(v) => v,
                None => continue,
            };
            rows.push(ResearchRow {
                ts: ts[i],
                symbol: row.symbol,
                mom5: row.mom5,
                mom20: row.mom20,
                vol20: row.vol20,
                rank_mom5_cs: row.rank_mom5_cs,
                rank_mom20_cs: row.rank_mom20_cs,
                rank_vol20_cs: row.rank_vol20_cs,
                vol_regime: row.vol_regime,
                score: row.score,
                y_oc_next: y,
            });
        }
    }
    rows.sort_by(|a, b| b.ts.cmp(&a.ts).then_with(|| a.symbol.cmp(&b.symbol)));
    let row_count = rows.len();
    // `sample_rows=0` means "return all rows" (useful for model training).
    let rows_sample = if sample_rows == 0 {
        rows
    } else {
        rows.into_iter().take(sample_rows).collect::<Vec<_>>()
    };
    let walkforward = walkforward_score_proxy(
        &prepared,
        quantile,
        short_enabled,
        train_days.max(60),
        test_days.max(5),
    );

    Phase2ResearchResponse {
        model_scaffold: "phase2_proxy_score_walkforward",
        label: "ln(C[t+1]/O[t+1])",
        data_available: row_count > 0,
        reason: if row_count > 0 {
            None
        } else {
            Some("no_rows_generated".to_string())
        },
        symbols_with_history,
        overlapping_days,
        train_days,
        test_days,
        quantile,
        short_enabled,
        row_count,
        rows_sample,
        walkforward,
    }
}

pub async fn compute_phase2_model_signals(
    client: &IbkrOAuthClient,
    symbol_conids: &[(String, u64)],
    quantile: f64,
    short_enabled: bool,
    train_days: usize,
) -> Phase2ModelSignalsResponse {
    let mut bars_by_symbol: HashMap<String, Vec<HistoryBar>> = HashMap::new();
    for (sym, conid) in symbol_conids {
        match client.fetch_history(*conid, "1y", "1d").await {
            Ok(bars) => {
                bars_by_symbol.insert(sym.clone(), bars);
            }
            Err(e) => tracing::warn!("phase2-model: history failed for {}: {}", sym, e),
        }
        tokio::time::sleep(std::time::Duration::from_millis(75)).await;
    }
    let symbols_with_history = bars_by_symbol.len();
    if symbols_with_history < 3 {
        return Phase2ModelSignalsResponse {
            model: "linear_regression_enhanced_v2",
            label: "ln(C[t+1]/O[t+1])",
            data_available: false,
            reason: Some("insufficient_symbols_with_history".to_string()),
            as_of_ts: None,
            symbols_with_history,
            overlapping_days: 0,
            training_rows: 0,
            quantile,
            short_enabled,
            publish_signals: false,
            coefficients: None,
            model_health: None,
            features: vec![],
            signals: vec![],
        };
    }

    let ts = aligned_timestamps(&bars_by_symbol);
    let overlapping_days = ts.len();
    if ts.len() < 23 {
        return Phase2ModelSignalsResponse {
            model: "linear_regression_enhanced_v2",
            label: "ln(C[t+1]/O[t+1])",
            data_available: false,
            reason: Some("insufficient_overlapping_history_for_features".to_string()),
            as_of_ts: None,
            symbols_with_history,
            overlapping_days,
            training_rows: 0,
            quantile,
            short_enabled,
            publish_signals: false,
            coefficients: None,
            model_health: None,
            features: vec![],
            signals: vec![],
        };
    }

    let mut prepared: Vec<PreparedSymbol> = Vec::new();
    for (sym, bars) in bars_by_symbol {
        if let Some(p) = prepare_symbol(sym, bars, &ts) {
            prepared.push(p);
        }
    }
    if prepared.len() < 3 {
        return Phase2ModelSignalsResponse {
            model: "linear_regression_enhanced_v2",
            label: "ln(C[t+1]/O[t+1])",
            data_available: false,
            reason: Some("not_enough_symbols_after_alignment".to_string()),
            as_of_ts: None,
            symbols_with_history,
            overlapping_days,
            training_rows: 0,
            quantile,
            short_enabled,
            publish_signals: false,
            coefficients: None,
            model_health: None,
            features: vec![],
            signals: vec![],
        };
    }

    // Choose prediction day as the latest usable full-feature day; this avoids
    // failing on partial latest bars and prevents training leakage.
    let Some((decision_i, day_scores_for_prediction)) =
        latest_usable_decision_day(&prepared, ts.len())
    else {
        return Phase2ModelSignalsResponse {
            model: "linear_regression_enhanced_v2",
            label: "ln(C[t+1]/O[t+1])",
            data_available: false,
            reason: Some("feature_computation_unavailable".to_string()),
            as_of_ts: None,
            symbols_with_history,
            overlapping_days,
            training_rows: 0,
            quantile,
            short_enabled,
            publish_signals: false,
            coefficients: None,
            model_health: None,
            features: vec![],
            signals: vec![],
        };
    };

    // Build train rows from historical dates except the chosen decision day.
    let mut train_rows = Vec::<ResearchRow>::new();
    let train_start = 20.max(decision_i.saturating_sub(train_days));
    for i in train_start..decision_i {
        let day_scores = match scores_at_day(&prepared, i) {
            Some(v) => v,
            None => continue,
        };
        for row in day_scores {
            let p = match prepared.iter().find(|x| x.symbol == row.symbol) {
                Some(v) => v,
                None => continue,
            };
            let y = match forward_oc_return(p.opens[i + 1], p.closes[i + 1]) {
                Some(v) => v,
                None => continue,
            };
            train_rows.push(ResearchRow {
                ts: ts[i],
                symbol: row.symbol,
                mom5: row.mom5,
                mom20: row.mom20,
                vol20: row.vol20,
                rank_mom5_cs: row.rank_mom5_cs,
                rank_mom20_cs: row.rank_mom20_cs,
                rank_vol20_cs: row.rank_vol20_cs,
                vol_regime: row.vol_regime,
                score: row.score,
                y_oc_next: y,
            });
        }
    }
    let training_rows = train_rows.len();
    let train_window_days = {
        let mut d = std::collections::HashSet::<u64>::new();
        for r in &train_rows {
            d.insert(r.ts);
        }
        d.len()
    };
    let beta = match fit_linear_model(&train_rows, 1e-4) {
        Some(v) => v,
        None => {
            return Phase2ModelSignalsResponse {
                model: "linear_regression_enhanced_v2",
                label: "ln(C[t+1]/O[t+1])",
                data_available: false,
                reason: Some("model_fit_failed".to_string()),
                as_of_ts: None,
                symbols_with_history,
                overlapping_days,
                training_rows,
                quantile,
                short_enabled,
                publish_signals: false,
                coefficients: None,
                model_health: None,
                features: vec![],
                signals: vec![],
            };
        }
    };

    // Predict y(t) on latest usable decision-day features.
    let day_scores = day_scores_for_prediction;

    let predicted_scores = day_scores
        .into_iter()
        .map(|row| {
            let pred = beta[0]
                + beta[1] * row.mom5
                + beta[2] * row.mom20
                + beta[3] * row.vol20
                + beta[4] * row.rank_mom5_cs
                + beta[5] * row.rank_mom20_cs
                + beta[6] * row.rank_vol20_cs
                + beta[7] * row.vol_regime;
            DayFeatureRow {
                score: pred,
                ..row
            }
        })
        .collect::<Vec<_>>();

    let (features, mut signals) = signals_from_scores(&predicted_scores, quantile, short_enabled);
    let model_health = ModelHealth {
        feature_row_count: predicted_scores.len(),
        nan_or_inf_count: count_non_finite_values(&train_rows, &beta, &predicted_scores),
        coef_norm: coef_l2_norm(&beta),
        train_window_days,
    };

    // Guardrail 1: cap concentration and renormalize each side.
    for side in ["LONG", "SHORT"] {
        let idx: Vec<usize> = signals
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.side == side { Some(i) } else { None })
            .collect();
        if idx.is_empty() {
            continue;
        }
        for &i in &idx {
            signals[i].weight = signals[i].weight.min(MODEL_MAX_WEIGHT_PER_SYMBOL);
        }
        let total: f64 = idx.iter().map(|&i| signals[i].weight).sum();
        if total > 1e-12 {
            for &i in &idx {
                signals[i].weight /= total;
            }
        }
    }

    // Guardrail 2: publish only if diagnostics pass.
    let mut publish_reason: Option<String> = None;
    if model_health.feature_row_count < MODEL_MIN_FEATURE_ROWS {
        publish_reason = Some("publish_blocked_low_feature_rows".to_string());
    } else if model_health.nan_or_inf_count > 0 {
        publish_reason = Some("publish_blocked_non_finite_features".to_string());
    } else if model_health.coef_norm > MODEL_MAX_COEF_NORM {
        publish_reason = Some("publish_blocked_coef_norm_outlier".to_string());
    } else if features.len() < MODEL_MIN_UNIVERSE_SIZE {
        publish_reason = Some("publish_blocked_small_universe".to_string());
    } else {
        let long_n = signals.iter().filter(|s| s.side == "LONG").count();
        let short_n = signals.iter().filter(|s| s.side == "SHORT").count();
        if long_n == 0 || (short_enabled && short_n == 0) {
            publish_reason = Some("publish_blocked_empty_signal_side".to_string());
        }
    }

    let publish_signals = publish_reason.is_none();
    if !publish_signals {
        signals.clear();
    }
    Phase2ModelSignalsResponse {
        model: "linear_regression_enhanced_v2",
        label: "ln(C[t+1]/O[t+1])",
        data_available: publish_signals,
        reason: publish_reason,
        as_of_ts: Some(ts[decision_i]),
        symbols_with_history,
        overlapping_days,
        training_rows,
        quantile,
        short_enabled,
        publish_signals,
        coefficients: Some(LinearModelCoefficients {
            intercept: beta[0],
            mom5: beta[1],
            mom20: beta[2],
            vol20: beta[3],
            rank_mom5_cs: beta[4],
            rank_mom20_cs: beta[5],
            rank_vol20_cs: beta[6],
            vol_regime: beta[7],
        }),
        model_health: Some(model_health),
        features,
        signals,
    }
}
