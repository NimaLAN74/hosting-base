-- Migration: Stock Monitoring – Ensure app role can read/write price history tables
-- Date: 2026-02
-- Description: Grant SELECT (and write where needed) to postgres and airflow so the app
-- can return price history and persist intraday regardless of which user it connects as.
-- Run after 025 and 026. Idempotent.

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'postgres') THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON stock_monitoring.price_history_daily TO postgres';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON stock_monitoring.price_history_intraday TO postgres';
    EXECUTE 'GRANT USAGE, SELECT ON SEQUENCE stock_monitoring.price_history_intraday_id_seq TO postgres';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'airflow') THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON stock_monitoring.price_history_daily TO airflow';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON stock_monitoring.price_history_intraday TO airflow';
    EXECUTE 'GRANT USAGE, SELECT ON SEQUENCE stock_monitoring.price_history_intraday_id_seq TO airflow';
  END IF;
END $$;
