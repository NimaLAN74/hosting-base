-- Migration: Stock Monitoring Symbol Catalog (sync from providers)
-- Date: 2026-02-26
-- Description: Table for symbol lists synced periodically by DAG; backend serves GET /symbols from DB.

CREATE TABLE IF NOT EXISTS stock_monitoring.symbols (
    id BIGSERIAL PRIMARY KEY,
    provider VARCHAR(64) NOT NULL,
    symbol VARCHAR(64) NOT NULL,
    display_symbol VARCHAR(64) NOT NULL,
    description TEXT,
    type VARCHAR(64),
    exchange VARCHAR(16) NOT NULL DEFAULT '',
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (provider, symbol)
);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_symbols_provider_exchange
    ON stock_monitoring.symbols(provider, exchange);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_symbols_provider
    ON stock_monitoring.symbols(provider);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_symbols_search
    ON stock_monitoring.symbols(provider)
    INCLUDE (symbol, display_symbol, description);

COMMENT ON TABLE stock_monitoring.symbols IS 'Symbol catalog synced by DAG from Finnhub etc.; backend serves list from here';

-- Grant to airflow when present
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'airflow') THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON stock_monitoring.symbols TO airflow';
    EXECUTE 'GRANT USAGE, SELECT ON SEQUENCE stock_monitoring.symbols_id_seq TO airflow';
  END IF;
END $$;
