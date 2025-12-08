# D2 — ENTSO-E Electricity Data Inventory  
Version 1.0

## 1. Purpose
Describe high-frequency electricity data from ENTSO-E Transparency Platform.

## 2. Dataset Categories
- Load (electricity demand)
- Generation by production type
- Cross-border physical flows
- Day-ahead and intraday prices (optional)
- Balancing and reserves data
- Outages and availability

## 3. Typical Fields
- `datetime` (local or UTC)
- `country` / `bidding_zone`
- `resolution` (PT60M, PT15M, etc.)
- `productionType` (for generation)
- `value` (MW or MWh)
- `quality` flag

## 4. Granularity
- Temporal: hourly or 15-minute intervals
- Spatial: country and bidding zones (e.g. SE1–SE4)

## 5. Known Issues
- Missing periods
- Differences in production type reporting between TSOs
- DST / timezone complications

## 6. Use in Platform
- High-resolution time series for forecasting and variability analysis
- Operational complement to Eurostat’s annual macro data
