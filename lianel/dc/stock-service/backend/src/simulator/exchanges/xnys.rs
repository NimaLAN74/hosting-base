use super::ExchangeAdapter;

pub fn adapter() -> ExchangeAdapter {
    ExchangeAdapter {
        code: "XNYS",
        session_open_utc: "14:30",
        session_close_utc: "21:00",
        fee_bps: 1.4,
        slippage_bps: 3.8,
        latency_ms: 75,
    }
}

