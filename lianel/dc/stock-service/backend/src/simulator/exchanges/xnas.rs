use super::ExchangeAdapter;

pub fn adapter() -> ExchangeAdapter {
    ExchangeAdapter {
        code: "XNAS",
        session_open_utc: "14:30",
        session_close_utc: "21:00",
        fee_bps: 1.2,
        slippage_bps: 3.5,
        latency_ms: 70,
    }
}

