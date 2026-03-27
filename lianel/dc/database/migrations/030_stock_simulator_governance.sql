-- Stock simulator governance + lifecycle detail schema (v2).

BEGIN;

CREATE SCHEMA IF NOT EXISTS stock_monitoring;

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_order_ledger (
  id BIGSERIAL PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES stock_monitoring.sim_run(run_id) ON DELETE CASCADE,
  order_id TEXT NOT NULL,
  decision_id TEXT NOT NULL,
  ts BIGINT NOT NULL,
  symbol TEXT NOT NULL,
  exchange TEXT NOT NULL,
  side TEXT NOT NULL,
  order_type TEXT NOT NULL,
  tif TEXT NOT NULL,
  qty_notional_usd DOUBLE PRECISION NOT NULL,
  intended_px DOUBLE PRECISION NOT NULL,
  status TEXT NOT NULL,
  filled_notional_usd DOUBLE PRECISION NOT NULL,
  remaining_notional_usd DOUBLE PRECISION NOT NULL,
  venue_latency_ms INTEGER NOT NULL,
  reasons JSONB NOT NULL DEFAULT '[]'::jsonb
);
CREATE INDEX IF NOT EXISTS idx_sim_order_ledger_run_ts ON stock_monitoring.sim_order_ledger(run_id, ts);
CREATE INDEX IF NOT EXISTS idx_sim_order_ledger_order_id ON stock_monitoring.sim_order_ledger(order_id);

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_risk_snapshot (
  id BIGSERIAL PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES stock_monitoring.sim_run(run_id) ON DELETE CASCADE,
  ts BIGINT NOT NULL,
  equity_usd DOUBLE PRECISION NOT NULL,
  cash_usd DOUBLE PRECISION NOT NULL,
  gross_exposure_usd DOUBLE PRECISION NOT NULL,
  leverage DOUBLE PRECISION NOT NULL,
  drawdown DOUBLE PRECISION NOT NULL,
  var_95_1d DOUBLE PRECISION NOT NULL,
  concentration_hhi DOUBLE PRECISION NOT NULL,
  benchmark_equity_usd DOUBLE PRECISION NOT NULL,
  relative_return DOUBLE PRECISION NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sim_risk_snapshot_run_ts ON stock_monitoring.sim_risk_snapshot(run_id, ts);

CREATE TABLE IF NOT EXISTS stock_monitoring.sim_readiness_eval (
  id BIGSERIAL PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES stock_monitoring.sim_run(run_id) ON DELETE CASCADE,
  status TEXT NOT NULL,
  min_days_required INTEGER NOT NULL,
  evaluated_days INTEGER NOT NULL,
  score DOUBLE PRECISION NOT NULL,
  pass BOOLEAN NOT NULL,
  criteria JSONB NOT NULL DEFAULT '{}'::jsonb,
  recommendation TEXT NOT NULL,
  created_at_ts BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sim_readiness_eval_run_unique ON stock_monitoring.sim_readiness_eval(run_id);

ALTER TABLE stock_monitoring.sim_fill_ledger
  ADD COLUMN IF NOT EXISTS ibkr_commission_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS exchange_fee_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS clearing_fee_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS regulatory_fee_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS fx_fee_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS tax_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS borrow_fee_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS market_impact_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS total_cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS buy_px DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS sell_px DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS buy_ts BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS sell_ts BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS buy_session_time_utc TEXT NOT NULL DEFAULT '',
  ADD COLUMN IF NOT EXISTS sell_session_time_utc TEXT NOT NULL DEFAULT '',
  ADD COLUMN IF NOT EXISTS market_data_source TEXT NOT NULL DEFAULT '';

COMMIT;

