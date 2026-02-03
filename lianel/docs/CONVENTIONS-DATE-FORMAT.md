# Date Format Convention (EU)

All new code, scripts, and documentation must emit human-readable dates using **day-first notation** (`DD/MM/YYYY`). This applies to:

- API responses intended for humans (CSV exports, monitoring scripts, run-books)
- User interface strings
- Markdown documentation and operational guides
- Shell/Python utilities that print or log dates for operators

## When to keep ISO 8601

Leave dates in ISO 8601 (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SSZ`) only when required by an upstream/downstream protocol, such as:

- External APIs that explicitly demand ISO (e.g., ENTSO-E, Eurostat)
- Database DATE/TIMESTAMP column serialization handled by drivers
- Audit logs consumed by automated tooling

Document every such exception inline with a comment.

## Helper functions

- **Rust (`comp-ai-service`)**: use `crate::utils::{format_eu_date, parse_eu_date}` or the `serde_eu_date` helpers instead of direct `format` / `parse_from_str` calls.
- **JavaScript/TypeScript** (frontend): add/update a central formatter that wraps your date library (`dayjs`, `Intl.DateTimeFormat`, etc.) with `DD/MM/YYYY`.
- **Python**: prefer `dt.strftime("%d/%m/%Y")`.
- **Shell**: use ``date +%d/%m/%Y`` for operator-facing echoes.

## Filename-safe variants

When file paths cannot contain `/`, format the date first (`DD/MM/YYYY`) and then substitute the slash with a dash (`-`). Example:

```bash
STAMP=$(date +%d/%m/%Y | tr '/' '-')
tar -czf "backup-${STAMP}.tar.gz" …
```

## Verification checklist

Before merging changes that introduce or modify date handling:

1. Confirm helper usage instead of bespoke format strings.
2. Update sample payloads/docs to reflect day-first notation.
3. Add/adjust tests covering the EU formatter or the serialized representation.
