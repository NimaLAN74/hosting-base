# Stock Exchange Monitoring and Analysis Service – Function List and Priorities

**Purpose:** Define the function list and priority order for a new stock exchange monitoring and analysis service, based on market research and best practices (2024–2025).

**Use:** Use this as the product backlog and scope baseline; implement in priority order (P0 → P1 → P2 → P3).

**Geographic scope:** Monitoring is **global in vision**; the **MVP focuses on EU markets** (exchanges, symbols, data sources and licensing for European venues). North America and other regions are explicitly **post-MVP** (Phase 2+). See `lianel/dc/stock-monitoring/docs/EU-MARKETS-MVP.md` for target exchanges and data choices.

---

## 1. Research Summary – Best Services and Capabilities

### Market surveillance (institutional)

- **Nasdaq SMARTS** and similar platforms focus on: real-time monitoring, AI/ML anomaly detection, case management, 200+ markets and data feeds, audit trails, and regulatory reporting. Used by exchanges and regulators.

### Retail / prosumer monitoring

- Leading platforms (e.g. Stock Alarm, Koyfin, Fidelity ATP, Trade-Ideas) emphasize: **real-time and delayed data**, **multi-condition alerts** (price, %, technicals, volume, news), **watchlists and portfolios**, **screeners**, and **multi-channel notifications** (push, email, SMS).

### Data and APIs

- **Alpha Vantage** – Real-time + historical, technical indicators, free tier; NASDAQ-licensed.  
- **IEX Cloud** – Integrity-focused, REST + streaming, SQL-style access.  
- **Polygon** – Free tier, clear docs, lightweight API.  
- **Financial Modeling Prep (FMP)** – Real-time, historical, fundamentals, estimates.  
- **Quandl / Intrinio** – Deeper research and alternative data.

### Compliance and audit

- **SEC Rule 613 (CAT)** and similar regimes require: full order/quote/trade audit trail, timestamps (ms or finer), unique IDs, tamper-proof storage, retention (e.g. 6+ months), and electronic analysis capability. Even non-CAT platforms benefit from **audit logging** and **immutable records** for trust and future regulation.

### Portfolio and analytics

- Expected capabilities: **performance metrics** (returns, volatility, Sharpe/Sortino, drawdowns), **benchmark comparison**, **attribution** (allocation vs selection), **risk views** (market, credit, liquidity), and **drill-down** from portfolio to security level.

---

## 2. Priority Levels

- **P0 – Must-have:** Core value; service is not credible without these.  
- **P1 – High:** Important for daily use and differentiation.  
- **P2 – Medium:** Improves UX and depth; can follow MVP.  
- **P3 – Nice-to-have:** Advanced or niche; post-MVP.

---

## 3. Prioritized Function List

### P0 – Must-have (MVP)

| # | Function | Description | Notes |
|---|----------|-------------|--------|
| P0.1 | **Real-time / near-real-time quotes** | Live (or 15–20 min delayed) prices for selected symbols. | **MVP: EU markets only** (Euronext, XETRA, LSE, etc.). Foundation for all monitoring; choose EU-capable data provider (e.g. Alpha Vantage, Polygon, IEX, FMP). |
| P0.2 | **Watchlist** | User-defined list of symbols; persistent per user. | Essential for “monitor my universe.” |
| P0.3 | **Price-based alerts** | Alerts when price crosses a level (above/below) or % change threshold. | Most requested feature in retail monitoring. |
| P0.4 | **Alert notifications** | Deliver alerts via at least one channel: email and/or in-app/push. | Without delivery, alerts have limited value. |
| P0.5 | **Basic dashboard** | Single screen: watchlist prices, last update time, simple trend (e.g. up/down vs previous close). | Minimal “control room” view. |
| P0.6 | **Authentication and user management** | Secure login, user identity, and (if multi-tenant) isolation of watchlists/alerts per user. | Reuse existing auth (e.g. Keycloak) where possible. |
| P0.7 | **Audit logging** | Log material actions: alert creation/change/trigger, watchlist changes, (optional) quote requests. Immutable, timestamped. | Needed for trust and future compliance. |

### P1 – High (first post-MVP)

| # | Function | Description | Notes |
|---|----------|-------------|--------|
| P1.1 | **Technical indicator alerts** | Alerts on RSI, MACD, SMA/EMA crosses (e.g. golden/death cross), or other configurable indicators. | Reduces need for external charting for basic signals. |
| P1.2 | **Volume and anomaly alerts** | Volume spike (e.g. vs average), unusual activity. | Complements price alerts. |
| P1.3 | **Portfolio / positions view** | User can enter positions (symbol, quantity, optional cost); view current value and P&L. | Turns monitoring into “my book” view. |
| P1.4 | **Multiple notification channels** | Add SMS and/or push (mobile/web) alongside email. | Users expect choice of channel. |
| P1.5 | **Historical data and charts** | Historical OHLC (and volume) for symbols; simple interactive chart (e.g. line/candle) with range selector. | Needed for context and backtesting alerts. |
| P1.6 | **Market screener (basic)** | Filter symbols by price, % change, volume, (optional) sector/industry). List results; optionally add to watchlist. | Discovery and prioritization. |
| P1.7 | **Alert management hub** | List all alerts; search/filter; enable/disable; edit/delete; show recent triggers. | Scalability and clarity as alert count grows. |
| P1.8 | **Data source and licensing clarity** | Document data source(s), delay (real-time vs delayed), and terms of use. | Legal and user trust. |

### P2 – Medium (enhanced product)

| # | Function | Description | Notes |
|---|----------|-------------|--------|
| P2.1 | **Multi-condition alerts** | Combine conditions (e.g. price > X and RSI < 30 and volume > 2× average). | Fewer false positives, better signal quality. |
| P2.2 | **Benchmark comparison** | Compare portfolio or watchlist performance vs index (e.g. S&P 500) over chosen period. | Standard expectation for “analysis.” |
| P2.3 | **Performance metrics** | Time-weighted return, volatility, Sharpe (or Sortino), max drawdown for portfolio. | Basic risk/return analytics. |
| P2.4 | **News and earnings integration** | News headlines or earnings dates for symbols; optional alerts on earnings. | Context for price moves. |
| P2.5 | **Extended hours** | Pre/post market quotes and optional alerts. | Important for active traders. |
| P2.6 | **Export and reporting** | Export watchlist, alerts, or portfolio to CSV/Excel; simple scheduled report (e.g. daily digest email). | Operational and audit support. |
| P2.7 | **API for power users** | REST API to read watchlists, alerts, (optional) quotes and trigger alerts programmatically. | Enables automation and integration. |

### P3 – Nice-to-have (advanced)

| # | Function | Description | Notes |
|---|----------|-------------|--------|
| P3.1 | **Multi-asset** | Extend beyond equities to ETFs, forex, crypto, or commodities. | Depends on data provider and target users. |
| P3.2 | **Performance attribution** | Decompose returns into allocation vs selection (and optionally currency). | For more advanced portfolio analysis. |
| P3.3 | **Custom benchmarks** | User-defined benchmark (e.g. weighted basket). | For institutional or sophisticated users. |
| P3.4 | **AI/ML signals** | Anomaly detection or suggested alerts based on patterns. | Differentiator; requires data and model ops. |
| P3.5 | **Mobile app** | Native or PWA for phones with push and core watchlist/alert features. | Improves engagement and retention. |
| P3.6 | **Collaboration / sharing** | Shared watchlists or alert templates, team workspaces. | For teams and small firms. |

---

## 4. Suggested Implementation Order

**Phase 1 – MVP (P0)**  
Data pipeline (one provider), watchlist CRUD, real-time/delayed quotes, price alerts, one notification channel (e-mail), basic dashboard, auth, audit log.

**Phase 2 – Core (P1)**  
Technical and volume alerts, portfolio view, more channels, historical data + charts, basic screener, alert hub, data licensing docs.

**Phase 3 – Analysis (P2)**  
Multi-condition alerts, benchmark comparison, performance metrics, news/earnings, extended hours, export/reports, optional read/alert API.

**Phase 4 – Advanced (P3)**  
Multi-asset, attribution, custom benchmarks, AI signals, mobile, collaboration – as roadmap and resources allow.

---

## 5. Non-functional Priorities

| Area | Priority | Notes |
|------|----------|--------|
| **Latency** | P0 | Quote and alert delivery must feel “real-time” (within provider and channel limits). |
| **Uptime** | P0 | High availability for quote and alert delivery; planned maintenance windows. |
| **Security** | P0 | Auth, secrets, HTTPS, and audit trail integrity. |
| **Data accuracy** | P0 | Correct symbols, prices, and timestamps; clear handling of corrections. |
| **Scalability** | P1 | Design for many symbols and many users/alerts from day one where feasible. |
| **Cost** | P1 | Data and infra cost vs pricing model; start with one provider and one region if needed. |

---

## 6. References (research)

- Nasdaq Market Surveillance (SMARTS) – institutional surveillance features.  
- Retail platforms: Stock Alarm, Koyfin, Fidelity Active Trader Pro, Trade-Ideas – alerts, watchlists, screeners.  
- Data APIs: Alpha Vantage, IEX Cloud, Polygon, Financial Modeling Prep – coverage, real-time, historical.  
- SEC Rule 613 (Consolidated Audit Trail) and audit trail program elements – compliance context.  
- Portfolio analytics: MSCI, JustETF, Quant Reports – performance and attribution expectations.

---

**Document owner:** Product / engineering.  
**Next step:** Turn P0 list into tickets, choose data provider and stack, then implement Phase 1 (MVP).
