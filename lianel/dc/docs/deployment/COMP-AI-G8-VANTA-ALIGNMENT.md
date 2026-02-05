# G8: Vanta (or external) control set alignment

This runbook describes how to align Comp-AI controls with an external control set (e.g. Vanta) using **external IDs**.

---

## Concepts

- **internal_id** — Comp-AI’s stable identifier for a control (e.g. `CC6.1`, `A.5.1`). Set by migrations and used in exports and APIs.
- **external_id** — Optional identifier from an external framework or tool (e.g. Vanta control ID). Used for auditor alignment and reporting; does not change how controls are stored.

Audit export (JSON/CSV) and control detail APIs include `external_id` when set, so auditors can match controls to your external control set.

---

## How to set external IDs

### Per control (UI)

1. Open **Comp AI → Controls & Evidence**.
2. Select a control.
3. In the control detail, use **External ID (e.g. Vanta)** and click **Save external ID**.

### Bulk (CSV-style)

1. Open **Comp AI → Controls & Evidence**.
2. In the **Audit (Phase 5)** section, find **G8: Align with external control set**.
3. Paste lines in the form: `internal_id,external_id` (one per line).
   - Example: `CC6.1,vanta-cc6.1`
   - To clear external_id for a control, use `internal_id,` (trailing comma or nothing after comma).
4. Click **Apply bulk external IDs**.
5. The result shows how many controls were updated and which internal_ids were not found.

### API

- **PATCH /api/v1/controls/:id** — body `{ "external_id": "vanta-123" }` or `{ "external_id": null }` to clear.
- **POST /api/v1/controls/bulk-external-id** — body `{ "updates": [ { "internal_id": "CC6.1", "external_id": "vanta-cc6.1" }, ... ] }`. Omit `external_id` or set to null to clear.

---

## Requirements

Requirement rows also have an **external_id** field (e.g. for framework requirement codes). Bulk update of requirement external IDs is not implemented in this runbook; controls are the primary alignment point for most audits.

---

## References

- Implementation plan: **COMP-AI-IMPLEMENTATION-PLAN-DOC-OPS-AND-GAPS.md** (G8).
- Gap list: **COMP-AI-VS-VANTA-GAP-LIST.md** (§2.6 Vanta Control Set alignment).
