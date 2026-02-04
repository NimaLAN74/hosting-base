# How Comp-AI Helps Organisations with Risk Analysis and Remediation

This document explains how the Comp-AI controls, evidence, and remediation features support **risk analysis** and **remediation work** in practice.

---

## Risk analysis

Organisations need to understand **where they stand** against frameworks (e.g. SOC 2, ISO 27001, GDPR) and **where the risk is** (uncovered controls, missing evidence).

| Comp-AI feature | How it supports risk analysis |
|-----------------|------------------------------|
| **Controls ↔ framework requirements** | Each internal control is mapped to one or more framework requirements (e.g. CC6.1, A.9.4.2, Art. 32). You see exactly which requirements are addressed by which controls. |
| **Gaps (controls without evidence)** | The **Gaps** view lists controls that have **no evidence** attached. These are the areas where you cannot yet demonstrate compliance — i.e. higher risk or audit exposure. |
| **Evidence per control** | Evidence is linked to controls (and thus to requirements). You can see which controls are “covered” by evidence and which are not, and prioritise by risk or auditor focus. |
| **Audit export (JSON/CSV)** | One-click export of controls, requirements, and evidence gives a snapshot for **internal risk reporting** or for auditors. Filter by framework to focus on one standard. |
| **Framework-aware AI chat** | Ask questions in the context of a framework (e.g. “What does CC6.1 require?”). Supports **risk and control design** and helps teams understand requirements before closing gaps. |

**In short:** Comp-AI gives a clear view of **coverage** (which requirements are addressed by controls) and **gaps** (where evidence is missing), so organisations can prioritise risk and focus on the right controls.

---

## Remediation work

Remediation is the work of **closing gaps**: collecting evidence, assigning owners, and tracking progress until controls are adequately supported.

| Comp-AI feature | How it supports remediation |
|-----------------|-----------------------------|
| **Gaps list** | Shows which controls need work. Teams can pick a control and start collecting evidence or assign remediation. |
| **Remediation tasks (per control)** | For each control you can set **assignee**, **due date**, **status** (open / in progress / done), and **notes**. This tracks who does what and by when. |
| **Evidence collection** | Add evidence manually (type, source, description, link) or via integrations (e.g. GitHub: last commit, branch protection). Use **Document**, **Policy**, **Spreadsheet**, or **Email** for operational/admin evidence (e.g. link to SharePoint, Drive). Evidence is linked to the control so the gap can be closed. |
| **AI remediation suggestion** | For a control, the **remediation/suggest** API (and future UI) can suggest concrete steps (e.g. “Assign owner, set due date, collect evidence for …”). Helps standardise and speed up remediation. |
| **Tests per control** | Automated or manual tests can be defined per control; last result and details are stored. Supports **ongoing compliance** and shows that remediation is not one-off but monitored. |

**In short:** Comp-AI supports the full remediation loop: **identify** (gaps), **assign** (remediation tasks), **collect** (evidence), **track** (status, tests), and **export** (for audit).

---

## End-to-end flow

1. **Risk view** — Use Controls, Gaps, and Export to see coverage and where evidence is missing.
2. **Prioritise** — Focus on high-risk or auditor-critical controls; use remediation tasks to assign owners and dates.
3. **Remediate** — Add evidence (manual or integrated), update remediation status, run or record tests.
4. **Reassess** — Gaps shrink as evidence is added; export again for reporting or audit.

Comp-AI does not replace a full GRC platform but gives organisations a **focused way** to map controls to frameworks, see gaps, and run remediation with evidence and task tracking in one place.
