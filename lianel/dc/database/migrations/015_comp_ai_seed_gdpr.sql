-- Seed: GDPR framework with sample articles; map existing controls (Phase 5)
-- Run after 011_comp_ai_seed_soc2.sql (controls already exist)

INSERT INTO comp_ai.frameworks (slug, name, version, scope)
VALUES ('gdpr', 'GDPR', 'Regulation (EU) 2016/679', 'EU data protection and privacy')
ON CONFLICT (slug) DO NOTHING;

-- GDPR – sample articles (security, breach, principles)
INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 32', 'Security of processing',
  'The controller and processor shall implement appropriate technical and organisational measures to ensure a level of security appropriate to the risk.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 33', 'Notification of a personal data breach',
  'In the case of a personal data breach, the controller shall without undue delay notify the supervisory authority.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 5(1)(f)', 'Integrity and confidentiality',
  'Personal data shall be processed in a manner that ensures appropriate security, including protection against unauthorised or unlawful processing.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 25', 'Data protection by design and by default',
  'The controller shall implement appropriate technical and organisational measures to implement data protection principles.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

-- Map existing controls to GDPR (cross-framework)
INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MFA-001' AND r.code = 'Art. 32' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MFA-001' AND r.code = 'Art. 5(1)(f)' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'Art. 32' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'Art. 25' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'Art. 32' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'Art. 33' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;
