# Comp-AI Demo Report (Sample)

**Generated**: Sample — run the demo script to get a live report  
**Environment**: https://www.lianel.se  

---

## 1. Summary

| Endpoint | Status | Notes |
|----------|--------|-------|
| GET /api/v1/comp-ai/health | ✅ 200 | Public health check |
| GET /api/v1/controls | ✅ 200 | |
| GET /api/v1/controls/gaps | ✅ 200 | |
| GET /api/v1/controls/export (JSON) | ✅ 200 | |
| GET /api/v1/requirements | ✅ 200 | |
| GET /api/v1/remediation | ✅ 200 | |
| GET /api/v1/evidence | ✅ 200 | |
| GET /api/v1/tests | ✅ 200 | |
| GET /api/v1/comp-ai/frameworks | ✅ 200 | |

---

## 2. Controls

Total controls: **3**

| Internal ID | Name | Category |
|-------------|------|----------|
| CTL-MFA-001 | Multi-factor authentication | Access control |
| CTL-ACCESS-001 | Access review process | Access control |
| CTL-MON-001 | Security monitoring | Monitoring |

---

## 3. Controls without evidence (gaps)

Count: **0** to **3** (depends on evidence collected).  
Controls listed here need evidence to be attached for audit readiness.

---

## 4. Framework requirements (sample)

Requirements are mapped from SOC 2, ISO 27001, and GDPR. Examples:

| Code | Title | Framework |
|------|-------|-----------|
| CC6.1 | Logical access – identity management | soc2 |
| A.9.4.2 | Secure log-on procedures | iso27001 |
| Art. 32 | Security of processing | gdpr |

---

## 5. Remediation tasks

Remediation tracks assignee, due date, and status per control.  
Use GET/PUT `/api/v1/controls/:id/remediation` to manage.

---

## 6. Automated tests per control

Each control can have one or more tests (e.g. integration, manual).  
Last run result is stored (pass / fail / skipped).

| Control ID | Test name | Type | Last result |
|------------|-----------|------|-------------|
| 1 | MFA enabled in IdP | integration | pass |
| 2 | Access review workflow exists | manual | — |
| 3 | SIEM/log ingestion health | integration | pass |

---

## 7. AI remediation suggestion (sample)

**Model used**: Ollama (e.g. tinyllama) or mock when unavailable.

**Suggestion** (example):

```
1. Assign an owner for this control.
2. Set a due date (e.g. 30 days).
3. Implement evidence collection for the mapped framework requirements.
4. Run the automated tests and record results via POST .../tests/:test_id/result.
```

---

## 8. Audit export

CSV and JSON export are produced by the demo script and saved in the run’s output directory for audit use.

---

*To generate a live report: run `run-comp-ai-demo-and-report.sh` with `COMP_AI_TOKEN` or Keycloak credentials. See docs/demo/README.md.*
