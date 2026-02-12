# EU Date Format Verification (11/02/2026)

## Scope Verified

- Backend (`comp-ai-service`): DLP scan date defaults and formatting are now `DD/MM/YYYY`.
- Frontend (`CompAIControls`, `CompAIAuditDocs`): date inputs use EU text format (`DD/MM/YYYY`) instead of native ISO date pickers.
- Shared helpers:
  - Rust: added `parse_eu_or_iso_date()` (EU-first with ISO fallback for compatibility).
  - JS: existing centralized formatter (`dateFormat.js`) remains the canonical UI formatter.
- Script output: demo report generator now prints `Generated` timestamp using day-first date.

## Explicit Exceptions (Do Not Touch)

- ISO 8601 timestamps used for machine protocols remain unchanged (e.g., JWT/token claims, `timestamp_utc`, RFC3339 fields).
- Energy/ENTSO-E query/date parameters and DAG internals that rely on ISO-compatible parsing remain unchanged where integrations or SQL paths require ISO date semantics.

## Validation Checklist

- ✅ `ReadLints` on all modified files: no linter diagnostics.
- ⚠️ Local `cargo test` could not be executed in this environment:
  - `cargo` binary is not installed in the sandbox.
  - Docker socket access is blocked in the sandbox.
- ✅ New Rust unit test added for mixed-format parser:
  - `parse_eu_or_iso_accepts_both_inputs` in `comp-ai-service/src/utils.rs`.

## Suggested Runtime Verification (outside sandbox)

Run from `lianel/dc/comp-ai-service`:

```bash
docker run --rm \
  -e RUSTUP_TOOLCHAIN=nightly \
  -v "$PWD":/workspace \
  -w /workspace \
  rustlang/rust:nightly \
  bash -lc "cargo test"
```
