-- Phase 5: Additional Sources Integration
-- Migration 005: Add ENTSO-E and OSM tables
-- Date: 2026-01-13

-- ============================================================================
-- ENTSO-E Electricity Timeseries Table
-- ============================================================================

-- Production type dimension (if not exists)
CREATE TABLE IF NOT EXISTS dim_production_type (
    code VARCHAR(50) PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    renewable_flag BOOLEAN DEFAULT FALSE,
    category VARCHAR(50), -- e.g., 'solar', 'wind', 'hydro', 'nuclear', 'fossil'
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert common production types
INSERT INTO dim_production_type (code, name, renewable_flag, category) VALUES
    ('B01', 'Biomass', TRUE, 'renewable'),
    ('B02', 'Fossil Brown coal/Lignite', FALSE, 'fossil'),
    ('B03', 'Fossil Coal-derived gas', FALSE, 'fossil'),
    ('B04', 'Fossil Gas', FALSE, 'fossil'),
    ('B05', 'Fossil Hard coal', FALSE, 'fossil'),
    ('B06', 'Fossil Oil', FALSE, 'fossil'),
    ('B07', 'Fossil Oil shale', FALSE, 'fossil'),
    ('B08', 'Fossil Peat', FALSE, 'fossil'),
    ('B09', 'Geothermal', TRUE, 'renewable'),
    ('B10', 'Hydro Pumped Storage', FALSE, 'storage'),
    ('B11', 'Hydro Run-of-river and poundage', TRUE, 'renewable'),
    ('B12', 'Hydro Water Reservoir', TRUE, 'renewable'),
    ('B13', 'Marine', TRUE, 'renewable'),
    ('B14', 'Nuclear', FALSE, 'nuclear'),
    ('B15', 'Other renewable', TRUE, 'renewable'),
    ('B16', 'Solar', TRUE, 'renewable'),
    ('B17', 'Waste', FALSE, 'other'),
    ('B18', 'Wind Offshore', TRUE, 'renewable'),
    ('B19', 'Wind Onshore', TRUE, 'renewable'),
    ('B20', 'Other', FALSE, 'other')
ON CONFLICT (code) DO NOTHING;

-- Main electricity timeseries table
CREATE TABLE IF NOT EXISTS fact_electricity_timeseries (
    id BIGSERIAL PRIMARY KEY,
    timestamp_utc TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    country_code VARCHAR(2) NOT NULL,
    bidding_zone VARCHAR(10),
    production_type VARCHAR(50),
    load_mw NUMERIC(12,2),
    generation_mw NUMERIC(12,2),
    resolution VARCHAR(10) NOT NULL, -- PT60M (hourly), PT15M (15-min)
    quality_flag VARCHAR(20), -- 'actual', 'estimated', 'forecast'
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_country FOREIGN KEY (country_code) REFERENCES dim_country(country_code),
    CONSTRAINT fk_production_type FOREIGN KEY (production_type) REFERENCES dim_production_type(code),
    CONSTRAINT chk_resolution CHECK (resolution IN ('PT15M', 'PT60M', 'PT30M', 'PT1H'))
);

-- Indexes for time-series queries
CREATE INDEX IF NOT EXISTS idx_electricity_timestamp ON fact_electricity_timeseries(timestamp_utc);
CREATE INDEX IF NOT EXISTS idx_electricity_country_time ON fact_electricity_timeseries(country_code, timestamp_utc);
CREATE INDEX IF NOT EXISTS idx_electricity_production_type ON fact_electricity_timeseries(production_type);
CREATE INDEX IF NOT EXISTS idx_electricity_bidding_zone ON fact_electricity_timeseries(bidding_zone) WHERE bidding_zone IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_electricity_country_type_time ON fact_electricity_timeseries(country_code, production_type, timestamp_utc);

-- Partitioning by year (optional, can be added later if needed)
-- For now, we'll use regular table with indexes

-- ============================================================================
-- OSM Geospatial Features Table
-- ============================================================================

CREATE TABLE IF NOT EXISTS fact_geo_region_features (
    id BIGSERIAL PRIMARY KEY,
    region_id VARCHAR(10) NOT NULL,
    feature_name VARCHAR(100) NOT NULL,
    feature_value NUMERIC(12,2) NOT NULL,
    snapshot_year INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_region FOREIGN KEY (region_id) REFERENCES dim_region(region_id),
    CONSTRAINT chk_snapshot_year CHECK (snapshot_year >= 2000 AND snapshot_year <= 2100),
    UNIQUE(region_id, feature_name, snapshot_year)
);

-- Indexes for geo features
CREATE INDEX IF NOT EXISTS idx_geo_region ON fact_geo_region_features(region_id);
CREATE INDEX IF NOT EXISTS idx_geo_feature ON fact_geo_region_features(feature_name);
CREATE INDEX IF NOT EXISTS idx_geo_year ON fact_geo_region_features(snapshot_year);
CREATE INDEX IF NOT EXISTS idx_geo_region_year ON fact_geo_region_features(region_id, snapshot_year);

-- ============================================================================
-- Metadata Tables for Phase 5
-- ============================================================================

-- ENTSO-E ingestion log
CREATE TABLE IF NOT EXISTS meta_entsoe_ingestion_log (
    id BIGSERIAL PRIMARY KEY,
    ingestion_date DATE NOT NULL,
    country_code VARCHAR(2) NOT NULL,
    start_timestamp TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    end_timestamp TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    records_ingested INTEGER DEFAULT 0,
    status VARCHAR(20) NOT NULL, -- 'success', 'failed', 'partial'
    error_message TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_country_entsoe FOREIGN KEY (country_code) REFERENCES dim_country(country_code)
);

CREATE INDEX IF NOT EXISTS idx_entsoe_log_date ON meta_entsoe_ingestion_log(ingestion_date);
CREATE INDEX IF NOT EXISTS idx_entsoe_log_country ON meta_entsoe_ingestion_log(country_code);

-- OSM extraction log
CREATE TABLE IF NOT EXISTS meta_osm_extraction_log (
    id BIGSERIAL PRIMARY KEY,
    extraction_date DATE NOT NULL,
    region_id VARCHAR(10) NOT NULL,
    feature_types TEXT[], -- Array of feature types extracted
    features_extracted INTEGER DEFAULT 0,
    status VARCHAR(20) NOT NULL, -- 'success', 'failed', 'partial'
    error_message TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_region_osm FOREIGN KEY (region_id) REFERENCES dim_region(region_id)
);

CREATE INDEX IF NOT EXISTS idx_osm_log_date ON meta_osm_extraction_log(extraction_date);
CREATE INDEX IF NOT EXISTS idx_osm_log_region ON meta_osm_extraction_log(region_id);

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE fact_electricity_timeseries IS 'High-frequency electricity data from ENTSO-E Transparency Platform';
COMMENT ON TABLE fact_geo_region_features IS 'OpenStreetMap features aggregated to NUTS2 regions';
COMMENT ON TABLE dim_production_type IS 'Electricity production type dimension (ENTSO-E codes)';
COMMENT ON TABLE meta_entsoe_ingestion_log IS 'Log of ENTSO-E data ingestion runs';
COMMENT ON TABLE meta_osm_extraction_log IS 'Log of OSM feature extraction runs';

COMMENT ON COLUMN fact_electricity_timeseries.resolution IS 'Time resolution: PT15M (15-min), PT60M (hourly)';
COMMENT ON COLUMN fact_electricity_timeseries.quality_flag IS 'Data quality: actual, estimated, or forecast';
COMMENT ON COLUMN fact_geo_region_features.feature_name IS 'OSM feature name, e.g., power_plant_count, building_residential_area_km2';
COMMENT ON COLUMN fact_geo_region_features.snapshot_year IS 'Year when OSM data was extracted (snapshot)';
