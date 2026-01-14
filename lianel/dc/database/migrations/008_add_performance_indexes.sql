-- Migration: Add Performance Indexes
-- Date: 2026-01-14
-- Purpose: Optimize queries based on performance analysis
-- 
-- Performance Analysis Findings:
-- - ml_dataset_forecasting_v1: 1,831 sequential scans, 493,020 tuples read
-- - ml_dataset_geo_enrichment_v1: 463 sequential scans
-- - ml_dataset_clustering_v1: 80 sequential scans
-- - Common filters: cntr_code, year, region_id

-- ============================================================================
-- ML Dataset Indexes
-- ============================================================================

-- ml_dataset_forecasting_v1 indexes
-- Common filters: cntr_code, year
CREATE INDEX IF NOT EXISTS idx_forecasting_cntr_code ON ml_dataset_forecasting_v1(cntr_code);
CREATE INDEX IF NOT EXISTS idx_forecasting_year ON ml_dataset_forecasting_v1(year);
CREATE INDEX IF NOT EXISTS idx_forecasting_cntr_year ON ml_dataset_forecasting_v1(cntr_code, year);
-- For ORDER BY year DESC, cntr_code
CREATE INDEX IF NOT EXISTS idx_forecasting_year_cntr ON ml_dataset_forecasting_v1(year DESC, cntr_code);

-- ml_dataset_clustering_v1 indexes
-- Common filters: cntr_code, year
CREATE INDEX IF NOT EXISTS idx_clustering_cntr_code ON ml_dataset_clustering_v1(cntr_code);
CREATE INDEX IF NOT EXISTS idx_clustering_year ON ml_dataset_clustering_v1(year);
CREATE INDEX IF NOT EXISTS idx_clustering_cntr_year ON ml_dataset_clustering_v1(cntr_code, year);

-- ml_dataset_geo_enrichment_v1 indexes
-- Common filters: cntr_code, year, region_id
CREATE INDEX IF NOT EXISTS idx_geo_enrich_cntr_code ON ml_dataset_geo_enrichment_v1(cntr_code);
CREATE INDEX IF NOT EXISTS idx_geo_enrich_year ON ml_dataset_geo_enrichment_v1(year);
CREATE INDEX IF NOT EXISTS idx_geo_enrich_region ON ml_dataset_geo_enrichment_v1(region_id);
CREATE INDEX IF NOT EXISTS idx_geo_enrich_cntr_year ON ml_dataset_geo_enrichment_v1(cntr_code, year);
CREATE INDEX IF NOT EXISTS idx_geo_enrich_region_year ON ml_dataset_geo_enrichment_v1(region_id, year);

-- ============================================================================
-- fact_energy_annual indexes (for API queries)
-- ============================================================================

-- Common filters: country_code, year, product_code, flow_code
-- Common ORDER BY: year DESC, country_code
CREATE INDEX IF NOT EXISTS idx_energy_country ON fact_energy_annual(country_code) WHERE source_system = 'eurostat';
CREATE INDEX IF NOT EXISTS idx_energy_year ON fact_energy_annual(year) WHERE source_system = 'eurostat';
CREATE INDEX IF NOT EXISTS idx_energy_country_year ON fact_energy_annual(country_code, year) WHERE source_system = 'eurostat';
CREATE INDEX IF NOT EXISTS idx_energy_year_country ON fact_energy_annual(year DESC, country_code) WHERE source_system = 'eurostat';
CREATE INDEX IF NOT EXISTS idx_energy_product ON fact_energy_annual(product_code) WHERE source_system = 'eurostat';
CREATE INDEX IF NOT EXISTS idx_energy_flow ON fact_energy_annual(flow_code) WHERE source_system = 'eurostat';

-- Composite index for common query pattern: country_code + year + product_code
CREATE INDEX IF NOT EXISTS idx_energy_country_year_product ON fact_energy_annual(country_code, year, product_code) WHERE source_system = 'eurostat';

-- ============================================================================
-- dim_region indexes (for JOINs)
-- ============================================================================

-- Common JOIN: ON e.country_code = r.cntr_code AND r.level_code = 0
CREATE INDEX IF NOT EXISTS idx_region_cntr_level ON dim_region(cntr_code, level_code);
CREATE INDEX IF NOT EXISTS idx_region_level_cntr ON dim_region(level_code, cntr_code);

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON INDEX idx_forecasting_cntr_year IS 'Composite index for filtering ML forecasting dataset by country and year';
COMMENT ON INDEX idx_energy_country_year IS 'Composite index for filtering energy data by country and year';
COMMENT ON INDEX idx_region_cntr_level IS 'Composite index for JOINs between fact_energy_annual and dim_region';
