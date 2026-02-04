-- Phase 6B: Automated tests per control (schema + seed)
-- One row per test definition per control; last_run_at / last_result updated when test runs.

CREATE TABLE IF NOT EXISTS comp_ai.control_tests (
    id BIGSERIAL PRIMARY KEY,
    control_id BIGINT NOT NULL REFERENCES comp_ai.controls(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    test_type VARCHAR(50) NOT NULL DEFAULT 'integration',
    schedule VARCHAR(100),
    last_run_at TIMESTAMP WITHOUT TIME ZONE,
    last_result VARCHAR(20),
    last_details TEXT,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(control_id, name)
);

CREATE INDEX IF NOT EXISTS idx_comp_ai_control_tests_control ON comp_ai.control_tests(control_id);
CREATE INDEX IF NOT EXISTS idx_comp_ai_control_tests_last_result ON comp_ai.control_tests(last_result);

COMMENT ON TABLE comp_ai.control_tests IS 'Automated test definitions per control; last run result stored on same row';

-- Seed: one or more tests per existing control
INSERT INTO comp_ai.control_tests (control_id, name, test_type, schedule, last_result, last_details)
SELECT c.id, 'MFA enabled in IdP', 'integration', '0 9 * * 1-5', 'pass', 'Keycloak conditional 2FA verified'
FROM comp_ai.controls c WHERE c.internal_id = 'CTL-MFA-001'
ON CONFLICT (control_id, name) DO NOTHING;

INSERT INTO comp_ai.control_tests (control_id, name, test_type, schedule, last_result, last_details)
SELECT c.id, 'Access review workflow exists', 'manual', NULL, NULL, NULL
FROM comp_ai.controls c WHERE c.internal_id = 'CTL-ACCESS-001'
ON CONFLICT (control_id, name) DO NOTHING;

INSERT INTO comp_ai.control_tests (control_id, name, test_type, schedule, last_result, last_details)
SELECT c.id, 'SIEM/log ingestion health', 'integration', '0 * * * *', 'pass', 'Last 24h events ingested'
FROM comp_ai.controls c WHERE c.internal_id = 'CTL-MON-001'
ON CONFLICT (control_id, name) DO NOTHING;

INSERT INTO comp_ai.control_tests (control_id, name, test_type, schedule, last_result, last_details)
SELECT c.id, 'Alerting pipeline run', 'integration', '0 */6 * * *', 'pass', 'Last run 2h ago'
FROM comp_ai.controls c WHERE c.internal_id = 'CTL-MON-001'
ON CONFLICT (control_id, name) DO NOTHING;

-- Grant to airflow role if it exists
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'airflow') THEN
    GRANT SELECT, INSERT, UPDATE, DELETE ON comp_ai.control_tests TO airflow;
    GRANT USAGE, SELECT ON SEQUENCE comp_ai.control_tests_id_seq TO airflow;
  END IF;
END $$;
