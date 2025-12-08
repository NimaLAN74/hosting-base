# D8 — Time Alignment Rules  
Version 1.0

## 1. Purpose
Align temporal aspects of Eurostat (annual), ENTSO-E (hourly/15-min), OSM (snapshot) and NUTS (vintages).

## 2. Canonical Time
- Timestamps stored as UTC ISO 8601
- Annual metrics represent full calendar years

## 3. ENTSO-E DST Handling
- Convert local-time series to UTC using timezone database
- Mark duplicated/missing hours during DST transitions

## 4. Aggregation
- Hourly → daily: sum hourly values
- Daily → annual: sum daily values
- No artificial disaggregation of annual Eurostat values

## 5. OSM Snapshot
- Use `snapshot_year = extraction_date.year` when attaching features to time series.
