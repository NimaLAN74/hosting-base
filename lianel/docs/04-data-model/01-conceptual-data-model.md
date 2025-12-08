# D9 — Conceptual Data Model  
Version 1.0

## 1. Purpose
Describe key conceptual entities and relationships.

## 2. Entities
- Energy: annual macro energy flows per country/sector/product
- ElectricityTimeseries: high-frequency load/generation per country
- Region: NUTS-based spatial units
- OSMFeature: aggregated counts and densities per region
- MlDataset: curated feature sets for specific use cases

## 3. Relationships
- Energy belongs to a Region (country)
- ElectricityTimeseries aggregates into Regions
- OSMFeature enriches Regions
- MlDataset combines Energy, ElectricityTimeseries and OSMFeature per Region and year.
