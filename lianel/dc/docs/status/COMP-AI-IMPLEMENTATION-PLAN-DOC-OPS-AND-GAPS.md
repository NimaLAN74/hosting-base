# Comp-AI Implementation Plan: Document/Operational + Gap-List Priorities

**Purpose:** One plan that (1) details how to implement **document/operational compliance** (non-code: docs, emails, Excel, scan for org), and (2) ties it to the **Vanta gap list** so you can choose what to prioritise first (doc/ops vs test runner vs integrations vs policies vs alerts).

**Use:** Pick a workstream from the plan or from the gap list, then execute in that order. This doc is the single place to see “what to build” and “how it fits with the gap list”.

---

## Part 1: Document / Operational Compliance – Implementation Plan

Scope: documents, policies, spreadsheets, emails; evidence linked to controls; optional content extraction + AI analysis; scan/monitor for organisation (operational and administrative parts).

### Phase A – Document evidence (no new infra)

**Goal:** Treat documents, policies, spreadsheets, and emails as first-class evidence linked to controls. No file storage or AI analysis yet.

| # | Task | Type | Notes |
|---|------|------|--------|
| A1 | **Evidence types** – Accept and display `document`, `policy`, `spreadsheet`, `email` in API and UI (in addition to `manual`, `github_*`). | Backend + Frontend | ✅ API accepts any type; UI dropdown added. |
| A2 | **UI – Add evidence** – In Controls → Add evidence: type selector includes Document, Policy, Spreadsheet, Email; source/description/link as today. Optional: “Link to document” hint (e.g. SharePoint, Drive URL). | Frontend | ✅ Select + link hint. |
| A3 | **Export** – Audit export (JSON/CSV) includes evidence type; filter or label by type so auditors see document vs technical evidence. | Backend | ✅ Already present (evidence_types in CSV; type in JSON). |
| A4 | **Docs** – Update user/demo docs: “You can attach document/policy/email evidence by link for operational and administrative controls.” | Docs | ✅ COMP-AI-VALUE-RISK-AND-REMEDIATION.md updated. |

**Deliverable:** Users can link document/policy/spreadsheet/email evidence to controls via URL + type; no new DB schema.

**Effort (rough):** Small (1–2 dev days).

**Status:** ✅ **Done** — Type selector in Add evidence: Document, Policy, Spreadsheet, Email, Manual, Other (custom); link hint for SharePoint/Drive; export already includes evidence type.

---

### Phase B – File upload + content extraction + AI analysis

**Goal:** Optional file upload for document evidence; extract text; AI suggests control mapping and gaps.

| # | Task | Type | Notes |
|---|------|------|--------|
| B1 | **Storage** – Define where to store uploaded files: e.g. S3-compatible bucket or local volume; config (e.g. `COMP_AI_EVIDENCE_STORAGE_PATH` or S3 bucket). | Backend / Infra | Migration or config only; no DB schema change if we store path/URL in existing `evidence.link_url` or new column `evidence.file_path`. |
| B2 | **DB (optional)** – If needed: add `evidence.file_path` or `evidence.file_name` and/or `evidence.content_type`; or reuse `link_url` for internal path. | Backend | Only if we need to distinguish “uploaded file” from “external link”. |
| B3 | **Upload API** – `POST /api/v1/evidence/upload` or extend `POST /api/v1/evidence` with multipart file: control_id, type (document/policy/spreadsheet), file. Save file, create evidence row with source=filename, link_url=internal path or download URL. | Backend | Auth required; size limit (e.g. 10–50 MB); allowed types: pdf, docx, xlsx, csv, txt. |
| B4 | **Text extraction** – Service or library to extract text from PDF, Office, CSV/Excel (e.g. Apache Tika in a sidecar, or Rust crate, or external API). Store extracted text in memory or in DB (e.g. `evidence.extracted_text` or separate table) for analysis. | Backend | New dependency or container; consider async job if files are large. |
| B5 | **Analyse document API** – `POST /api/v1/evidence/:id/analyze` or `POST /api/v1/analyze/document`: input text (or evidence id → fetch extracted text); prompt with control/requirement list; Ollama returns summary + suggested controls/requirements + gaps. Response: `{ summary, suggested_control_ids?, gaps?, model_used }`. | Backend | Reuse Ollama; similar to remediation/suggest. |
| B6 | **UI – Upload** – In Add evidence: option “Upload file” (type Document/Policy/Spreadsheet); file picker; submit to upload endpoint; show success and link to evidence. | Frontend | |
| B7 | **UI – Analyse** – On evidence detail or list: “Analyse” button for evidence that has extracted text; call analyse API; show summary and suggested control mapping; “Link to control” to attach to another control. | Frontend | |

**Deliverable:** Upload documents; extract text; one-click AI analysis with control mapping and gap suggestions.

**Effort (rough):** Medium (1–2 weeks: upload + extraction + analyse endpoint + UI).

---

### Phase C – Scan / monitor for organisation

**Goal:** Run a “scan” over many documents (folder, list of URLs, or one integration) to create/update evidence and optionally run analysis for the whole org.

| # | Task | Type | Notes |
|---|------|------|--------|
| C1 | **Scan API (batch)** – `POST /api/v1/scan/documents`: body = list of URLs or of (url, type, optional control_id). For each: fetch or accept metadata; create evidence (or update); optionally trigger extraction + analysis (async). Return job_id or list of evidence ids. | Backend | Auth required; rate-limit; consider async queue (e.g. background job). |
| C2 | **Folder / file list** – Alternative: `POST /api/v1/scan/upload-batch`: multiple files; create evidence for each; optional extraction + analysis. | Backend | Simpler than C1 if no external URLs. |
| C3 | **Integration (optional)** – One connector: e.g. SharePoint or Google Drive: list docs in a folder/library; for each, create evidence (link) and optionally fetch content for analysis. | Backend | OAuth + list files + optional download; effort depends on provider. |
| C4 | **UI – Scan** – “Scan documents” page or section: paste URLs or upload multiple files; select control (or “no control – analyse only”); run scan; show progress and list of created evidence + analysis results. | Frontend | |
| C5 | **Operational view (optional)** – Filter controls or evidence by tag/category “operational” or “administrative” (or by department: HR, Finance, IT). Requires: control or evidence tag/category in DB + filter in API + UI. | Backend + Frontend | Can be a later sub-phase. |

**Deliverable:** Org-wide document scan (batch URLs or batch upload or one integration); evidence + optional AI analysis in one run.

**Effort (rough):** Medium–High (1–3 weeks depending on async and integration).

---

### Phase D – Email evidence

**Goal:** Email as evidence type; optional integration for metadata/summary (no need to store full bodies).

| # | Task | Type | Notes |
|---|------|------|--------|
| D1 | **Evidence type “email”** – Already in Phase A; ensure UI and export treat “email” clearly (e.g. source = “From: X; Subject: Y; Date”). | Frontend | May be done in A1–A2. |
| D2 | **Manual email evidence** – User adds evidence type=email, source=description (e.g. “Retention notice from M365, 2026-01-15”), link_url=optional link to mail archive. | - | No backend change if A1 done. |
| D3 | **Integration (optional)** – M365 or Google: list recent emails (metadata only) or “compliance” channel; create evidence per relevant email (subject, date, link). No body storage. | Backend | OAuth + Mail API; effort Medium. |
| D4 | **DLP-style (optional)** – Integrate with DLP or external tool that flags “sensitive in email”; store result as evidence (e.g. “DLP scan 2026-01-15: no violation”). | Backend | Lower priority. |

**Deliverable:** Email as first-class evidence; optional connector for metadata.

**Effort (rough):** Small for D1–D2; Medium for D3.

---

## Part 2: Vanta Gap List – Implementation Options (for prioritisation)

These are the same gaps as in **COMP-AI-VS-VANTA-GAP-LIST.md**, turned into **concrete workstreams** you can prioritise.

| ID | Workstream | What to build | Effort (rough) | Doc/Ops overlap |
|----|-------------|----------------|----------------|------------------|
| **G1** | **Test runner + scheduler** | Job that runs control tests on a schedule (cron or queue); store results; optionally create evidence from pass/fail. | High | No → **Done:** Airflow DAG `comp_ai_control_tests`; COMP-AI-AIRFLOW-RUNNER-DESIGN.md. |
| **G2** | **More automated tests** | Expand test library (e.g. 10–50 tests for GitHub, then IdP, then cloud). | High | No |
| **G3** | **IdP integration** | One IdP (e.g. Okta): MFA, users, groups; automated evidence. | High | No |
| **G4** | **One cloud integration** | e.g. AWS or Azure: list resources, run checks, evidence. | Very high | No |
| **G5** | **Document / policy evidence** | Phase A + B (+ optional C) above. | Medium | **Yes – Part 1** |
| **G6** | **Policy / System Description** | Auto-generate SOC 2 System Description; control–policy mapping. | Medium | Partial (policies) |
| **G7** | **Alerts** | Slack or email when test fails or gap created. | Medium | **Yes** — Airflow DAG `comp_ai_alerts`; log + optional Slack |
| **G8** | **Vanta Control Set** | Adopt or mirror control/requirement IDs and names. | Medium | No |
| **G9** | **AI evidence review** | “Review all evidence for control X” → AI flags gaps and suggests fixes. | Medium | Overlaps B5 (analyse document). |

---

## Part 3: How to Choose What to Prioritise

**Step 1 – Choose direction**

- **Operational / administrative first (non-code):** Prioritise **Part 1 – Document/Operational**: Phase A → B → C (and optionally D). This is **G5** in the gap list. Good if your users need policy/document/Excel/email evidence and org-wide scan before doubling down on tech integrations.
- **Technical / Vanta-parity first:** Prioritise **G1 (test runner)** and **G3 (IdP)** or **G4 (one cloud)** from Part 2. Good if you want to close the “automated tests + integrations” gap first.
- **Audit readiness (policies/docs):** Prioritise **G5 (Phase A+B)** and **G6 (System Description / policy mapping)**. Good for “auditor-ready” without full automation.

**Step 2 – Pick one workstream to start**

Suggested ordering **if you want balance**:

1. **Phase A (Doc evidence)** – Small effort; unblocks ops/admin users immediately; no new infra.
2. **G7 (Alerts)** or **G1 (Test runner)** – Alerts are quick win; test runner unlocks all future automated evidence.
3. **Phase B (Upload + AI analysis)** – Document analysis and control mapping; closes part of “policies/audit docs” gap.
4. Then either **G3 (IdP)** or **Phase C (Scan for org)** depending on whether you prioritise technical or operational “run for organisation”.

**Step 3 – Track in this doc**

- When you decide the priority order, add a short “**Current priority**” section at the top of this doc, e.g. “Q1: Phase A → Alerts → Phase B.”
- When a phase or workstream is done, mark it in the plan and in **COMP-AI-VS-VANTA-GAP-LIST.md**.

---

## Part 4: Current priority (to fill in)

*Use this section to record what you chose to prioritise.*

| Order | Workstream | Status |
|-------|------------|--------|
| 1 | Phase A – Document evidence | ✅ Done |
| 2 | **G1 Test runner (Airflow)** | ✅ Done — Airflow DAG `comp_ai_control_tests` runs tests and records results; see COMP-AI-AIRFLOW-RUNNER-DESIGN.md |
| 3 | **G7 Alerts** | ✅ Done — DAG `comp_ai_alerts`; log + optional Slack (SLACK_WEBHOOK_URL) |
| 4 | **Phase B – Upload + AI analysis** | Done — upload API, text extraction (PDF/txt), analyse API, UI upload + Analyse button |
| … | | |

**Runner:** All scheduled/automated Comp-AI jobs use **Airflow** as runner. Event-based sync is the fallback if coordination between Comp-AI and Airflow becomes a problem (see COMP-AI-AIRFLOW-RUNNER-DESIGN.md).

---

## References

- **Document/operational scope:** `lianel/dc/docs/status/COMP-AI-DOCUMENT-OPERATIONAL-COMPLIANCE.md`
- **Vanta gap list:** `lianel/dc/docs/status/COMP-AI-VS-VANTA-GAP-LIST.md`
- **Strategy:** `lianel/dc/docs/status/STRATEGY-COMP-AI-VANTA-ROADMAP.md`
- **Implementation status:** `lianel/dc/docs/deployment/COMP-AI-IMPLEMENTATION-STATUS.md`
