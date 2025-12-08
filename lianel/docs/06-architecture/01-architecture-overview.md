# D16 — Architecture Overview  
Version 1.0

## 1. Purpose
Provide a high-level view of the future technical architecture.

## 2. Layers
- Ingestion (Eurostat, ENTSO-E, OSM, GISCO)
- Standardisation and harmonisation
- Storage (relational database and/or data lake)
- ML preparation (feature engineering, dataset curation)
- Serving layer (APIs, dashboards) in later phases

## 3. High-Level Flow
Sources → Ingestion → Standardisation → Harmonisation → Curated Storage → ML / Analytics → External consumers
