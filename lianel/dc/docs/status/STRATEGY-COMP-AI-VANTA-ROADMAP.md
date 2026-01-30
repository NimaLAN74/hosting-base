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
- **Evidence & controls**: (Roadmap) Integrations and workflows to collect evidence, map controls, and prepare audit-ready artifacts.
- **Transparency**: Open source, self-hosted option, no vendor lock-in.

Current Comp-AI service is the foundation: auth (Keycloak), request history (DB), and AI processing (Ollama/hosted). Next steps extend toward compliance use cases, multiframework control/evidence model, and AI prompts per framework.

---

## 3. Comp-AI roadmap (high level)

| Phase | Focus | Status |
|-------|--------|--------|
| **1. Foundation** | Service, Keycloak, DB, AI (Ollama/mock), API, deploy | ✅ Done |
| **2. Core product** | Rate limiting ✅, stable AI doc ✅, history verification ✅, API hardening ✅ | ✅ Done |
| **3. Compliance features** | Framework-aware prompts, GET /api/v1/frameworks, frontend selector | Done (first) |
| **4. Integrations & evidence** | Tool integrations, evidence collection, policy/control mapping (Vanta-like) | Future |
| **5. Frameworks & audit** | Multiframework mapping, audit-ready outputs, remediation workflows | Future |

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

## 5. Next step (from this roadmap)

**Immediate next step**: **Phase 2 – Core product**

1. **Rate limiting** ✅ – Per-IP rate limit on `/api/v1/process` and `/api/v1/history` (configurable; 429 + Retry-After).  
2. **Stable AI behavior** ✅ – Ollama/tinyllama default (compose); documented in `COMP-AI-MODEL-USAGE-AND-SCALING.md` §0; fallback to mock when Ollama fails.  
3. **History & UX** ✅ – History API and frontend verified; doc and script `scripts/monitoring/verify-comp-ai-history.sh` for remote verification.  
4. **API hardening** ✅ – Prompt validation (required, max length 32768); consistent error JSON `{"error": "..."}`.

**Phase 2 complete.** Next: **Phase 3** (compliance-oriented prompts and first control/evidence concepts).

---

## 6. References

- **Comp-AI multiframework design**: `lianel/dc/docs/status/COMP-AI-MULTIFRAMEWORK-SUPPORT.md`  
- **Comp-AI implementation status**: `lianel/dc/docs/deployment/COMP-AI-IMPLEMENTATION-STATUS.md`  
- **Comp-AI deployment**: `lianel/dc/docs/deployment/COMP-AI-DEPLOYMENT-PLAN.md`  
- **Model usage (Ollama)**: `lianel/dc/docs/deployment/COMP-AI-MODEL-USAGE-AND-SCALING.md`  
- **Energy roadmap (on hold)**: `lianel/docs/08-implementation/01-roadmap.md`  

---

**Next action**: Proceed with Phase 2 – Core product (rate limiting, history/UX verification, API hardening). Then implement multiframework data model and seed one framework (e.g. SOC 2) per COMP-AI-MULTIFRAMEWORK-SUPPORT.md.
