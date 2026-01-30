# Comp-AI: Multiframework Support (SOC 2, ISO 27001, GDPR, etc.)

**Date**: January 2026  
**Status**: Design – target coverage aligned with Vanta’s 20+ frameworks.

---

## 1. Objective

Support **multiple compliance frameworks** in Comp-AI so users can:

- Work toward SOC 2, ISO 27001, GDPR, HIPAA, and others from one place  
- **Reuse controls and evidence** across frameworks (one control → many framework requirements)  
- Use **AI-assisted** guidance, gap explanation, and remediation per framework  
- Produce **audit-ready** evidence and mappings (Vanta-like)

Design is informed by **Vanta** (20+ frameworks, 1,200+ automated tests, 300+ integrations, control cross-mapping) and by the **Vanta Control Set** (Apache-2.0, machine-readable standards).

---

## 2. Target frameworks (cover as much as Vanta-style as possible)

### 2.1 Primary (first-class support)

| Framework | Scope | Notes |
|-----------|--------|------|
| **SOC 2** | Security, availability, processing integrity, confidentiality, privacy | Trust Services Criteria; most requested. |
| **ISO 27001** | ISMS, risk management, Annex A controls | Global baseline; maps well to SOC 2. |
| **GDPR** | EU privacy, DPA, DPO, breach, rights | Privacy-focused; evidence overlaps with SOC 2 / ISO. |
| **HIPAA** | US healthcare, PHI, safeguards | Required for US health; Vanta Control Set has HIPAA. |
| **PCI DSS** | Cardholder data | Required for payment card processing. |
| **NIST CSF 2.0** | Identify, Protect, Detect, Respond, Recover | US gov/industry baseline; NIST 800-53 subset. |

### 2.2 Secondary (supported, same control/evidence model)

| Framework | Scope | Notes |
|-----------|--------|------|
| **NIST 800-53** | FedRAMP / US federal controls | Detailed control catalog. |
| **NIST 800-171** | CUI, defense supply chain | Subset of 800-53. |
| **FedRAMP** | US federal cloud | Built on NIST 800-53. |
| **HITRUST CSF** | Healthcare + general security | Combines multiple standards. |
| **ISO 27017** | Cloud security | Extension of ISO 27001. |
| **ISO 27018** | Cloud PII | Extension of ISO 27001. |
| **ISO 27701** | PIMS, privacy extension | Extension of ISO 27001. |
| **ISO 42001** | AI management system | Emerging AI compliance. |
| **CMMC** | US DoD supply chain | Levels 1–3; overlaps NIST. |
| **Cyber Essentials** | UK baseline | Simple, widely required. |
| **CJIS** | Criminal justice information | US law enforcement. |
| **DORA** | EU financial sector resilience | EU regulation. |
| **NIS 2** | EU critical infrastructure | EU directive. |
| **SOX ITGC** | IT general controls for financial reporting | Public companies. |
| **USDP** | US Data Privacy (state laws) | CCPA, state privacy. |
| **MVSP** | Minimum Viable Security Product | Vendor security baseline. |
| **AWS FTR** | AWS Foundational Technical Review | Cloud partner. |
| **Microsoft SSPA** | Microsoft security assessment | Cloud partner. |
| **OFDSS** | Open Finance Data Security Standard | Open banking. |

### 2.3 Custom frameworks

- Allow **user-defined frameworks** and **custom controls** (internal policies, contract-specific requirements).  
- Same data model: controls, requirements, evidence, mappings.

---

## 3. Control model and cross-mapping

### 3.1 Concepts (Vanta-aligned)

- **Control** (internal): One actionable item we implement and test (e.g. “MFA required for all users”).  
- **Framework requirement**: A specific criterion in a standard (e.g. SOC 2 CC6.1, ISO 27001 A.9.4.2, GDPR Art. 32).  
- **Mapping**: One internal control can satisfy **many** framework requirements across **many** frameworks (cross-mapping).  
- **Evidence**: Artifacts (screenshots, logs, policies, API outputs) attached to controls; one evidence item can support multiple controls/requirements.

### 3.2 Data model (high level)

- **Frameworks** – id, name, slug, version, scope (e.g. SOC 2 Type II).  
- **Requirements** – id, framework_id, code (e.g. CC6.1), title, description.  
- **Controls** – id, internal_id, name, description, category.  
- **Control–requirement mappings** – control_id, requirement_id (many-to-many).  
- **Evidence** – id, control_id(s), type, source, collected_at, file/link.  
- **Tests / automated checks** – id, control_id, type (e.g. integration), schedule, result.

### 3.3 Evidence reuse

- One **evidence** record can be linked to multiple **controls**.  
- One **control** can be linked to multiple **framework requirements**.  
- Result: Collect once, satisfy many frameworks (same approach as Vanta).

---

## 4. Use of Vanta Control Set

- **Vanta Control Set** (GitHub: VantaInc/vanta-control-set, Apache-2.0):  
  - Machine-readable JSON per standard (`controls/`).  
  - Structure: `standard` (metadata + control IDs per section), `controlDetail` (per control).  
  - Includes at least SOC 2 and HIPAA; schema in `schema.json`.

**Comp-AI approach:**

- **Option A**: Consume or mirror Vanta Control Set data (import SOC 2, HIPAA, etc.) for control/requirement definitions and IDs.  
- **Option B**: Define our own control set and **map** to Vanta’s requirement IDs for compatibility and user familiarity.  
- **Option C**: Hybrid – use Vanta Control Set for frameworks they publish, add our own for GDPR, ISO 27001, etc. from public specs.

Document the chosen option and any schema adaptations in the implementation phase.

---

## 5. Comp-AI integration (AI + multiframework)

- **Framework-aware prompts**: User selects framework(s); AI answers in terms of SOC 2, ISO 27001, GDPR, etc. (requirements, controls, gaps).  
- **Control/evidence APIs**: CRUD for frameworks, requirements, controls, mappings, evidence (used by UI and automation).  
- **Gap and readiness**: For each framework, list satisfied vs missing requirements; suggest controls and evidence.  
- **Remediation**: AI-suggested steps (and later, code/scripts) mapped to control IDs and framework requirements.  
- **Audit view**: Export per framework (control ↔ requirement matrix, evidence links) for auditors.

---

## 6. Implementation order (suggested)

1. **Data model** – Tables (or equivalent) for frameworks, requirements, controls, control–requirement mappings, evidence.  
2. **Seed data** – Import at least one framework (e.g. SOC 2 or subset) from Vanta Control Set or public source.  
3. **APIs** – List frameworks, requirements, controls; link evidence to controls; filter by framework.  
4. **AI** – Framework parameter in Comp-AI prompts; responses reference requirement codes and control names.  
5. **Cross-mapping** – Many-to-many control ↔ requirement; UI/export showing “one control → these requirements.”  
6. **Expand** – Add ISO 27001, GDPR, HIPAA, then others from §2.

---

## 7. References

- **Strategy**: `lianel/dc/docs/status/STRATEGY-COMP-AI-VANTA-ROADMAP.md`  
- **Vanta Control Set**: https://github.com/VantaInc/vanta-control-set (Apache-2.0)  
- **Vanta multi-framework**: https://www.vanta.com/products/additional-frameworks  
- **Vanta Compliance Standards Library**: https://help.vanta.com/en/collections/12575334-compliance-standards-library  

---

**Next action**: Implement data model and seed one framework (e.g. SOC 2); then add framework-aware Comp-AI prompts and control/evidence APIs.
