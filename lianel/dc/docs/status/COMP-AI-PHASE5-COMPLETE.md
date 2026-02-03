# Comp-AI Phase 5: Frameworks & audit – complete

**Date**: January 2026  
**Status**: Done

---

## Delivered

### 1. Multiframework mapping (migrations)

- **013_comp_ai_seed_iso27001.sql** – Seeds ISO 27001 framework and Annex A requirements (A.9.4.1, A.9.4.2, A.12.4.1, A.12.4.3); maps existing controls (CTL-MFA-001, CTL-ACCESS-001, CTL-MON-001) to ISO requirements so one control satisfies both SOC 2 and ISO 27001.
- **015_comp_ai_seed_gdpr.sql** – GDPR framework seed (if present).
- **011_comp_ai_seed_soc2.sql** – SOC 2 requirements and control mappings (already present).

### 2. Audit-ready export

- **GET /api/v1/controls/export**
  - `?format=csv` – CSV export (control_id, internal_id, name, requirement_codes, evidence_count, evidence_types).
  - `?format=json` or omit – JSON export (controls with requirements and evidence).
  - **`?framework=soc2` or `?framework=iso27001`** – Restricts export to controls mapped to that framework and only those requirement codes (per-framework audit view).

### 3. Remediation / gaps

- **GET /api/v1/controls/gaps** – Returns controls that have no evidence (gaps).
- **GET /api/v1/controls/:id/remediation** – Get remediation task for a control.
- **PUT /api/v1/controls/:id/remediation** – Create or update remediation (assigned_to, due_date, status, notes).
- Table **comp_ai.remediation_tasks** (migration 014).

### 4. Requirements API (Phase 5)

- **GET /api/v1/requirements** – List all framework requirements from DB.
- **GET /api/v1/requirements?framework=soc2** – List requirements for one framework (audit view).

---

## One-time: run migrations

The comp_ai schema and Phase 4/5 tables (frameworks, requirements, controls, control_requirements, evidence, remediation_tasks) and seeds (SOC 2, ISO 27001, GDPR) are applied by running migrations **009–015** on the same Postgres used by the Comp-AI service.

**From repo (lianel/dc):**

```bash
cd lianel/dc
bash scripts/deployment/run-comp-ai-migrations.sh
```

Requires `.env` with `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_USER`, `POSTGRES_PASSWORD` (or `COMP_AI_MIGRATION_PASSWORD`), and the database name used by the Comp-AI service. The script runs 009 → 010 → 011 → 012 → 013 → 014 → 015 in order.

---

## References

- Strategy: `lianel/dc/docs/status/STRATEGY-COMP-AI-VANTA-ROADMAP.md`
- Multiframework design: `lianel/dc/docs/status/COMP-AI-MULTIFRAMEWORK-SUPPORT.md`
- Implementation status: `lianel/dc/docs/deployment/COMP-AI-IMPLEMENTATION-STATUS.md`
- Migration script: `lianel/dc/scripts/deployment/run-comp-ai-migrations.sh`
