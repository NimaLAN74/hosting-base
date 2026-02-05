-- Migration: G8 – External control set alignment (external_id on controls and requirements)
-- Allows mapping comp_ai controls/requirements to external framework/control-set IDs.
-- Date: 2026-01

-- Controls: optional external identifier (e.g. framework control ID)
ALTER TABLE comp_ai.controls
  ADD COLUMN IF NOT EXISTS external_id VARCHAR(255);

COMMENT ON COLUMN comp_ai.controls.external_id IS 'External control ID for alignment with frameworks/control sets';

-- Requirements: optional external identifier (e.g. framework requirement ID)
ALTER TABLE comp_ai.requirements
  ADD COLUMN IF NOT EXISTS external_id VARCHAR(255);

COMMENT ON COLUMN comp_ai.requirements.external_id IS 'External requirement ID for alignment with frameworks/control sets';
