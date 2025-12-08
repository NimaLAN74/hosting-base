# D12 — Metadata & Lineage  
Version 1.0

## 1. Purpose
Define how data provenance and processing stages are recorded.

## 2. Required Metadata Fields
- `src_system`
- `src_location`
- `extraction_date`
- `original_unit`
- `harmonisation_version`
- `processing_stage` (raw, standardised, harmonised, curated)

## 3. Lineage Stages
1. Source ingestion
2. Standardisation (units, codes)
3. Harmonisation (joins, mappings)
4. Aggregation (time and/or space)
5. ML curation (feature engineering)

Each curated dataset must carry enough metadata to trace back to its raw inputs.
