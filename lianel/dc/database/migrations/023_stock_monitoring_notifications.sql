-- Migration: Stock Monitoring Notifications (P0.4 in-app channel)
-- Date: 2026-02-12
-- Description: Adds in-app notifications table for alert triggers.

CREATE TABLE IF NOT EXISTS stock_monitoring.notifications (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    alert_id BIGINT REFERENCES stock_monitoring.alerts(id) ON DELETE SET NULL,
    message TEXT NOT NULL,
    details JSONB,
    read_at TIMESTAMP WITHOUT TIME ZONE,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_notifications_user_created
    ON stock_monitoring.notifications(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_stock_monitoring_notifications_unread
    ON stock_monitoring.notifications(user_id, read_at)
    WHERE read_at IS NULL;

COMMENT ON TABLE stock_monitoring.notifications IS 'In-app user notifications for stock monitoring alerts';

-- Grant to airflow app role when present.
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'airflow') THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON stock_monitoring.notifications TO airflow';
  END IF;
END $$;
