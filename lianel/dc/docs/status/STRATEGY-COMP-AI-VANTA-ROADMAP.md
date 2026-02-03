# Strategy: Energy on Hold, Comp-AI as Open-Source Vanta Replacement

**Date**: January 2026  
**Status**: **Current plan** – supersedes energy-platform-first roadmap for now.

---

## 1. Strategic decision

- **Energy project**: **On hold.** Eurostat/ENTSO-E/OSM pipelines, geo-enrichment, and related analytics remain as-is; no new energy-focused development for the moment.
- **Primary focus**: **Comp-AI** – build it as an **open-source replacement for Vanta** (compliance automation: SOC 2, evidence collection, control management, policy/audit support).
- **Rationale**: Prioritize a single product (Comp-AI) that can serve compliance/security automation (Vanta-like) and grow in that direction.

---

## 2. Vision: Comp-AI as open-source Vanta alternative

**Vanta (reference)** automates compliance (e.g. SOC 2) via:

- Automated tests/monitoring of security controls  
- Control management and framework mapping (SOC 2, ISO 27001, etc.)  
- Evidence collection and documentation  
- Policy/audit document generation  
- Alerts, remediation workflows, integrations  

**Comp-AI direction** (open-source replacement):

- **AI-assisted compliance**: Use LLMs (Ollama/hosted) for natural-language queries, policy summarization, gap explanations, and remediation suggestions.
- **Multiframework support**: SOC 2, ISO 27001, GDPR, HIPAA, PCI DSS, NIST CSF, and 15+ other frameworks (see §6 below); one control/evidence model, cross-mapping so one control satisfies many framework requirements.
- **Evidence & controls**: Controls, evidence, GitHub integration, and /comp-ai/controls UI are in place (Phase 4).
- **Transparency**: Open source, self-hosted option, no vendor lock-in.

Current Comp-AI service is the foundation: auth (Keycloak), request history (DB), and AI processing (Ollama/hosted). Next steps extend toward compliance use cases, multiframework control/evidence model, and AI prompts per framework.

---

## 3. Comp-AI roadmap (high level)

| Phase | Focus | Status |
|-------|--------|--------|
| **1. Foundation** | Service, Keycloak, DB, AI (Ollama/mock), API, deploy | ✅ Done |
| **2. Core product** | Rate limiting ✅, stable AI doc ✅, history verification ✅, API hardening ✅ | ✅ Done |
| **3. Compliance features** | Framework-aware prompts, GET /api/v1/frameworks, frontend selector | ✅ Done |
| **4. Integrations & evidence** | Controls, evidence, GitHub integration, frontend /comp-ai/controls | ✅ Done |
| **5. Frameworks & audit** | Multiframework mapping, audit-ready outputs, remediation workflows | ✅ Done |

---

## 4. Multiframework support (SOC 2, ISO 27001, GDPR, etc.)

Comp-AI will support **multiple compliance frameworks** in one place, aligned with Vanta’s 20+ frameworks so we can cover as much as possible.

**Primary frameworks (first-class):** SOC 2, ISO 27001, GDPR, HIPAA, PCI DSS, NIST CSF 2.0.

**Additional frameworks (same control/evidence model):** NIST 800-53, NIST 800-171, FedRAMP, HITRUST CSF, ISO 27017/27018/27701, ISO 42001, CMMC, Cyber Essentials, CJIS, DORA, NIS 2, SOX ITGC, USDP, MVSP, AWS FTR, Microsoft SSPA, OFDSS, and custom frameworks.

**Approach:**

- **Control model**: Internal “controls” (one implementation) map to many **framework requirements** (e.g. SOC 2 CC6.1, ISO 27001 A.9.4.2, GDPR Art. 32). One control can satisfy multiple frameworks (cross-mapping).
- **Evidence reuse**: One evidence item (screenshot, log, policy) can be linked to multiple controls; auditors see which frameworks each control/evidence supports.
- **Vanta Control Set**: Use or mirror [Vanta Control Set](https://github.com/VantaInc/vanta-control-set) (Apache-2.0, machine-readable SOC 2, HIPAA, etc.) for standard definitions and IDs where applicable.
- **AI**: Framework-aware prompts (user picks SOC 2, ISO 27001, GDPR, etc.); AI answers in terms of that framework’s requirements and suggests controls/evidence.

Full design: **`lianel/dc/docs/status/COMP-AI-MULTIFRAMEWORK-SUPPORT.md`**.

---

## 5. Phase 5 – Frameworks & audit (done)

**Completed:**

1. **Multiframework mapping** – ISO 27001 seeded (migration 013); existing controls mapped to SOC 2 and ISO 27001 (one control → many requirements). GDPR seed in migration 015.
2. **Audit-ready export** – `GET /api/v1/controls/export?format=csv|json&framework=soc2|iso27001` exports controls + requirements + evidence; optional `framework` filter for per-framework audit view.
3. **Remediation / gaps** – `GET /api/v1/controls/gaps` lists controls with no evidence; `GET/PUT /api/v1/controls/:id/remediation` for assignee, due date, status (table `comp_ai.remediation_tasks`, migration 014).
4. **Requirements API** – `GET /api/v1/requirements?framework=soc2|iso27001` lists framework requirements from DB for audit view.

**Run migrations** (one-time on the DB used by comp-ai-service): from `lianel/dc`, run `bash scripts/deployment/run-comp-ai-migrations.sh` (runs 009–015).

---

## 6. References

- **Comp-AI multiframework design**: `lianel/dc/docs/status/COMP-AI-MULTIFRAMEWORK-SUPPORT.md`  
- **Comp-AI implementation status**: `lianel/dc/docs/deployment/COMP-AI-IMPLEMENTATION-STATUS.md`  
- **Comp-AI deployment**: `lianel/dc/docs/deployment/COMP-AI-DEPLOYMENT-PLAN.md`  
- **Model usage (Ollama)**: `lianel/dc/docs/deployment/COMP-AI-MODEL-USAGE-AND-SCALING.md`  
- **Energy roadmap (on hold)**: `lianel/docs/08-implementation/01-roadmap.md`  

---

**Next action**: Phase 6 or beyond – Expand frameworks (more requirements per framework), automated tests per control, or deeper AI remediation suggestions. See COMP-AI-MULTIFRAMEWORK-SUPPORT.md.
