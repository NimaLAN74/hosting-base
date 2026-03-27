use super::ExchangeAdapter;

pub fn adapter() -> ExchangeAdapter {
    ExchangeAdapter {
        code: "XLON",
        session_open_utc: "08:00",
        session_close_utc: "16:30",
        fee_bps: 2.1,
        ibkr_commission_bps: 0.48,
        exchange_fee_bps: 0.72,
        clearing_fee_bps: 0.22,
        regulatory_fee_bps: 0.12,
        fx_fee_bps: 0.15,
        tax_bps: 0.35,
        spread_bps: 2.9,
        slippage_bps: 4.8,
        market_impact_bps: 3.2,
        borrow_fee_bps: 1.9,
        depth_notional_usd: 14_000.0,
        latency_ms: 95,
        auction_window: true,
        auction_extra_latency_ms: 220,
        short_borrow_available: false,
    }
}

