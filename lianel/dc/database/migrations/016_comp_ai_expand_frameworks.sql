-- Phase 6A: Expand frameworks – more requirements per framework (SOC 2, ISO 27001, GDPR)
-- Run after 015_comp_ai_seed_gdpr.sql

-- ---------- SOC 2: additional Trust Services Criteria ----------
INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC6.3', 'Logical access – authorization',
  'The entity authorizes, modifies, or removes access to data, software, functions, and other resources based on roles and responsibilities.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC6.4', 'Logical access – access removal',
  'The entity removes access to data, software, and other resources when access is no longer authorized.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC6.5', 'Logical access – authentication',
  'The entity implements logical access security measures to protect against threats from sources outside its system boundaries.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC7.1', 'Detection – security events',
  'The entity has a process to identify, select, and develop ongoing and separate evaluations to ascertain whether the criteria are present.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC7.3', 'Detection – anomalies',
  'The entity evaluates security events to determine whether they could or have resulted in a failure to meet the entity''s objectives.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'CC7.4', 'Detection – response to identified issues',
  'The entity responds to identified security incidents by executing a defined incident response program.'
FROM comp_ai.frameworks f WHERE f.slug = 'soc2'
ON CONFLICT (framework_id, code) DO NOTHING;

-- SOC 2 control–requirement mappings (expand)
INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MFA-001' AND r.code = 'CC6.3' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MFA-001' AND r.code = 'CC6.5' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'CC6.3' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'CC6.4' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'CC7.1' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'CC7.3' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'CC7.4' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'soc2')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

-- ---------- ISO 27001: additional Annex A controls ----------
INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.5.1', 'Policies for information security',
  'Policies for information security shall be defined, approved by management, published and communicated to employees and relevant external parties.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.5.2', 'Information security roles and responsibilities',
  'Information security roles and responsibilities shall be defined and allocated in accordance with the organisation''s information security policy.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.8.1', 'Inventory of assets',
  'Assets associated with information and information processing facilities shall be identified and the organisation shall define and implement appropriate protection responsibilities.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.9.2.1', 'User registration and de-registration',
  'A formal user registration and de-registration process shall be implemented to enable assignment of access rights.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.12.1.1', 'Documented operating procedures',
  'Operating procedures for information processing facilities shall be documented and made available to personnel who need them.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'A.12.4.2', 'Protection of log information',
  'Logging facilities and log information shall be protected against tampering and unauthorized access.'
FROM comp_ai.frameworks f WHERE f.slug = 'iso27001'
ON CONFLICT (framework_id, code) DO NOTHING;

-- ISO 27001 control–requirement mappings (expand)
INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'A.9.2.1' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'iso27001')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'A.12.4.2' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'iso27001')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

-- ---------- GDPR: additional articles ----------
INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 24', 'Responsibility of the controller',
  'The controller shall implement appropriate technical and organisational measures to ensure and be able to demonstrate that processing is performed in accordance with this Regulation.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 28', 'Processor',
  'The processor shall implement appropriate technical and organisational measures to ensure a level of security appropriate to the risk.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 30', 'Records of processing activities',
  'The controller shall maintain a record of processing activities under its responsibility.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 34', 'Communication of a personal data breach to the data subject',
  'When the personal data breach is likely to result in a high risk to the rights and freedoms of natural persons, the controller shall communicate the breach to the data subject.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

INSERT INTO comp_ai.requirements (framework_id, code, title, description)
SELECT f.id, 'Art. 5(1)(a)', 'Lawfulness, fairness and transparency',
  'Personal data shall be processed lawfully, fairly and in a transparent manner in relation to the data subject.'
FROM comp_ai.frameworks f WHERE f.slug = 'gdpr'
ON CONFLICT (framework_id, code) DO NOTHING;

-- GDPR control–requirement mappings (expand)
INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MFA-001' AND r.code = 'Art. 24' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'Art. 24' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-ACCESS-001' AND r.code = 'Art. 30' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'Art. 34' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;

INSERT INTO comp_ai.control_requirements (control_id, requirement_id)
SELECT c.id, r.id FROM comp_ai.controls c, comp_ai.requirements r
WHERE c.internal_id = 'CTL-MON-001' AND r.code = 'Art. 28' AND r.framework_id = (SELECT id FROM comp_ai.frameworks WHERE slug = 'gdpr')
ON CONFLICT (control_id, requirement_id) DO NOTHING;
