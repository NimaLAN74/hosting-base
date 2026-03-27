use super::ExchangeAdapter;

pub fn adapter() -> ExchangeAdapter {
    ExchangeAdapter {
        code: "XNAS",
        session_open_utc: "14:30",
        session_close_utc: "21:00",
        fee_bps: 1.2,
        ibkr_commission_bps: 0.35,
        exchange_fee_bps: 0.25,
        clearing_fee_bps: 0.12,
        regulatory_fee_bps: 0.06,
        fx_fee_bps: 0.0,
        tax_bps: 0.0,
        spread_bps: 1.6,
        slippage_bps: 3.5,
        market_impact_bps: 2.2,
        borrow_fee_bps: 0.9,
        depth_notional_usd: 30_000.0,
        latency_ms: 70,
        auction_window: true,
        auction_extra_latency_ms: 120,
        short_borrow_available: true,
    }
}

