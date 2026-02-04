# Comp-AI vs Vanta: Gap List and Progress

**Purpose:** Track the gap between our open-source Comp-AI service and Vanta so we know what’s left to build for a true replacement.  
**Last updated:** January 2026.

---

## 1. Vanta at a glance (reference)

| Dimension | Vanta (approx.) |
|-----------|------------------|
| **Automated tests** | 1,200+ tests, run continuously (e.g. hourly) |
| **Integrations** | 300+ (AWS, Azure, GCP, Okta, GitHub, Jira, Slack, 1Password, etc.) |
| **Frameworks** | 20+ (SOC 2, ISO 27001, GDPR, HIPAA, PCI DSS, NIST CSF, FedRAMP, HITRUST, etc.) |
| **Evidence** | Largely automated (90%+); Evidence tab with test criteria and resource-level results |
| **AI** | Reviews evidence, flags gaps, suggests remediation (including code snippets) |
| **Policies / audit docs** | Auto-generated (e.g. SOC 2 System Description), control–policy mapping |
| **Remediation** | Workflows, step-by-step guidance, Slack/email alerts, ticketing integration |
| **Customisation** | Custom tests, scoping by product/team/region, custom frameworks |
| **Deployment** | SaaS only; no self-hosted / open-source option |

---

## 2. Gap list: Vanta has it, we don’t (yet)

### 2.1 Automated testing and evidence

| Gap | Vanta | Comp-AI today | Effort (rough) |
|-----|--------|----------------|-----------------|
| **Volume of automated tests** | 1,200+ tests | Few (seed only); tests defined in DB, results recorded via API; **no scheduler** that runs tests automatically | High |
| **Test execution engine** | Runs tests on a schedule (e.g. hourly), stores results | No runner; we only **store** test definitions and **record** results (POST result) | High |
| **Evidence from tests** | Tests produce evidence automatically (pass/fail + artifacts) | Evidence is manual or from GitHub only; no “evidence from test run” | Medium |
| **Transparent test logic** | Evidence tab shows criteria, logic, resource-level results | No “test criteria / logic” view for auditors | Medium |

### 2.2 Integrations

| Gap | Vanta | Comp-AI today | Effort (rough) |
|-----|--------|----------------|-----------------|
| **Breadth** | 300+ integrations | **1** (GitHub: last commit, branch protection) | Very high |
| **IdP / IAM** | Okta, Azure AD, etc. (MFA, users, groups) | None | High |
| **Cloud** | AWS (40+ resources), Azure, GCP | None | Very high |
| **Ticketing / comms** | Jira, Slack (alerts, remediation) | None | Medium |
| **Other common tools** | 1Password, Notion, HR (e.g. ADP), etc. | None | High |

### 2.3 Policies and audit documents

| Gap | Vanta | Comp-AI today | Effort (rough) |
|-----|--------|----------------|-----------------|
| **System Description (SOC 2)** | Auto-generated | Not implemented | Medium |
| **Policy generation / mapping** | Auto-maps controls to policies | Not implemented | Medium |
| **Policy library / templates** | Pre-built policy content | Not implemented | Medium |
| **Document / non-code evidence** | Upload policies, .docx/.pdf/.xlsx; AI extracts and maps to controls | Evidence is link/manual or GitHub only; no document upload or doc-level AI analysis | Medium (see COMP-AI-DOCUMENT-OPERATIONAL-COMPLIANCE.md) |

### 2.4 Alerts and notifications

| Gap | Vanta | Comp-AI today | Effort (rough) |
|-----|--------|----------------|-----------------|
| **Alerts on failure** | Email, Slack when tests fail or gaps appear | None | Medium |
| **Remediation reminders** | Notifications to owners | None | Low–medium |

### 2.5 AI and UX

| Gap | Vanta | Comp-AI today | Effort (rough) |
|-----|--------|----------------|-----------------|
| **AI reviews evidence** | Flags gaps, suggests fixes (e.g. code snippets) | We have AI remediation **suggest** per control (and Phase 7.1 UI); no “review all evidence” or code suggestions | Medium |
| **AI gap/risk analysis** | Prioritisation, risk view | Planned (Phase 7.2); not built | Low–medium |
| **Structured “apply” from AI** | One-click apply suggestions | Not implemented (Phase 7.3 optional) | Low |

### 2.6 Frameworks and control set

| Gap | Vanta | Comp-AI today | Effort (rough) |
|-----|--------|----------------|-----------------|
| **Framework coverage** | 20+ frameworks, deep per-framework | SOC 2, ISO 27001, GDPR (seeded); HIPAA/PCI/NIST in prompts only (no full DB seed) | Medium |
| **Vanta Control Set alignment** | Native | Strategy mentions using/mirroring Vanta Control Set; **not done** | Medium |
| **Custom frameworks** | Supported | Schema supports it; no UI or import flow | Low–medium |

### 2.7 Enterprise and scale

| Gap | Vanta | Comp-AI today | Effort (rough) |
|-----|--------|----------------|-----------------|
| **Scoping** | By product, team, region | Single scope (all controls in one pool) | Medium |
| **SSO / auth** | Native SSO, multiple IdPs | Keycloak only | Low (already sufficient for many) |
| **Multi-tenant / multi-org** | Yes | No (single tenant) | High if ever needed |
| **Auditor / read-only access** | Dedicated auditor experience | Same UI; export suffices for many | Low–medium |

---

## 3. What we have (Comp-AI today)

- **Service:** Rust API, Keycloak auth, Postgres, Ollama (or mock), deploy pipeline.
- **Controls & evidence:** Controls, requirements, control–requirement mapping, evidence (manual + GitHub), gaps (controls with no evidence).
- **Remediation:** Tasks per control (assignee, due date, status, notes); **AI remediation suggest** (API + Phase 7.1 UI “Get AI suggestion” / “Use in notes”).
- **Tests:** Schema and API for test definitions per control and recording results; **no automatic test runner**.
- **Audit export:** JSON/CSV export (controls + requirements + evidence), optional framework filter.
- **AI chat:** Framework-aware Q&A (SOC 2, ISO 27001, GDPR, etc.), history.
- **Frameworks in DB:** SOC 2, ISO 27001, GDPR (expanded requirements in 016); HIPAA/PCI/NIST in prompts only.
- **Integrations:** GitHub only (last commit, branch protection).
- **Open source / self-hosted:** Yes; no vendor lock-in.

---

## 4. “How long are we?” – Progress vs Vanta

Rough **percentage of “Vanta-like” capability** (by area, then overall):

| Area | Weight (approx.) | We have (approx.) | Notes |
|------|------------------|-------------------|--------|
| **Control & requirement model** | 15% | ~90% | Core model and mapping done; more frameworks and Vanta Control Set alignment pending |
| **Evidence collection** | 20% | ~15% | Manual + GitHub only; no auto evidence from tests, no cloud/IdP |
| **Automated testing** | 25% | ~5% | Schema + APIs; no runner, no 1,200-style test library |
| **Integrations** | 20% | ~1% | 1 vs 300+ |
| **Remediation workflow** | 10% | ~70% | Tasks + AI suggest + UI; missing alerts and ticketing |
| **Policies / audit docs** | 5% | 0% | Not started |
| **AI (reviews, gaps, suggest)** | 5% | ~50% | Suggest + chat; no “review evidence” or code snippets |

**Overall (weighted):** we are roughly **20–25%** of the way to “Vanta parity” by capability breadth.  
By **foundation and core workflow** (controls, evidence, gaps, remediation, export, AI chat + suggest): we are much closer (e.g. **50–60%** of the “core” product), but **automation and integrations** are the big gaps.

### 4.1 What would move the needle most

1. **Test runner + more automated tests** — Run tests on a schedule, store results, optionally create evidence from passes. Gets us from “we record results” to “we produce evidence.”
2. **More integrations** — Next high-impact: IdP (e.g. Okta) and one cloud (e.g. AWS or GitHub deeper). Each integration adds automated evidence.
3. **Vanta Control Set** — Adopt or mirror for standard control/requirement IDs and names; improves alignment with auditors and Vanta users.
4. **Alerts** — Slack/email when a test fails or a gap is created; improves remediation behaviour.
5. **Policy / System Description** — Auto-docs for SOC 2 (and others) improve audit readiness.

---

## 5. Summary

- **Gap list:** Section 2 is the structured gap (automated tests & runner, 300+ integrations, policy/audit docs, alerts, AI evidence review, framework breadth, Vanta Control Set).
- **Progress:** We are about **20–25%** toward full Vanta-like breadth; **50–60%** on core control/evidence/remediation/export/AI workflow.
- **Next priorities** to close the gap: (1) test execution engine + scheduled runs, (2) 2–3 more high-value integrations (e.g. IdP, cloud), (3) Vanta Control Set alignment, (4) alerts, (5) policy/System Description generation.

This doc should be updated as we ship test runner, new integrations, policies, and alerts.
