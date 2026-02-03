-- Seed: ISO 27001 framework with sample Annex A requirements; map existing controls (Phase 5)
-- Run after 011_comp_ai_seed_soc2.sql (controls already exist)

INSERT INTO comp_ai.frameworks (slug, name, version, scope)
VALUES ('iso27001', 'ISO 27001', '2022', 'Information security management system (ISMS), Annex A controls')
ON CONFLICT (slug) DO NOTHING;

-- ISO 27001 Annex A – sample requirements (access control, logging)
INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.9.4.1', 'Information access restriction',
  'Access to information and other associated assets shall be restricted in accordance with the access control policy.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.9.4.2', 'Secure log-on procedures',
  'Where required by the access control policy, access to systems and applications shall be controlled by a secure log-on procedure.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.12.4.1', 'Event logging',
  'Event logs recording user activities, exceptions, faults and other relevant events shall be produced, kept and regularly reviewed.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.12.4.3', 'Administrator and operator logs',
  'Administrator and operator activities shall be logged and the logs protected and regularly reviewed.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

-- Map existing controls to ISO 27001 requirements (cross-framework)
INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MFA-001' AND r.code = 'A.9.4.2' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'iso27001')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'A.9.4.1' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'iso27001')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'A.12.4.1' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'iso27001')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'A.12.4.3' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'iso27001')
ON CONFLICT (control_id, requirement_id) DO NOTHING;
