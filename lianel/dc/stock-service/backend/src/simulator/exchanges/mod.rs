pub mod xetr;
pub mod xlon;
pub mod xnas;
pub mod xnys;

#[derive(Debug, Clone)]
pub struct ExchangeAdapter {
    pub code: &'static str,
    pub session_open_utc: &'static str,
    pub session_close_utc: &'static str,
    pub fee_bps: f64,
    pub ibkr_commission_bps: f64,
    pub exchange_fee_bps: f64,
    pub clearing_fee_bps: f64,
    pub regulatory_fee_bps: f64,
    pub fx_fee_bps: f64,
    pub tax_bps: f64,
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub market_impact_bps: f64,
    pub borrow_fee_bps: f64,
    pub depth_notional_usd: f64,
    pub latency_ms: u64,
    pub auction_window: bool,
    pub auction_extra_latency_ms: u64,
    pub short_borrow_available: bool,
}

pub fn infer_exchange(symbol: &str) -> ExchangeAdapter {
    let s = symbol.trim().to_ascii_uppercase();
    // Lightweight symbol-to-exchange heuristic for v1 replay.
    // We will replace this with explicit MIC metadata in a later phase.
    if ["SHEL", "HSBC", "RIO", "BP"].contains(&s.as_str()) {
        return xlon::adapter();
    }
    if ["ASML", "NOK", "SONY", "TM"].contains(&s.as_str()) {
        return xetr::adapter();
    }
    if ["AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "INTC", "CSCO"]
        .contains(&s.as_str())
    {
        return xnas::adapter();
    }
    xnys::adapter()
}

