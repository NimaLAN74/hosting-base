//! Phase-1 daily momentum strategy: decision at close of day `t`, execution next session
//! open → close (`t+1`). Label for research: `y(t) = ln(C_{t+1}/O_{t+1})`.
//!
//! Features (no lookahead): 5d/20d momentum on closes, 20d realized vol of daily log returns.
//! IBKR history bars do not include volume — volume surprise is omitted until we add a feed.

use crate::ibkr::{HistoryBar, IbkrOAuthClient};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Top/bottom fraction of the cross-section for long/short (20% each → 2 names on a 10-name list).
pub const DEFAULT_QUANTILE: f64 = 0.2;

#[derive(Clone, Debug, Serialize)]
pub struct SymbolFeatures {
    pub symbol: String,
    pub mom5: f64,
    pub mom20: f64,
    pub vol20: f64,
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
        let ts: HashSet<u64> = bars.iter().map(|b| b.t).collect();
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
    let map: HashMap<u64, &HistoryBar> = bars.iter().map(|b| (b.t, b)).collect();
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

/// Cross-sectional composite scores at day index `i`.
fn scores_at_day(prepared: &[PreparedSymbol], i: usize) -> Option<Vec<(String, f64, f64, f64, f64)>> {
    let mut rows = Vec::new();
    for p in prepared {
        let f = features_at_index(&p.closes, &p.rets, i)?;
        rows.push((p.symbol.clone(), f.0, f.1, f.2, 0.0));
    }
    if rows.is_empty() {
        return None;
    }
    let mom5: Vec<f64> = rows.iter().map(|(_, m5, _, _, _)| *m5).collect();
    let mom20: Vec<f64> = rows.iter().map(|(_, _, m20, _, _)| *m20).collect();
    let vols: Vec<f64> = rows.iter().map(|(_, _, _, v, _)| *v).collect();
    let z5 = z_scores(&mom5);
    let z20 = z_scores(&mom20);
    let zv = z_scores(&vols);
    let mut out = Vec::new();
    for k in 0..rows.len() {
        let score = z5[k] + z20[k] - zv[k];
        out.push((
            rows[k].0.clone(),
            rows[k].1,
            rows[k].2,
            rows[k].3,
            score,
        ));
    }
    Some(out)
}

/// Rank by score; long top `quantile`, short bottom `quantile`; vol-parity weights within side.
fn signals_from_scores(
    scores: &[(String, f64, f64, f64, f64)],
    quantile: f64,
    short_enabled: bool,
) -> (Vec<SymbolFeatures>, Vec<DailySignal>) {
    let n = scores.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        scores[a]
            .4
            .partial_cmp(&scores[b].4)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let k = ((n as f64 * quantile).ceil() as usize).max(1).min(n);

    let long_idx: Vec<usize> = order.iter().rev().take(k).copied().collect();
    let short_idx: Vec<usize> = order.iter().take(k).copied().collect();

    let inv_vol = |i: usize| 1.0 / scores[i].3.max(1e-8);
    let long_w_raw: f64 = long_idx.iter().map(|&i| inv_vol(i)).sum();
    let short_w_raw: f64 = short_idx.iter().map(|&i| inv_vol(i)).sum();

    let mut features: Vec<SymbolFeatures> = scores
        .iter()
        .map(|(sym, m5, m20, v, sc)| SymbolFeatures {
            symbol: sym.clone(),
            mom5: *m5,
            mom20: *m20,
            vol20: *v,
            score: *sc,
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
            symbol: scores[i].0.clone(),
            side: "LONG",
            weight: w,
            score: scores[i].4,
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
                symbol: scores[i].0.clone(),
                side: "SHORT",
                weight: w,
                score: scores[i].4,
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
                bars_by_symbol.insert(sym.clone(), bars);
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

    let ts = aligned_timestamps(&bars_by_symbol);
    let overlapping_days = ts.len();
    if ts.is_empty() {
        return empty_daily_response(
            requested_universe,
            quantile,
            short_enabled,
            symbols_with_history,
            0,
            "no_overlapping_trading_days".to_string(),
        );
    }
    if ts.len() < 22 {
        return empty_daily_response(
            requested_universe,
            quantile,
            short_enabled,
            symbols_with_history,
            overlapping_days,
            "insufficient_overlapping_history_for_features".to_string(),
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

    let last_i = ts.len() - 1;
    let Some(sc) = scores_at_day(&prepared, last_i) else {
        tracing::info!(
            "daily-signals: feature computation skipped (e.g. holiday bar without full lookback)"
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
        as_of_ts: Some(ts[last_i]),
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
