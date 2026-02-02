-- Seed: SOC 2 framework with sample requirements and controls (Phase 4)
-- Run after 010_comp_ai_controls_evidence.sql

INSERT INTO comp_ai.frameworks (slug, name, version, scope)
VALUES ('soc2', 'SOC 2', '2017', 'Trust Services Criteria: security, availability, processing integrity, confidentiality, privacy')
ON CONFLICT (slug) DO NOTHING;

-- Requirements (SOC 2 Trust Services Criteria – sample)
INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC6.1', 'Logical access – identity management',
  'The entity implements logical access security measures to protect against threats from sources outside its system boundaries.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC6.2', 'Logical access – access credentials',
  'Prior to issuing system credentials and access rights, the entity registers and authorizes new internal and external users.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC7.2', 'System monitoring',
  'The entity monitors the system for anomalies that are relevant to the security of its system.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

-- Internal controls (one control can satisfy multiple requirements)
INSERT INTO comp_ai.controls (internal_id, name, description, category)
VALUES
  ('CTL-MFA-001', 'Multi-factor authentication', 'MFA required for all user access to systems and applications.', 'Access control'),
  ('CTL-ACCESS-001', 'Access review process', 'Periodic review and recertification of user access rights.', 'Access control'),
  ('CTL-MON-001', 'Security monitoring', 'Continuous monitoring and alerting for security-relevant events.', 'Monitoring')
ON CONFLICT (internal_id) DO NOTHING;

-- Control–requirement mappings (cross-map)
INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MFA-001' AND r.code = 'CC6.1'
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'CC6.1'
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'CC6.2'
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'CC7.2'
ON CONFLICT (control_id, requirement_id) DO NOTHING;
