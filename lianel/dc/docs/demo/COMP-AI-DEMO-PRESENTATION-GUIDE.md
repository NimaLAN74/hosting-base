# Comp-AI: Demo & Presentation Guide

Use this to run a live demo or client presentation of the Comp-AI UI and service.

**How it helps organisations:** For risk analysis and remediation, see **[COMP-AI-VALUE-RISK-AND-REMEDIATION.md](COMP-AI-VALUE-RISK-AND-REMEDIATION.md)** — coverage vs gaps, evidence, remediation tasks, and export.

---

## 0. If Controls / Gaps / Evidence are empty

The Comp-AI UI reads from the **same database** as the Comp-AI service. If you see no controls, no gaps, and empty lists, the **comp_ai** schema may not have seed data.

**Do not run the commands below locally.** Run them **on the server** (over SSH) where the Comp-AI service and PostgreSQL run.

### Fix: run on the server only

1. **SSH into the server**, then copy-paste this single block (adjust `cd` path if your repo is not under `/root/hosting-base`):

   ```bash
   # 1) See which DB the Comp-AI container uses (match this in .env)
   docker exec lianel-comp-ai-service env | grep POSTGRES

   # 2) Ensure .env in lianel/dc has same POSTGRES_DB and POSTGRES_HOST, then run migrations
   cd /root/hosting-base/lianel/dc
   COMP_AI_MIGRATION_USER=postgres bash scripts/deployment/run-comp-ai-migrations.sh

   # 3) Restart Comp-AI so it picks up any env change (optional)
   cd /root/hosting-base/lianel/dc
   docker compose -f docker-compose.infra.yaml -f docker-compose.comp-ai.yaml up -d comp-ai-service
   ```

2. **Refresh the Comp-AI UI** in the browser.

The script runs migrations **009–018** (018 seeds 3 controls + requirements + tests) and **verifies** that `comp_ai.controls` has at least one row; if not, it exits with an error and tells you to align `POSTGRES_DB` with the container.

**Alternative:** Re-run the **Deploy Comp AI Service to Production** workflow; it runs the same migrations on the remote DB via the pipeline. After it succeeds, refresh the Controls page.

---

## 1. Pipeline status

The **Run Comp-AI Demo and Report** workflow was triggered. To run it again:

- **GitHub:** Actions → **Run Comp-AI Demo and Report** → **Run workflow**
- **CLI:** `gh workflow run "Run Comp-AI Demo and Report"`

After a successful run, download the **comp-ai-demo-report** artifact for the Markdown report and raw JSON/CSV.

---

## 2. Open the Comp-AI UI

1. **URL:** https://www.lianel.se (or your deployed frontend URL).
2. **Log in** with your Keycloak account (SSO).
3. From the **Dashboard**, click **Comp AI** (or go directly to https://www.lianel.se/comp-ai).
4. **Navigation:** From any Comp-AI page, use the sidebar links: **Controls & Evidence**, **Request history**, **Monitoring**. The Controls page sidebar also links back to **Comp AI chat**.

---

## 3. Demo flow (step-by-step)

### A. Comp AI chat (AI-powered compliance Q&A)

**Route:** `/comp-ai` (main Comp AI page)

1. **Framework (optional):** Choose **General** or a framework (e.g. **SOC 2**, **ISO 27001**, **GDPR**) so answers are framed for that standard.
2. **Prompt:** Type a compliance question, e.g.:
   - *“What does CC6.1 require for logical access?”*
   - *“Summarize GDPR Article 32 in one paragraph.”*
   - *“What are the main differences between SOC 2 and ISO 27001 for access control?”*
3. Click **Submit Request**. The AI response appears in the conversation; you can ask **follow-ups** in the same thread.
4. **New conversation:** Use **New conversation** to start a clean thread.
5. **Sidebar:** Point out **Controls & Evidence** and **History** links for the rest of the demo.

**Talking points:** Framework-aware answers, multi-turn chat, processing time shown.

---

### B. Controls & evidence (audit readiness)

**Route:** `/comp-ai/controls` (or link from Comp AI sidebar)

1. **Controls list:** Show the three seeded controls (e.g. MFA, Access review, Security monitoring).
2. **Click one control** to open detail:
   - **Mapped requirements** (SOC 2, ISO 27001, GDPR codes).
   - **Evidence** section (empty or existing items).
3. **Add evidence (manual):**
   - Type, source, description, optional link.
   - Submit to attach evidence to that control.
4. **GitHub evidence (if configured):**
   - Owner, repo, type (e.g. last commit / branch protection).
   - Submit to pull evidence from GitHub and link to the control.
5. **Remediation (per control):**
   - Assignee, due date (DD/MM/YYYY), status (open / in progress / done), notes.
   - **Start remediation** or **Update remediation** to save.

**Talking points:** One control → many framework requirements; evidence reuse; remediation workflow.

---

### C. Gaps (controls without evidence)

**On the same Controls page:**

1. Use the **Gaps** view / button (controls with no evidence).
2. Show which controls still need evidence for audit readiness.
3. Optionally open one gap → add evidence or set remediation as in step B.

**Talking points:** Clear view of what’s missing; supports prioritisation.

---

### D. Export (audit pack)

**On the Controls page:**

1. **Export as JSON** or **Export as CSV**.
2. Open or share the file to show an audit-style export: controls, requirements, evidence in one pack.
3. Optional: filter by framework if the UI exposes it (or mention the API supports `?framework=soc2` etc.).

**Talking points:** One-click export for auditors; same data the demo script uses for the report artifact.

---

### E. History (past requests)

**Route:** `/comp-ai/history`

1. Show **request history**: past prompts, responses, model, timing.
2. Use it to show that all Comp AI chat usage is logged and reviewable.

**Talking points:** Traceability, compliance, and reuse of past Q&A.

---

### F. Monitoring (service health)

**Route:** `/comp-ai/monitoring`

1. Show **Service Health Status** (e.g. healthy / unhealthy).
2. Use **Refresh** to re-check.
3. Mention auto-refresh (e.g. every 30s) and that health is public (no login required for the health endpoint).

**Talking points:** Operational visibility and reliability.

---

## 4. Quick reference: URLs (after login)

| What              | URL                          |
|-------------------|------------------------------|
| Comp AI chat      | /comp-ai                     |
| Controls & evidence | /comp-ai/controls          |
| History           | /comp-ai/history             |
| Monitoring        | /comp-ai/monitoring          |

Base: **https://www.lianel.se** (or your frontend base URL).

---

## 5. API-only: AI remediation suggestion

The **AI remediation suggestion** (POST `/api/v1/controls/:id/remediation/suggest`) is not yet in the UI. For a demo you can:

- Use **Swagger:** https://www.lianel.se/comp-ai-api/swagger-ui (or your Comp-AI API base + `/swagger-ui`), find the remediation/suggest endpoint, and call it with a control ID and optional `context`.
- Or use the **demo script** (or the workflow artifact), which calls this endpoint and includes the suggestion in the report.

---

## 6. Tips for a smooth presentation

1. **Before the demo:** Log in once, open Comp AI and Controls in two tabs; do one test question and one control click so the first load is fast.
2. **Browser:** Use a clean window or incognito if you need a fresh login; avoid other heavy tabs.
3. **Frameworks:** Use **SOC 2** or **GDPR** for the first chat so the framework dropdown is clearly used.
4. **Fallback:** If the live site is slow or down, use the **sample report** (`COMP-AI-DEMO-REPORT-SAMPLE.md`) and the **demo report artifact** from the last successful workflow run to walk through the same data offline.

---

*Pipeline: run **Run Comp-AI Demo and Report** for a fresh report. Report artifact = real data; sample = structure only.*
