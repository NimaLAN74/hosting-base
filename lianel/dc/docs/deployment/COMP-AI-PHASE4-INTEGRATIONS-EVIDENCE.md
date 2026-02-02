# Comp-AI Phase 4: Integrations & Evidence

**Date**: February 2026  
**Status**: Implemented (data model, API, first integration, design doc).

---

## 1. Scope

Phase 4 adds **controls**, **evidence**, and **framework mappings** so Comp-AI can:

- Store internal **controls** and map them to **framework requirements** (e.g. SOC 2 CC6.1, ISO A.9.4.2).
- Attach **evidence** (artifacts) to controls; one evidence item can support one control (extensible to many later).
- Expose **APIs** to list controls, get control with requirement mappings, list/attach evidence.
- Use a **first integration** (GitHub) to collect evidence (last commit, branch protection) and link it to a control.

Aligned with **COMP-AI-MULTIFRAMEWORK-SUPPORT.md** and **STRATEGY-COMP-AI-VANTA-ROADMAP.md**.

---

## 2. Data model

### 2.1 Tables (migrations 010 + 011)

| Table | Purpose |
|-------|--------|
| `comp_ai.frameworks` | Frameworks (e.g. SOC 2): id, slug, name, version, scope. |
| `comp_ai.requirements` | Framework requirements: id, framework_id, code, title, description. |
| `comp_ai.controls` | Internal controls: id, internal_id, name, description, category. |
| `comp_ai.control_requirements` | Many-to-many: control_id, requirement_id. |
| `comp_ai.evidence` | Evidence: id, control_id, type, source, description, link_url, collected_at, created_by. |

### 2.2 Seed (SOC 2)

- **Framework**: SOC 2 (slug `soc2`).
- **Requirements**: CC6.1, CC6.2, CC7.2 (sample).
- **Controls**: CTL-MFA-001 (MFA), CTL-ACCESS-001 (Access review), CTL-MON-001 (Monitoring).
- **Mappings**: CTL-MFA-001 → CC6.1; CTL-ACCESS-001 → CC6.1, CC6.2; CTL-MON-001 → CC7.2.

**Migrations**: `010_comp_ai_controls_evidence.sql`, `011_comp_ai_seed_soc2.sql`. Run on the host (same DB as comp_ai.requests).

---

## 3. API (controls & evidence)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/controls` | List all controls (auth required). |
| GET | `/api/v1/controls/:id` | Get one control with requirement mappings (auth required). |
| GET | `/api/v1/evidence` | List evidence; query `control_id`, `limit`, `offset` (auth required). |
| POST | `/api/v1/evidence` | Attach evidence: body `control_id`, `type`, `source`, `description`, `link_url` (auth required). |
| POST | `/api/v1/integrations/github/evidence` | Collect evidence from GitHub and link to control (auth + GITHUB_TOKEN required). Body: `control_id`, `owner`, `repo`, `evidence_type`: `"last_commit"` or `"branch_protection"`. |

---

## 4. First integration: GitHub

- **Config**: Optional `GITHUB_TOKEN` (env). If unset, `POST /api/v1/integrations/github/evidence` returns 503.
- **Evidence types**:
  - **last_commit**: Fetches repo default branch and last commit (SHA, message, author); creates evidence with type `github_last_commit`, source `github:owner/repo`, description and link to commit.
  - **branch_protection**: Fetches default branch and branch protection status; creates evidence with type `github_branch_protection`, source `github:owner/repo`, description and repo link.
- **Flow**: Client sends `control_id`, `owner`, `repo`, `evidence_type` → backend calls GitHub API → creates row in `comp_ai.evidence` linked to control → returns `CreateEvidenceResponse`.

---

## 5. Task list (implemented)

- [x] **1. Control/evidence data model** – Migration 010 (tables), 011 (SOC 2 seed); Rust models and DB queries.
- [x] **2. First integration** – GitHub: `last_commit`, `branch_protection`; `POST /api/v1/integrations/github/evidence`.
- [x] **3. API for controls/evidence** – GET controls, GET control/:id, GET evidence, POST evidence.
- [x] **4. Phase 4 design doc** – This document.

---

## 6. Next steps (Phase 4+)

- **Run migrations** on the remote DB: execute `010_comp_ai_controls_evidence.sql` and `011_comp_ai_seed_soc2.sql` (e.g. from host or CI).
- **Optional**: Add more frameworks (ISO 27001, GDPR, etc.) and requirements; seed via migrations or admin API.
- **Optional**: Evidence ↔ multiple controls (evidence_control many-to-many) if one artifact supports several controls.
- **Optional**: More integrations (e.g. AWS, Slack, Jira) following the same pattern (env token, fetch, create evidence).
- **UI**: Controls and evidence list/detail pages in the frontend; “Collect from GitHub” button calling `POST /api/v1/integrations/github/evidence`.

---

## 7. References

- **Strategy**: `lianel/dc/docs/status/STRATEGY-COMP-AI-VANTA-ROADMAP.md`
- **Multiframework design**: `lianel/dc/docs/status/COMP-AI-MULTIFRAMEWORK-SUPPORT.md`
- **Migrations**: `lianel/dc/database/migrations/010_comp_ai_controls_evidence.sql`, `011_comp_ai_seed_soc2.sql`
