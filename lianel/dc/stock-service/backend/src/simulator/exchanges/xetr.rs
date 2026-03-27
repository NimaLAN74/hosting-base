use super::ExchangeAdapter;

pub fn adapter() -> ExchangeAdapter {
    ExchangeAdapter {
        code: "XETR",
        session_open_utc: "08:00",
        session_close_utc: "16:30",
        fee_bps: 1.8,
        ibkr_commission_bps: 0.45,
        exchange_fee_bps: 0.55,
        clearing_fee_bps: 0.18,
        regulatory_fee_bps: 0.09,
        fx_fee_bps: 0.15,
        tax_bps: 0.0,
        spread_bps: 2.4,
        slippage_bps: 4.0,
        market_impact_bps: 2.8,
        borrow_fee_bps: 1.6,
        depth_notional_usd: 18_000.0,
        latency_ms: 85,
        auction_window: true,
        auction_extra_latency_ms: 180,
        short_borrow_available: true,
    }
}

