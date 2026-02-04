# Phase 7: AI Tools for Risk Analysis and Remediation

**Goal:** Use AI to help users/customers with **risk analysis** and **remediation** (and optionally **retaliation**-related compliance). This phase adds tools and UI so the product directly supports those workflows.

**Status:** Plan defined; **7.1 UI for remediation suggest done.** Next: 7.2 AI gap/risk analysis.

---

## Current state (end of Phase 6)

- **AI chat:** Framework-aware Q&A (`POST /api/v1/process`); helps with “what does CC6.1 require?” etc.
- **AI remediation suggest (backend only):** `POST /api/v1/controls/:id/remediation/suggest` returns AI-generated steps (assignee, due date, actionable steps). **No UI** — users cannot trigger it from the app.
- **Controls, gaps, evidence, remediation tasks, export, requirements, tests:** All in place. Value doc: `COMP-AI-VALUE-RISK-AND-REMEDIATION.md`.

**Gap:** Users have no in-app way to get AI suggestions for a control, and no AI-assisted view of “risk” or “prioritise my gaps.”

---

## Phase 7 scope

| # | Deliverable | Purpose |
|---|-------------|--------|
| **7.1** | **UI for AI remediation suggest** | In Controls → control detail → Remediation section: button “Get AI suggestion” that calls the existing API and shows the suggestion; optional “Use in notes” to copy into remediation notes. |
| **7.2** | **AI-assisted gap / risk analysis** | New flow: user sees their gaps (controls with no evidence); AI summarises prioritisation, typical evidence types, and suggested order of work. Options: (A) dedicated endpoint `POST /api/v1/analysis/gaps` that takes gap list + optional framework and returns AI summary, or (B) pre-filled Comp-AI chat prompt with gap context so the same chat model answers. Prefer (A) for a structured “Risk summary” card; (B) for flexibility. |
| **7.3** | **Structured “apply” from AI (optional)** | If the AI suggests assignee/due date/evidence types, allow one-click apply: e.g. “Apply suggestion” that parses (or the API returns structured fields) and pre-fills remediation form or creates a task. Can follow 7.1 once the UI is used and we see what users need. |
| **7.4** | **Retaliation / whistleblower (optional)** | If “retaliation” means **whistleblower retaliation** compliance: add a small flow — e.g. framework or policy Q&A (“How should we handle retaliation reports?”), or a dedicated prompt context. Can be a later sub-phase once 7.1–7.2 are in use. |

---

## Recommended order

1. **7.1 UI for remediation suggest** — Immediate value; API exists. **Next step: implement this.**
2. **7.2 AI gap/risk analysis** — High value for “help me with risk and analysis”; either new endpoint or chat-with-context.
3. **7.3 Structured apply** — After 7.1, decide if we need structured API response and apply buttons.
4. **7.4 Retaliation** — After 7.1–7.2, if required by customers.

---

## 7.1 Implementation checklist (current focus)

- [x] **Frontend API:** `compAiApi.postRemediationSuggest(controlId, { context?: string })` → `{ suggestion, model_used }`.
- [x] **CompAIControls.js:** In the Remediation section (when a control is selected):
  - [x] State: `remediationSuggest`, `remediationSuggestLoading`, `remediationSuggestError`.
  - [x] Button “Get AI suggestion” (disabled while loading).
  - [x] Display suggestion in a card (with `model_used`).
  - [x] “Use in notes” copies suggestion into remediation notes (user can edit before saving).
- [x] **Error handling:** 503 (Ollama unavailable) → friendly message; 401/404 as usual.
- [x] **CSS:** `.comp-ai-remediation-suggest`, `.comp-ai-suggestion-card`, etc. in CompAI.css.

---

## 7.2 AI gap/risk analysis (next)

- **Option A – Dedicated endpoint:** `POST /api/v1/analysis/gaps`  
  - Body: `{ framework?: string }` (optional; defaults to all).  
  - Backend fetches gaps, builds a prompt (“These controls have no evidence: … Suggest prioritisation and evidence types.”), calls Ollama, returns `{ summary, model_used }`.  
  - Frontend: “Analyse my gaps” button on Controls (or Gaps view) that calls this and shows the summary in a modal or new section.

- **Option B – Chat with context:**  
  - Frontend builds a message like “My controls with no evidence are: [list]. Suggest prioritisation and what evidence to collect first.” and sends it to `POST /api/v1/process` with the chosen framework.  
  - No new endpoint; reuses chat. Less structured; good for ad-hoc questions.

Recommendation: **Option A** for a clear “Risk analysis” feature; Option B as a complement for follow-up questions.

---

## References

- Value doc: `lianel/dc/docs/demo/COMP-AI-VALUE-RISK-AND-REMEDIATION.md`
- Strategy: `lianel/dc/docs/status/STRATEGY-COMP-AI-VANTA-ROADMAP.md`
- Implementation status: `lianel/dc/docs/deployment/COMP-AI-IMPLEMENTATION-STATUS.md`
