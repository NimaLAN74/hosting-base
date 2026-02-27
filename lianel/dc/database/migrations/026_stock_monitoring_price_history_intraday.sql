-- Migration: Stock Monitoring – Intraday price cache (today’s points, rolled to daily at EOD)
-- Date: 2026-02-27
-- Description: Stores each quote during the day; EOD job aggregates to price_history_daily and truncates.
-- See: lianel/dc/stock-monitoring/docs/STOCK-PRICE-HISTORY-DESIGN.md

CREATE TABLE IF NOT EXISTS stock_monitoring.price_history_intraday (
    id BIGSERIAL PRIMARY KEY,
    symbol VARCHAR(32) NOT NULL,
    observed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    price REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_price_history_intraday_symbol_observed
    ON stock_monitoring.price_history_intraday(symbol, observed_at);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_price_history_intraday_observed
    ON stock_monitoring.price_history_intraday(observed_at);

COMMENT ON TABLE stock_monitoring.price_history_intraday IS 'Intraday points for current day; roll to price_history_daily at EOD then clear';

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'airflow') THEN
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON stock_monitoring.price_history_intraday TO airflow';
    EXECUTE 'GRANT USAGE, SELECT ON SEQUENCE stock_monitoring.price_history_intraday_id_seq TO airflow';
  END IF;
END $$;
