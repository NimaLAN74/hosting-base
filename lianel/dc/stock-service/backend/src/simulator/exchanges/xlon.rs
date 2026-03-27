use super::ExchangeAdapter;

pub fn adapter() -> ExchangeAdapter {
    ExchangeAdapter {
        code: "XLON",
        session_open_utc: "08:00",
        session_close_utc: "16:30",
        fee_bps: 2.1,
        slippage_bps: 4.8,
        latency_ms: 95,
    }
}

