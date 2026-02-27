-- Migration: Stock Monitoring – Daily OHLC price history (space-efficient)
-- Date: 2026-02-27
-- Description: One row per symbol per calendar day (open, high, low, close). ~7 MB/year for 500 symbols.
-- See: lianel/dc/stock-monitoring/docs/STOCK-PRICE-HISTORY-DESIGN.md

CREATE TABLE IF NOT EXISTS stock_monitoring.price_history_daily (
    symbol VARCHAR(32) NOT NULL,
    trade_date DATE NOT NULL,
    open_price REAL NOT NULL,
    high_price REAL NOT NULL,
    low_price REAL NOT NULL,
    close_price REAL NOT NULL,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (symbol, trade_date)
);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_price_history_daily_symbol
    ON stock_monitoring.price_history_daily(symbol);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_price_history_daily_trade_date
    ON stock_monitoring.price_history_daily(trade_date DESC);

COMMENT ON TABLE stock_monitoring.price_history_daily IS 'Daily OHLC per symbol; upsert on each quote ingest for minimal storage';

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'airflow') THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON stock_monitoring.price_history_daily TO airflow';
  END IF;
END $$;
