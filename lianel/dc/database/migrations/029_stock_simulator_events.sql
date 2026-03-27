-- Stock simulator event-sourced storage (v1).
-- Note: migration number starts at 029 because 024-028 are already in use.

BEGIN;

CREATE SCHEMA IF NOT EXISTS stock_monitoring;

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_run (
  run_id TEXT PRIMARY KEY,
  status TEXT NOT NULL,
  created_at_ts BIGINT NOT NULL,
  started_at_ts BIGINT,
  finished_at_ts BIGINT,
  days_requested INTEGER NOT NULL,
  symbols_count INTEGER NOT NULL,
  exchanges JSONB NOT NULL DEFAULT '[]'::jsonb,
  initial_capital_usd DOUBLE PRECISION NOT NULL,
  ending_equity_usd DOUBLE PRECISION,
  pnl_usd DOUBLE PRECISION
);

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_event_log (
  id BIGSERIAL PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES stock_monitoring.sim_run(run_id) ON DELETE CASCADE,
  event_id TEXT NOT NULL,
  ts BIGINT NOT NULL,
  kind TEXT NOT NULL,
  exchange TEXT,
  payload JSONB NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX IF NOT EXISTS idx_sim_event_log_run_ts ON stock_monitoring.sim_event_log(run_id, ts);
CREATE INDEX IF NOT EXISTS idx_sim_event_log_kind ON stock_monitoring.sim_event_log(kind);

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_decision_trace (
  id BIGSERIAL PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES stock_monitoring.sim_run(run_id) ON DELETE CASCADE,
  decision_id TEXT NOT NULL,
  decision_ts BIGINT NOT NULL,
  symbol TEXT NOT NULL,
  exchange TEXT NOT NULL,
  side TEXT NOT NULL,
  weight DOUBLE PRECISION NOT NULL,
  hybrid_score DOUBLE PRECISION NOT NULL,
  features JSONB NOT NULL DEFAULT '{}'::jsonb,
  rationale JSONB NOT NULL DEFAULT '[]'::jsonb
);
CREATE INDEX IF NOT EXISTS idx_sim_decision_trace_run_decision ON stock_monitoring.sim_decision_trace(run_id, decision_id);

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_fill_ledger (
  id BIGSERIAL PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES stock_monitoring.sim_run(run_id) ON DELETE CASCADE,
  decision_id TEXT NOT NULL,
  exec_ts BIGINT NOT NULL,
  symbol TEXT NOT NULL,
  exchange TEXT NOT NULL,
  side TEXT NOT NULL,
  qty_notional_usd DOUBLE PRECISION NOT NULL,
  open_px DOUBLE PRECISION NOT NULL,
  close_px DOUBLE PRECISION NOT NULL,
  ret_simple DOUBLE PRECISION NOT NULL,
  fee_usd DOUBLE PRECISION NOT NULL,
  slippage_usd DOUBLE PRECISION NOT NULL,
  pnl_usd DOUBLE PRECISION NOT NULL,
  latency_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sim_fill_ledger_run_exec ON stock_monitoring.sim_fill_ledger(run_id, exec_ts);

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_portfolio_curve (
  id BIGSERIAL PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES stock_monitoring.sim_run(run_id) ON DELETE CASCADE,
  ts BIGINT NOT NULL,
  equity_usd DOUBLE PRECISION NOT NULL,
  pnl_cum_usd DOUBLE PRECISION NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sim_portfolio_curve_run_ts ON stock_monitoring.sim_portfolio_curve(run_id, ts);

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_bias_findings (
  id BIGSERIAL PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES stock_monitoring.sim_run(run_id) ON DELETE CASCADE,
  severity TEXT NOT NULL,
  code TEXT NOT NULL,
  message TEXT NOT NULL,
  details JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at_ts BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT)
);
CREATE INDEX IF NOT EXISTS idx_sim_bias_findings_run ON stock_monitoring.sim_bias_findings(run_id);

COMMIT;

