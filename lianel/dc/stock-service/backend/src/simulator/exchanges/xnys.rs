use super::ExchangeAdapter;

pub fn adapter() -> ExchangeAdapter {
    ExchangeAdapter {
        code: "XNYS",
        session_open_utc: "14:30",
        session_close_utc: "21:00",
        fee_bps: 1.4,
        spread_bps: 1.8,
        slippage_bps: 3.8,
        market_impact_bps: 2.4,
        borrow_fee_bps: 1.1,
        depth_notional_usd: 26_000.0,
        latency_ms: 75,
        auction_window: true,
        auction_extra_latency_ms: 140,
        short_borrow_available: true,
    }
}

