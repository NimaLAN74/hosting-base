# Comp-AI: Airflow as Runner for Scheduled and Automated Tasks

**Purpose:** Use **Airflow** as the single runner for all scheduled and automated Comp-AI jobs (control tests, document scan, gap analysis, etc.). If sync between Comp-AI and Airflow becomes a problem, use an **event-based** solution for coordination.

**Status:** Design agreed; control tests, alerts, and organisation scan DAGs implemented.

---

## 1. Principle: Airflow as runner

- **All scheduled and automated Comp-AI tasks** are executed by **Airflow DAGs** (cron-like schedules or triggers).
- Comp-AI service remains **stateless** for job orchestration: it exposes **APIs** that Airflow (or tasks started by Airflow) call. It does not run its own cron or queue.
- Benefits: one place for scheduling, retries, monitoring, and logs (Airflow UI); reuse of existing Airflow infra and patterns.

---

## 2. Which jobs run in Airflow

| Job | DAG / task | Schedule (example) | Comp-AI API used |
|-----|------------|--------------------|-------------------|
| **Control tests** | `comp_ai_control_tests` | Daily (or per-test schedule later) | GET /api/v1/tests, POST /api/v1/controls/:id/tests/:test_id/result |
| **Organisation scan + monitoring (Phase C)** | `comp_ai_scan_documents` | Weekly Sun 08:00 UTC | POST /api/v1/scan/documents, then POST /api/v1/analysis/gaps |
| **Alerts (G7)** | `comp_ai_alerts` | Daily 07:00 UTC (after tests) | GET /api/v1/controls/gaps, GET /api/v1/tests; log + optional Slack |

---

## 3. Sync between Comp-AI and Airflow

- **Default:** Airflow **pulls** from Comp-AI (HTTP calls). Airflow runs on a schedule, calls Comp-AI APIs (e.g. list tests, post result). No push from Comp-AI to Airflow in the base design.
- **If sync becomes a problem** (e.g. Comp-AI needs to trigger a run when evidence is added, or we need near–real-time reaction):
  - **Event-based fallback:** Introduce a small **event channel**:
    - **Option A:** Comp-AI emits events (e.g. HTTP webhook to Airflow REST API to trigger a DAG run, or to a message queue). Airflow listens or is triggered by the queue.
    - **Option B:** Airflow polls Comp-AI more frequently, or Comp-AI writes a “pending jobs” table that Airflow reads (shared DB).
  - We **do not** implement event-based sync until we see a concrete sync problem; then we choose Option A or B and document it here.

---

## 4. Authentication and configuration

- Airflow needs to call Comp-AI APIs with **Bearer token** (Keycloak).
- **Option A – Stored token:** Set `COMP_AI_BASE_URL` and `COMP_AI_TOKEN` (Airflow Variables or env). Token must be **ASCII-only** (no ellipsis … or other non-ASCII). Refresh periodically (e.g. run `scripts/maintenance/set-comp-ai-airflow-token.sh` on the server) or use a long-lived token.
- **Option B – Client credentials (recommended):** Set `COMP_AI_BASE_URL` and `COMP_AI_KEYCLOAK_URL`, `COMP_AI_CLIENT_ID`, `COMP_AI_CLIENT_SECRET` (and optionally `COMP_AI_KEYCLOAK_REALM`, default `lianel`). The DAG fetches a fresh token each run via Keycloak; avoids 401 from expiry and non-ASCII in stored token.
- **Non-ASCII / 401:** If you see “non-ASCII characters” or “401 Unauthorized”, see runbook `COMP-AI-AIRFLOW-401-TOKEN.md` or re-run the token script with ASCII sanitization, or switch to Option B.

### 4.1. Airflow Variables reference

Set these in **Airflow → Admin → Variables** (or via env in the Airflow worker/scheduler). CLI: `airflow variables set KEY value`.

| Variable | Required | Used by | Description |
|----------|----------|---------|-------------|
| `COMP_AI_BASE_URL` | Yes | All Comp-AI DAGs | Comp-AI API base URL (e.g. `https://www.lianel.se/api/v1/comp-ai` or `http://comp-ai-service:3002`). No trailing slash. |
| `COMP_AI_TOKEN` | If no Keycloak | All Comp-AI DAGs | Bearer token for Comp-AI API. ASCII-only. Prefer Keycloak (below) to avoid expiry. |
| `COMP_AI_KEYCLOAK_URL` | If no token | All Comp-AI DAGs | Keycloak base URL (e.g. `https://auth.lianel.se`). Used with client_credentials to get a fresh token. |
| `COMP_AI_CLIENT_ID` | If using Keycloak | All Comp-AI DAGs | Keycloak client id (e.g. `comp-ai-service`). |
| `COMP_AI_CLIENT_SECRET` | If using Keycloak | All Comp-AI DAGs | Keycloak client secret. Prefer Airflow secret backend for this. |
| `COMP_AI_KEYCLOAK_REALM` | No | All Comp-AI DAGs | Keycloak realm; default `lianel`. |
| `COMP_AI_SCAN_DOCUMENTS_CONFIG` | No | `comp_ai_scan_documents` | JSON: `{"control_id": N, "documents": [{"url": "..."}]}` or list of such jobs. Unset = no-op. |
| `COMP_AI_GAP_ANALYSIS_FRAMEWORK` | No | `comp_ai_scan_documents` | Optional framework filter for gap analysis (e.g. `soc2`). |
| `SLACK_WEBHOOK_URL` | No | `comp_ai_alerts` | Slack webhook URL for alert notifications. Unset = log only. |

---

## 5. Implemented: Control tests DAG

- **DAG id:** `comp_ai_control_tests`
- **Schedule:** Daily (e.g. `0 6 * * *` 06:00 UTC); configurable.
- **Steps:**
  1. Get list of tests: `GET {COMP_AI_BASE_URL}/api/v1/tests` with `Authorization: Bearer {COMP_AI_TOKEN}`.
  2. For each test: run test logic (stub or real check; e.g. integration check), then `POST /api/v1/controls/{control_id}/tests/{test_id}/result` with `{ "result": "pass"|"fail"|"skipped", "details": "..." }`.
- **Test logic:** Initially a **stub** (return pass with details “Scheduled run (Airflow)”). Later, per-test-type logic (e.g. GitHub branch protection, IdP MFA check) can be added in the same DAG or in a shared Python module.
- **Helper:** `dags/utils/comp_ai_client.py` — `get_tests()`, `post_test_result()`, using Variables for base URL and token.

---

## 6. Implemented: Alerts DAG (G7)

- **DAG id:** `comp_ai_alerts`
- **Schedule:** Daily at 07:00 UTC (after `comp_ai_control_tests` at 06:00).
- **Steps:** `GET /api/v1/controls/gaps`, `GET /api/v1/tests`; filter tests with `last_result == 'fail'`; build summary; log; if `SLACK_WEBHOOK_URL` (Airflow Variable or env) is set, POST summary to Slack (or "No gaps, no failed tests" when clear).
- **Helper:** `comp_ai_client.get_gaps()`, `get_tests()`.

## 7. Implemented: Organisation scan + monitoring DAG (Phase C)

- **DAG id:** `comp_ai_scan_documents`
- **Schedule:** Weekly Sunday 08:00 UTC (`0 8 * * 0`).
- **Tasks:**
  1. **run_organisation_document_scan** — Reads Airflow Variable `COMP_AI_SCAN_DOCUMENTS_CONFIG` (JSON). For each job `{ "control_id": N, "documents": [ {"url": "https://..."}, ... ] }` calls `POST /api/v1/scan/documents`. Creates evidence from URLs for the whole organisation. If Variable unset or empty, no-op (0 created).
  2. **run_gap_analysis_monitoring** — Calls `POST /api/v1/analysis/gaps` (optional filter via `COMP_AI_GAP_ANALYSIS_FRAMEWORK`); logs AI gap/risk summary for monitoring.
- **Config (Airflow Variables):**
  - `COMP_AI_SCAN_DOCUMENTS_CONFIG` — JSON: single job `{"control_id": 2, "documents": [{"url": "..."}]}` or list of jobs `[{...}, {...}]`. Max 50 URLs per job (API limit).
  - `COMP_AI_GAP_ANALYSIS_FRAMEWORK` — Optional; e.g. `soc2` to filter gap analysis by framework.
- **Helper:** `comp_ai_client.post_scan_documents()`, `post_analysis_gaps()`.

---

## 8. Event-based sync (fallback, not yet implemented)

When we need Comp-AI and Airflow to stay in sync in a more reactive way:

1. **Define the problem** (e.g. “When evidence is uploaded, run analysis within 5 minutes”).
2. **Choose mechanism:** (A) Comp-AI calls Airflow REST API to trigger DAG run; (B) Comp-AI publishes to a queue (e.g. Redis, SQS), Airflow sensor or worker consumes; (C) shared DB “job queue” table that Airflow polls.
3. **Document** the choice and update this section with the chosen design and links to code.

---

## References

- **Implementation plan:** `COMP-AI-IMPLEMENTATION-PLAN-DOC-OPS-AND-GAPS.md` (G1 test runner, Phase C scan).
- **Comp-AI API:** `GET /api/v1/tests`, `POST /api/v1/controls/:id/tests/:test_id/result` (and future scan/analysis endpoints).
- **Airflow DAGs:** `comp_ai_control_tests_dag.py`, `comp_ai_alerts_dag.py`, `comp_ai_scan_documents_dag.py`, `lianel/dc/dags/utils/comp_ai_client.py`.
