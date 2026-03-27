use super::ExchangeAdapter;

pub fn adapter() -> ExchangeAdapter {
    ExchangeAdapter {
        code: "XETR",
        session_open_utc: "08:00",
        session_close_utc: "16:30",
        fee_bps: 1.8,
        slippage_bps: 4.0,
        latency_ms: 85,
    }
}

