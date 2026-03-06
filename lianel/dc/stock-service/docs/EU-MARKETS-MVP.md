# EU Markets MVP – Stock Monitoring

MVP scope: **European Union markets** (exchanges, symbols, data). Global (e.g. North America) is post-MVP.

## Target exchanges (EU)

- **Euronext** (Paris, Amsterdam, Brussels, Lisbon, Dublin, Oslo)
- **XETRA** / **Frankfurt** (Deutsche Börse)
- **London Stock Exchange** (LSE) – post-Brexit still core for EU users
- **SIX Swiss Exchange** (optional for EU-adjacent)
- **Borsa Italiana** (Milan)
- **BMV** (Spain) / **Nasdaq Nordic** (Stockholm, Helsinki, Copenhagen, Iceland) as needed

## Data sources (EU-focused)

- Prefer providers with EU coverage and (where required) licensing: Alpha Vantage, IEX Cloud, Polygon, FMP, or exchange/aggregator feeds.
- Ensure **symbol format** and **mic** (market identifier) align with EU exchanges (e.g. XPAR, XETR, XLON, SIX).
- **MVP implementation choice (current):** Backend quote endpoint uses Yahoo Finance as primary with short cache, Stooq as secondary fallback, and optional Alpha Vantage fallback when `STOCK_MONITORING_DATA_PROVIDER_API_KEY` is set.

## Symbols and identifiers

- Use **ISIN** and/or **RIC** where appropriate for EU instruments.
- Document symbol lists per exchange in this repo (e.g. `docs/symbols-eu-mvp.csv` or similar) when available.

## Licensing and compliance

- Data licensing and redistribution rules are provider- and exchange-specific; document chosen provider(s) and any EU-specific terms here.
- No US-only compliance (e.g. SEC Rule 613) required for EU-only MVP; add when expanding to North America.

## Out of scope for MVP

- North American exchanges (NYSE, Nasdaq US, etc.) – Phase 2+.
- Real-time L1/L2 for all venues – start with delayed or near-real-time where sufficient for alerts and dashboard.

## Implementation

- Build order and task list: see [IMPLEMENTATION-PLAN.md](./IMPLEMENTATION-PLAN.md) and [TASK-CHECKLIST.md](./TASK-CHECKLIST.md) in this folder.
