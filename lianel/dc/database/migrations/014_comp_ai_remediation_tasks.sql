-- Migration: Comp-AI remediation tasks (Phase 5 – gaps workflow)
-- One task per control (e.g. assignee, due date, status) for controls with no evidence.

CREATE TABLE IF NOT EXISTS comp_ai.remediation_tasks (
    id BIGSERIAL PRIMARY KEY,
    control_id BIGINT NOT NULL REFERENCES comp_ai.controls(id) ON DELETE CASCADE,
    assigned_to VARCHAR(255),
    due_date DATE,
    status VARCHAR(20) NOT NULL DEFAULT 'open',
    notes TEXT,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(control_id)
);

CREATE INDEX IF NOT EXISTS idx_comp_ai_remediation_tasks_control ON comp_ai.remediation_tasks(control_id);
CREATE INDEX IF NOT EXISTS idx_comp_ai_remediation_tasks_status ON comp_ai.remediation_tasks(status);

COMMENT ON TABLE comp_ai.remediation_tasks IS 'Remediation workflow: one task per control (assignee, due date, status)';

-- Grant to airflow role if it exists (e.g. production); skip on CI where role may not exist
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'airflow') THEN
    GRANT SELECT, INSERT, UPDATE, DELETE ON comp_ai.remediation_tasks TO airflow;
    GRANT USAGE, SELECT ON SEQUENCE comp_ai.remediation_tasks_id_seq TO airflow;
  END IF;
END $$;
