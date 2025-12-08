# D7 — Country & Region Mapping  
Version 1.0

## 1. Purpose
Define mapping rules between ISO countries, NUTS regions and ENTSO-E bidding zones.

## 2. Country ↔ NUTS0
- ISO code == NUTS0 code (e.g. SE ↔ SE, DE ↔ DE)

## 3. NUTS Hierarchy
- NUTS0 → NUTS1 → NUTS2 (strict hierarchy)

## 4. ENTSO-E Bidding Zones
Examples:
- Sweden: SE1, SE2, SE3, SE4
- Norway: NO1–NO5

Mapped to:
- Country (ISO/NUTS0)
- Approximate overlays for regional analysis (optional)

## 5. OSM Integration
OSM features aggregated to NUTS2 using spatial joins.
