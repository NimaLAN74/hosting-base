-- Migration: Comp-AI controls, evidence, and framework mappings (Phase 4)
-- Date: 2026-02
-- Aligned with COMP-AI-MULTIFRAMEWORK-SUPPORT.md

-- Frameworks (e.g. SOC 2, ISO 27001)
CREATE TABLE IF NOT EXISTS comp_ai.frameworks (
    id BIGSERIAL PRIMARY KEY,
    slug VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(200) NOT NULL,
    version VARCHAR(50),
    scope TEXT,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Framework requirements (e.g. SOC 2 CC6.1, ISO A.9.4.2)
CREATE TABLE IF NOT EXISTS comp_ai.requirements (
    id BIGSERIAL PRIMARY KEY,
    framework_id BIGINT NOT NULL REFERENCES comp_ai.frameworks(id) ON DELETE CASCADE,
    code VARCHAR(50) NOT NULL,
    title VARCHAR(500),
    description TEXT,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(framework_id, code)
);

-- Internal controls (one control can satisfy many requirements)
CREATE TABLE IF NOT EXISTS comp_ai.controls (
    id BIGSERIAL PRIMARY KEY,
    internal_id VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(500) NOT NULL,
    description TEXT,
    category VARCHAR(100),
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Control <-> requirement mapping (many-to-many)
CREATE TABLE IF NOT EXISTS comp_ai.control_requirements (
    control_id BIGINT NOT NULL REFERENCES comp_ai.controls(id) ON DELETE CASCADE,
    requirement_id BIGINT NOT NULL REFERENCES comp_ai.requirements(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (control_id, requirement_id)
);

-- Evidence (artifacts linked to controls)
CREATE TABLE IF NOT EXISTS comp_ai.evidence (
    id BIGSERIAL PRIMARY KEY,
    control_id BIGINT NOT NULL REFERENCES comp_ai.controls(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL,
    source VARCHAR(500),
    description TEXT,
    link_url TEXT,
    collected_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_comp_ai_requirements_framework ON comp_ai.requirements(framework_id);
CREATE INDEX IF NOT EXISTS idx_comp_ai_control_requirements_control ON comp_ai.control_requirements(control_id);
CREATE INDEX IF NOT EXISTS idx_comp_ai_control_requirements_requirement ON comp_ai.control_requirements(requirement_id);
CREATE INDEX IF NOT EXISTS idx_comp_ai_evidence_control ON comp_ai.evidence(control_id);
CREATE INDEX IF NOT EXISTS idx_comp_ai_evidence_collected_at ON comp_ai.evidence(collected_at DESC);

COMMENT ON TABLE comp_ai.frameworks IS 'Compliance frameworks (SOC 2, ISO 27001, etc.)';
COMMENT ON TABLE comp_ai.requirements IS 'Framework requirements (e.g. CC6.1, A.9.4.2)';
COMMENT ON TABLE comp_ai.controls IS 'Internal controls; one control can map to many requirements';
COMMENT ON TABLE comp_ai.control_requirements IS 'Control-to-requirement mapping (cross-framework)';
COMMENT ON TABLE comp_ai.evidence IS 'Evidence artifacts linked to controls';
