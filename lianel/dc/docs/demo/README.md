# Comp-AI Demo and Client Report

This folder holds **demo reports** produced by running all Comp-AI controls/tests APIs. Use them for client demos and audit-style summaries.

## How to generate a report

### Option 1: Run the script locally (or on the server)

You need a valid Bearer token for the Comp-AI API (same as when using the Comp AI UI).

1. **Get a token**  
   Log in to https://www.lianel.se → open **Comp AI** → open browser DevTools → **Network** → trigger any Comp AI request → copy the `Authorization: Bearer <token>` value (only the token part).

2. **Run the script** from repo root or from `lianel/dc`:
   ```bash
   COMP_AI_TOKEN='<paste_token_here>' bash lianel/dc/scripts/monitoring/run-comp-ai-demo-and-report.sh
   ```
   Or with Keycloak credentials (if your client allows Direct access grants):
   ```bash
   DEMO_USER=youruser DEMO_PASSWORD=yourpass \
   COMP_AI_KEYCLOAK_CLIENT_ID=comp-ai-service \
   COMP_AI_KEYCLOAK_CLIENT_SECRET=... \
   KEYCLOAK_URL=https://www.lianel.se/auth \
   bash lianel/dc/scripts/monitoring/run-comp-ai-demo-and-report.sh
   ```

3. **Output**
   - Markdown report: `lianel/dc/docs/demo/COMP-AI-DEMO-REPORT-<timestamp>.md`
   - Raw JSON/CSV: `lianel/dc/docs/demo/demo-output-<timestamp>/`

### Option 2: GitHub Actions (manual workflow)

1. **Add one of these as repository secrets** (Settings → Secrets and variables → Actions):
   - **Option A – Bearer token:** **`COMP_AI_TOKEN`** = token from browser (Comp AI → DevTools → Network → copy from a request).
   - **Option B – Keycloak (workflow gets token):** **`COMP_AI_DEMO_USER`**, **`COMP_AI_DEMO_PASSWORD`**. Optional: **`COMP_AI_KEYCLOAK_CLIENT_ID`** (default `comp-ai-service`), **`COMP_AI_KEYCLOAK_CLIENT_SECRET`**, **`KEYCLOAK_URL`** (default `https://www.lianel.se/auth`), **`KEYCLOAK_REALM`** (default `lianel`). The workflow has a **Get token from Keycloak** step that runs when `COMP_AI_TOKEN` is not set and fetches a fresh token each run.
2. **Actions** → **Run Comp-AI Demo and Report** → **Run workflow**.
3. After the run (green), download the **comp-ai-demo-report** artifact (Markdown report + raw JSON/CSV).

## What the demo covers

- Health check (public)
- List controls and control detail (with framework requirements)
- Controls without evidence (gaps)
- Audit export (JSON and CSV)
- Framework requirements (all + filtered by framework)
- Remediation tasks
- Evidence list
- Automated tests (all and per control)
- AI remediation suggestion (one sample per run)
- Frameworks list

## Sample report

See **COMP-AI-DEMO-REPORT-SAMPLE.md** for an example of the report structure. Replace with a real run for client-facing demos.
