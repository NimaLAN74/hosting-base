# Non-Code / Document & Operational Compliance: Tools and Comp-AI Extension

**Purpose:** Answer (1) what tools exist for scanning/monitoring/analysing non-code artefacts (emails, documents, Excel, etc.) for organisational and administrative compliance, and (2) whether we can use the current Comp-AI solution to build such a capability.

**Audience:** Organisations that need compliance not only for “technical” controls (code, infra, IdP) but for **operational and administrative** areas: policies, procedures, spreadsheets, emails, HR/finance docs.

---

## 1. What tools exist for non-code scan / monitor / analysis?

### 1.1 Compliance platforms (document-centric)

| Tool / area | What it does | Non-code focus |
|-------------|----------------|----------------|
| **Vanta** | Document upload (policies + evidence); Vanta AI extracts policy details, maps controls to policies; supports .docx, .pdf, .xlsx, .csv, images. Documents by section (HR, IT, policy, engineering, risk). | Yes: policies, spreadsheets, general docs as evidence; AI maps to controls. |
| **SecureFrame, Drata** | Similar: policy/document upload, control mapping, readiness dashboards. | Yes: policy and doc evidence. |
| **Microsoft Purview (DLP)** | Scans documents, emails, files for sensitive data; policy-based; on-prem and cloud. | Yes: content scanning, classification, DLP policies (not control mapping per se). |
| **OneTrust, TrustArc** | Privacy/governance; document and process workflows; policy management. | Yes: policies, procedures, records. |

### 1.2 Generic capabilities (often used with compliance)

- **Document management + classification:** SharePoint, Google Drive, Confluence — store and tag; some have basic retention/compliance.
- **Email:** M365/Google retention, eDiscovery, DLP — scan and retain; less often “map to control X”.
- **Content extraction + AI:** Use text from PDFs/Office/Excel (e.g. Apache Tika, cloud APIs) and run NLP/LLM to classify, summarise, or check “does this satisfy requirement Y?”.
- **Spreadsheet/Excel:** Parsed as data or text; then rules or AI for checks (e.g. “contains PII?”, “approval dates present?”).

So **yes, there are tools** that do scan/monitor/analysis for **non-code objects** (mails, documents, Excel, policies) for organisational and administrative compliance — either inside compliance platforms (Vanta-style) or via DLP/content/AI tooling. Few are open-source and self-hosted end-to-end.

---

## 2. Can we use our current Comp-AI solution to develop this?

**Yes.** The current design already supports it with extensions.

### 2.1 What we have today

- **Control–evidence model:** Evidence is linked to controls; each evidence has `type`, `source`, `description`, `link_url`. No restriction that evidence must be “code” or “technical”.
- **Evidence types:** Free-form (e.g. `manual`, `github_last_commit`). We can add `document`, `policy`, `spreadsheet`, `email` (or keep as `manual` with a clear source).
- **AI:** We have Ollama (or hosted) for chat and remediation suggest. We can add “analyse this content” (summary, control relevance, gap check).
- **Frameworks and requirements:** Same controls/requirements apply to operational and administrative controls (e.g. “Policy for access control”, “HR training records”). We don’t need a separate product — we need **evidence** that points to or contains non-code artefacts and **optional content analysis**.

### 2.2 What we’d add (extension path)

| Layer | Addition | Purpose |
|-------|----------|--------|
| **Evidence** | Document/policy/email evidence types; optional **file upload** or **URL** to doc store. | Treat docs, Excel, emails as first-class evidence linked to controls. |
| **Storage** | Store files (e.g. S3, local volume) or only metadata + link. | Support “upload policy” or “link to shared drive”. |
| **Content extraction** | Text extraction from PDF, Office, Excel (e.g. Tika, or external service). | Feed text to AI for analysis. |
| **AI analysis** | New endpoint or flow: “Analyse this document” — summary, which controls/requirements it might satisfy, gaps. | Same Ollama stack; prompt with control/requirement list + document text. |
| **Scan / monitor** | **Option A:** Batch job: scan a folder or API (SharePoint, Drive, S3), create/update evidence and run analysis. **Option B:** Integrations (e.g. M365, Google) that list docs/emails and optionally pull content. | “Run for organisation” on docs and admin artefacts. |
| **Operational view** | Optional: tag controls or requirements as “operational” / “administrative” or by department (HR, Finance, IT). Filter UI/export by these. | Help ops/admin teams see their part of compliance. |

So we **reuse** controls, requirements, evidence, frameworks, and AI; we **add** document-aware evidence, optional upload/storage, text extraction, AI document analysis, and a way to “scan” or “run” over many non-code objects (batch or integration).

---

## 3. Suggested roadmap (concise)

1. **Phase A – Document evidence (no new infra)**  
   - Evidence types: `document`, `policy`, `spreadsheet`, `email` (or descriptive `manual`).  
   - UI: when adding evidence, user can paste link (SharePoint, Drive, etc.) or upload a file (if we add upload).  
   - No content analysis yet; just link evidence to controls for operational/admin use.

2. **Phase B – Content extraction + AI analysis**  
   - Backend: text extraction from uploaded/linked files (e.g. Tika for PDF/Office; CSV/Excel parsed).  
   - New API: e.g. `POST /api/v1/evidence/:id/analyze` or `POST /api/v1/analyze/document` (text + control/requirement context) → AI returns summary + suggested control mapping + gaps.  
   - UI: “Analyse” button on document evidence; show AI result and optionally suggest which controls to link.

3. **Phase C – Scan/monitor for organisation**  
   - Batch: “Scan folder” or “Scan this list of URLs” (e.g. policy library) → create evidence + run analysis.  
   - Optional: integrations (e.g. SharePoint, Google Drive, or M365) to list and optionally fetch documents; run analysis and attach evidence to controls.  
   - Optional: “Operational / administrative” filter or dashboard so orgs can run and view compliance for non-technical areas.

4. **Phase D – Email**  
   - Evidence type “email” with source (e.g. “Retention notice from M365”) or integration that pulls metadata/summary (no need to store full bodies).  
   - Optional: DLP-style checks (e.g. “sensitive in email”) via integration or external tool; we store result as evidence.

---

## 4. Summary

| Question | Answer |
|----------|--------|
| **Are there tools for scan/monitor/analysis of non-code objects (mails, documents, Excel)?** | Yes: Vanta/SecureFrame/Drata (document + policy upload, AI mapping); Microsoft Purview (DLP, content); others (OneTrust, etc.). |
| **Something that runs for the organisation, especially operational/admin?** | Yes: same platforms support HR, IT, policy, risk sections; DLP and content tools scan org-wide. |
| **Can we use our current solution to develop this?** | Yes: same control/evidence/framework model; add document evidence types, optional upload + extraction, AI document analysis, and batch or integration-based “scan” for docs/operational artefacts. |
| **What to build first?** | Document/policy/email evidence types and links (and optional file upload); then AI “analyse document” endpoint and UI; then batch scan or one integration (e.g. SharePoint or folder) so we can “run for organisation” on non-code objects. |

This keeps one product (Comp-AI) for both technical and operational/administrative compliance, and extends it rather than building a separate system.

**Implementation plan:** For phased tasks (Phase A–D) and how this ties to the Vanta gap list so you can prioritise, see **`COMP-AI-IMPLEMENTATION-PLAN-DOC-OPS-AND-GAPS.md`**.
