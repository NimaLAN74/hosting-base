# Comp-AI Test Prompts by Framework

Use these prompts to verify framework-aware behavior in the Comp-AI portal or via `POST /api/v1/process` with `{"prompt": "...", "framework": "<id>"}`. Supported framework IDs: `soc2`, `iso27001`, `gdpr`, `hipaa`, `pci_dss`, `nist_csf` (see `GET /api/v1/frameworks`).

---

## No framework (generic)

Use `framework` omitted or empty to test baseline behavior.

| Purpose | Example prompt |
|--------|-----------------|
| General | What are the main steps to implement a security awareness program? |
| Policy | What should a password policy include? |

---

## SOC 2 (`soc2`)

Trust Services Criteria (security, availability, processing integrity, confidentiality, privacy).

| Purpose | Example prompt |
|--------|-----------------|
| Control | What does CC6.1 require for logical access? |
| Evidence | What evidence would satisfy CC7.2 for system monitoring? |
| Policy | What should our access review policy cover for SOC 2? |
| Scope | List the five Trust Service Criteria and one control from each. |

---

## ISO 27001 (`iso27001`)

Information security management system (ISMS), Annex A controls.

| Purpose | Example prompt |
|--------|-----------------|
| Control | What does Annex A control A.9.4.2 require? |
| Evidence | What evidence supports A.12.4.1 (logging)? |
| Policy | What should an ISO 27001 risk treatment plan include? |
| Scope | Name three Annex A controls related to access control. |

---

## GDPR (`gdpr`)

EU General Data Protection Regulation (privacy, DPA, breach, rights).

| Purpose | Example prompt |
|--------|-----------------|
| Article | What does Article 32 require for security of processing? |
| Rights | What are the main data subject rights under GDPR? |
| Breach | When must we notify the supervisory authority of a breach (Article 33)? |
| Policy | What should a Record of Processing Activities include? |

---

## HIPAA (`hipaa`)

US Health Insurance Portability and Accountability Act (PHI, safeguards).

| Purpose | Example prompt |
|--------|-----------------|
| Safeguard | What are the HIPAA Security Rule administrative safeguards? |
| PHI | What is considered PHI and how must it be protected? |
| BAA | When do we need a Business Associate Agreement? |
| Policy | What should a HIPAA workforce training policy cover? |

---

## PCI DSS (`pci_dss`)

Payment Card Industry Data Security Standard.

| Purpose | Example prompt |
|--------|-----------------|
| Requirement | What does Requirement 8 (identify and authenticate access) require? |
| Scope | How do we determine PCI scope for cardholder data? |
| Evidence | What evidence is needed for Requirement 10 (track access)? |
| Policy | What should a PCI DSS password policy include? |

---

## NIST CSF 2.0 (`nist_csf`)

NIST Cybersecurity Framework (Identify, Protect, Detect, Respond, Recover).

| Purpose | Example prompt |
|--------|-----------------|
| Function | What subcategories does the Protect function (PR) include? |
| Subcategory | What does ID.AM-5 (resources prioritized) mean? |
| Mapping | How does NIST CSF map to incident response? |
| Policy | What should a CSF-based risk assessment cover? |

---

## Quick copy-paste (one per framework)

```
What does CC6.1 require for logical access? [SOC 2]
What does Annex A control A.9.4.2 require? [ISO 27001]
What does Article 32 require for security of processing? [GDPR]
What are the HIPAA Security Rule administrative safeguards? [HIPAA]
What does PCI DSS Requirement 8 require? [PCI DSS]
What subcategories does the NIST CSF Protect function include? [NIST CSF]
```

---

## API examples

**With framework (SOC 2):**
```bash
curl -s -X POST https://comp-ai.lianel.se/api/v1/process \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <JWT>" \
  -d '{"prompt": "What does CC6.1 require for logical access?", "framework": "soc2"}'
```

**List frameworks:**
```bash
curl -s https://comp-ai.lianel.se/api/v1/frameworks
```

**No framework:**
```bash
curl -s -X POST https://comp-ai.lianel.se/api/v1/process \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <JWT>" \
  -d '{"prompt": "What should a password policy include?"}'
```
