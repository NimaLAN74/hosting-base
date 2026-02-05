-- Migration: G8 – Vanta Control Set alignment (external_id on controls and requirements)
-- Allows mapping comp_ai controls/requirements to external IDs (e.g. Vanta Control Set).
-- Date: 2026-01

-- Controls: optional external identifier (e.g. Vanta control ID)
ALTER TABLE comp_ai.controls
  ADD COLUMN IF NOT EXISTS external_id VARCHAR(255);

COMMENT ON COLUMN comp_ai.controls.external_id IS 'External control ID (e.g. Vanta Control Set) for alignment';

-- Requirements: optional external identifier (e.g. Vanta requirement ID)
ALTER TABLE comp_ai.requirements
  ADD COLUMN IF NOT EXISTS external_id VARCHAR(255);

COMMENT ON COLUMN comp_ai.requirements.external_id IS 'External requirement ID (e.g. Vanta Control Set) for alignment';
