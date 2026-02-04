# Comp-AI: Airflow as Runner for Scheduled and Automated Tasks

**Purpose:** Use **Airflow** as the single runner for all scheduled and automated Comp-AI jobs (control tests, document scan, gap analysis, etc.). If sync between Comp-AI and Airflow becomes a problem, use an **event-based** solution for coordination.

**Status:** Design agreed; control tests and alerts DAGs implemented.

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
| **Document scan (Phase C)** | `comp_ai_scan_documents` (future) | On-demand or weekly | POST /api/v1/scan/documents (when implemented) |
| **Gap / risk analysis** | `comp_ai_gap_analysis` (future) | Weekly or on-demand | POST /api/v1/analysis/gaps (when implemented) |
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
- **Variables** (recommended): `COMP_AI_BASE_URL` (e.g. `http://lianel-comp-ai-service:3002` or `https://www.lianel.se`), `COMP_AI_TOKEN` (Bearer token for a service account or machine user). Set in Airflow UI or via env.
- **Optional:** Airflow Connection (e.g. HTTP) with token in password; DAG code reads connection and sets `Authorization: Bearer <token>`.
- Token must be refreshed periodically (Keycloak) or use a long-lived service-account token; refresh can be a separate Airflow task or external process.

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

## 7. Future DAGs (when APIs exist)

- **comp_ai_scan_documents:** Call `POST /api/v1/scan/documents` (Phase C); schedule weekly or trigger via event.
- **comp_ai_gap_analysis:** Call `POST /api/v1/analysis/gaps` (Phase 7.2); schedule weekly; optionally feed into alerts.

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
- **Airflow DAGs:** `lianel/dc/dags/comp_ai_control_tests_dag.py`, `lianel/dc/dags/comp_ai_alerts_dag.py`, `lianel/dc/dags/utils/comp_ai_client.py`.
