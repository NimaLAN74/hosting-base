-- Migration: Stock Monitoring – Provider per symbol (watchlist, daily, intraday)
-- Date: 2026-03
-- Description: Each watchlist item and price history row is tied to a quote provider.
-- Watchlist: (watchlist_id, symbol, provider) unique. Daily/Intraday: add provider column and use in PK/reads.

-- 1) watchlist_items: add provider, allow same symbol with different providers
ALTER TABLE stock_monitoring.watchlist_items
  ADD COLUMN IF NOT EXISTS provider VARCHAR(32) NOT NULL DEFAULT 'yahoo';

ALTER TABLE stock_monitoring.watchlist_items
  DROP CONSTRAINT IF EXISTS watchlist_items_watchlist_id_symbol_key;

CREATE UNIQUE INDEX IF NOT EXISTS idx_watchlist_items_watchlist_symbol_provider
  ON stock_monitoring.watchlist_items (watchlist_id, symbol, provider);

COMMENT ON COLUMN stock_monitoring.watchlist_items.provider IS 'Quote provider for this symbol: yahoo, finnhub, alpaca, stooq, alpha_vantage';

-- 2) price_history_daily: add provider, new composite PK
ALTER TABLE stock_monitoring.price_history_daily
  ADD COLUMN IF NOT EXISTS provider VARCHAR(32) NOT NULL DEFAULT 'yahoo';

-- Drop old PK (symbol, trade_date) and add (symbol, provider, trade_date)
ALTER TABLE stock_monitoring.price_history_daily
  DROP CONSTRAINT IF EXISTS price_history_daily_pkey;

ALTER TABLE stock_monitoring.price_history_daily
  ADD PRIMARY KEY (symbol, provider, trade_date);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_price_history_daily_symbol_provider
  ON stock_monitoring.price_history_daily(symbol, provider);

-- 3) price_history_intraday: add provider column
ALTER TABLE stock_monitoring.price_history_intraday
  ADD COLUMN IF NOT EXISTS provider VARCHAR(32) NOT NULL DEFAULT 'yahoo';

CREATE INDEX IF NOT EXISTS idx_price_history_intraday_symbol_provider_observed
  ON stock_monitoring.price_history_intraday(symbol, provider, observed_at);

COMMENT ON COLUMN stock_monitoring.price_history_daily.provider IS 'Quote provider for this series';
COMMENT ON COLUMN stock_monitoring.price_history_intraday.provider IS 'Quote provider for this series';
