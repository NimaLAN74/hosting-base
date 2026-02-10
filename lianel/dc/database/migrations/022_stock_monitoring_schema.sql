-- Migration: Stock Monitoring Service Schema (EU markets MVP)
-- Date: 2026-01-23
-- Description: Schema and tables for watchlists, alerts, and audit. Scope: EU markets only for MVP.

CREATE SCHEMA IF NOT EXISTS stock_monitoring;

-- Watchlists: user-defined symbol lists (EU symbols; ISIN/ric per docs)
CREATE TABLE IF NOT EXISTS stock_monitoring.watchlists (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, name)
);

CREATE TABLE IF NOT EXISTS stock_monitoring.watchlist_items (
    id BIGSERIAL PRIMARY KEY,
    watchlist_id BIGINT NOT NULL REFERENCES stock_monitoring.watchlists(id) ON DELETE CASCADE,
    symbol VARCHAR(32) NOT NULL,
    mic VARCHAR(8),
    added_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (watchlist_id, symbol)
);

-- Price-based alerts (P0)
CREATE TABLE IF NOT EXISTS stock_monitoring.alerts (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    symbol VARCHAR(32) NOT NULL,
    mic VARCHAR(8),
    condition_type VARCHAR(32) NOT NULL,
    condition_value NUMERIC(20, 6),
    notified_at TIMESTAMP WITHOUT TIME ZONE,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_watchlists_user ON stock_monitoring.watchlists(user_id);
CREATE INDEX IF NOT EXISTS idx_stock_monitoring_watchlist_items_watchlist ON stock_monitoring.watchlist_items(watchlist_id);
CREATE INDEX IF NOT EXISTS idx_stock_monitoring_alerts_user ON stock_monitoring.alerts(user_id);
CREATE INDEX IF NOT EXISTS idx_stock_monitoring_alerts_symbol ON stock_monitoring.alerts(symbol);
CREATE INDEX IF NOT EXISTS idx_stock_monitoring_alerts_enabled ON stock_monitoring.alerts(enabled) WHERE enabled = true;

-- Audit log (P0.7): material actions
CREATE TABLE IF NOT EXISTS stock_monitoring.audit_log (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(255),
    action VARCHAR(64) NOT NULL,
    resource_type VARCHAR(64),
    resource_id VARCHAR(255),
    details JSONB,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_audit_user ON stock_monitoring.audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_stock_monitoring_audit_created ON stock_monitoring.audit_log(created_at DESC);

COMMENT ON SCHEMA stock_monitoring IS 'Stock exchange monitoring – EU markets MVP; shared DB, product-specific schema';
COMMENT ON TABLE stock_monitoring.watchlists IS 'User watchlists; scope EU symbols';
COMMENT ON TABLE stock_monitoring.alerts IS 'Price (and later technical) alerts; EU symbols';
COMMENT ON TABLE stock_monitoring.audit_log IS 'Immutable audit trail for alerts, watchlists, etc.';
