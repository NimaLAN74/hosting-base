-- Ensure demo seed data exists (run after 010; safe to run multiple times).
-- If Controls / Gaps / Evidence lists are empty in the UI, run this migration
-- (or the full run-comp-ai-migrations.sh) against the same DB the Comp-AI service uses.

-- Frameworks
INSERT INTO comp_ai.frameworks (slug, name, version, scope)
VALUES ('soc2', 'SOC 2', '2017', 'Trust Services Criteria: security, availability, processing integrity, confidentiality, privacy')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO comp_ai.frameworks (slug, name, version, scope)
VALUES ('iso27001', 'ISO 27001', '2022', 'Information security management system (ISMS), Annex A controls')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO comp_ai.frameworks (slug, name, version, scope)
VALUES ('gdpr', 'GDPR', 'Regulation (EU) 2016/679', 'EU data protection and privacy')
ON CONFLICT (slug) DO NOTHING;

-- SOC 2 requirements
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

-- Internal controls (3 demo controls)
INSERT INTO comp_ai.controls (internal_id, name, description, category)
VALUES
  ('CTL-MFA-001', 'Multi-factor authentication', 'MFA required for all user access to systems and applications.', 'Access control'),
  ('CTL-ACCESS-001', 'Access review process', 'Periodic review and recertification of user access rights.', 'Access control'),
  ('CTL-MON-001', 'Security monitoring', 'Continuous monitoring and alerting for security-relevant events.', 'Monitoring')
ON CONFLICT (internal_id) DO NOTHING;

-- SOC 2 control–requirement mappings
INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MFA-001' AND r.code = 'CC6.1' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'CC6.1' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'CC6.2' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'CC7.2' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

-- ISO 27001 requirements
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

-- ISO 27001 control mappings
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

-- GDPR requirements
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

-- GDPR control mappings
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

-- Sample control tests (Phase 6B) so Tests section has data
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
