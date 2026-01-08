-- ============================================================================
-- EU Energy & Geospatial Intelligence Platform - Database Schema DDL
-- ============================================================================
-- Version: 1.0
-- Date: January 7, 2026
-- Database: PostgreSQL 17 + PostGIS
-- ============================================================================

-- ============================================================================
-- EXTENSIONS
-- ============================================================================

-- Enable PostGIS for geospatial data
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS postgis_topology;

-- Enable pg_stat_statements for query performance monitoring
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- ============================================================================
-- SCHEMA
-- ============================================================================

-- Create schema if it doesn't exist (optional - can use public schema)
-- CREATE SCHEMA IF NOT EXISTS energy;

-- ============================================================================
-- DIMENSION TABLES
-- ============================================================================

-- ----------------------------------------------------------------------------
-- dim_country: Country dimension table
-- ----------------------------------------------------------------------------
CREATE TABLE dim_country (
    country_code VARCHAR(2) NOT NULL PRIMARY KEY,
    country_name VARCHAR(100) NOT NULL,
    nuts0_code VARCHAR(2) NOT NULL,
    iso_alpha3 VARCHAR(3),
    eu_member BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    CONSTRAINT chk_country_code CHECK (LENGTH(country_code) = 2),
    CONSTRAINT chk_nuts0_code CHECK (LENGTH(nuts0_code) = 2)
);

COMMENT ON TABLE dim_country IS 'Country dimension table with ISO 3166-1 alpha-2 codes';
COMMENT ON COLUMN dim_country.country_code IS 'ISO 3166-1 alpha-2 country code (e.g., SE, DE, FR)';
COMMENT ON COLUMN dim_country.nuts0_code IS 'NUTS level 0 code (same as country_code for EU countries)';
COMMENT ON COLUMN dim_country.eu_member IS 'Whether the country is an EU member state';

-- Indexes
CREATE INDEX idx_country_nuts0 ON dim_country(nuts0_code);
CREATE INDEX idx_country_name ON dim_country(country_name);

-- ----------------------------------------------------------------------------
-- dim_region: NUTS region dimension table with geospatial data
-- ----------------------------------------------------------------------------
CREATE TABLE dim_region (
    region_id VARCHAR(10) NOT NULL PRIMARY KEY,
    level_code INTEGER NOT NULL,
    cntr_code VARCHAR(2) NOT NULL,
    name_latn VARCHAR(255),
    nuts_name VARCHAR(255),
    mount_type VARCHAR(50),
    urbn_type VARCHAR(50),
    coast_type VARCHAR(50),
    area_km2 DOUBLE PRECISION,
    geometry GEOMETRY(MULTIPOLYGON, 3035),  -- EPSG:3035 for analysis (area calculations)
    geometry_wgs84 GEOMETRY(MULTIPOLYGON, 4326),  -- EPSG:4326 for display (web maps)
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    CONSTRAINT chk_level_code CHECK (level_code IN (0, 1, 2)),
    CONSTRAINT chk_region_id CHECK (LENGTH(region_id) BETWEEN 2 AND 10),
    CONSTRAINT fk_region_country FOREIGN KEY (cntr_code) 
        REFERENCES dim_country(country_code) ON DELETE RESTRICT
);

COMMENT ON TABLE dim_region IS 'NUTS region dimension table with geospatial boundaries';
COMMENT ON COLUMN dim_region.region_id IS 'NUTS identifier (e.g., SE11, DE12, FR10)';
COMMENT ON COLUMN dim_region.level_code IS 'NUTS level: 0=country, 1=major region, 2=sub-region';
COMMENT ON COLUMN dim_region.geometry IS 'Region boundary in EPSG:3035 (ETRS89/LAEA Europe) for analysis';
COMMENT ON COLUMN dim_region.geometry_wgs84 IS 'Region boundary in EPSG:4326 (WGS84) for display';

-- Indexes
CREATE INDEX idx_region_cntr_code ON dim_region(cntr_code);
CREATE INDEX idx_region_level ON dim_region(level_code);
CREATE INDEX idx_region_geometry ON dim_region USING GIST (geometry);
CREATE INDEX idx_region_geometry_wgs84 ON dim_region USING GIST (geometry_wgs84);

-- ----------------------------------------------------------------------------
-- dim_energy_product: Energy product dimension table
-- ----------------------------------------------------------------------------
CREATE TABLE dim_energy_product (
    product_code VARCHAR(20) NOT NULL PRIMARY KEY,
    product_name VARCHAR(255) NOT NULL,
    category VARCHAR(50),
    siec_code VARCHAR(10),  -- Eurostat SIEC classification
    renewable_flag BOOLEAN DEFAULT FALSE,
    fossil_flag BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW()
);

COMMENT ON TABLE dim_energy_product IS 'Energy product dimension table (Eurostat SIEC codes)';
COMMENT ON COLUMN dim_energy_product.product_code IS 'Eurostat product code (SIEC)';
COMMENT ON COLUMN dim_energy_product.siec_code IS 'Standard International Energy Classification code';

-- Indexes
CREATE INDEX idx_product_category ON dim_energy_product(category);
CREATE INDEX idx_product_renewable ON dim_energy_product(renewable_flag);
CREATE INDEX idx_product_siec ON dim_energy_product(siec_code);

-- ----------------------------------------------------------------------------
-- dim_energy_flow: Energy flow dimension table
-- ----------------------------------------------------------------------------
CREATE TABLE dim_energy_flow (
    flow_code VARCHAR(20) NOT NULL PRIMARY KEY,
    flow_name VARCHAR(255) NOT NULL,
    flow_category VARCHAR(50),  -- e.g., 'supply', 'transformation', 'consumption'
    description TEXT,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW()
);

COMMENT ON TABLE dim_energy_flow IS 'Energy balance flow dimension table (Eurostat nrg_bal codes)';
COMMENT ON COLUMN dim_energy_flow.flow_code IS 'Eurostat energy balance flow code (e.g., PPRD, IMP, EXP, FC)';

-- Indexes
CREATE INDEX idx_flow_category ON dim_energy_flow(flow_category);

-- ----------------------------------------------------------------------------
-- dim_production_type: Electricity production type dimension (future use)
-- ----------------------------------------------------------------------------
CREATE TABLE dim_production_type (
    code VARCHAR(20) NOT NULL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    renewable_flag BOOLEAN DEFAULT FALSE,
    technology_type VARCHAR(50),  -- e.g., 'solar', 'wind', 'hydro', 'nuclear'
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW()
);

COMMENT ON TABLE dim_production_type IS 'Electricity production type dimension (for future ENTSO-E data)';

-- Indexes
CREATE INDEX idx_production_renewable ON dim_production_type(renewable_flag);
CREATE INDEX idx_production_technology ON dim_production_type(technology_type);

-- ============================================================================
-- FACT TABLES
-- ============================================================================

-- ----------------------------------------------------------------------------
-- fact_energy_annual: Annual energy data (partitioned by year)
-- ----------------------------------------------------------------------------
CREATE TABLE fact_energy_annual (
    id BIGSERIAL,
    country_code VARCHAR(2) NOT NULL,
    year INTEGER NOT NULL,
    product_code VARCHAR(20),
    flow_code VARCHAR(20),
    sector_code VARCHAR(50),  -- Optional: sector classification if available
    value_gwh NUMERIC(15,3) NOT NULL,
    unit VARCHAR(10) DEFAULT 'GWh',
    source_system VARCHAR(50) NOT NULL DEFAULT 'eurostat',
    source_table VARCHAR(50),  -- e.g., 'nrg_bal_s', 'nrg_cb_e'
    harmonisation_version VARCHAR(20),
    ingestion_timestamp TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (id, year),
    CONSTRAINT chk_year CHECK (year >= 1960 AND year <= 2100),
    CONSTRAINT chk_value_positive CHECK (value_gwh >= 0),
    CONSTRAINT fk_energy_country FOREIGN KEY (country_code) 
        REFERENCES dim_country(country_code) ON DELETE RESTRICT,
    CONSTRAINT fk_energy_product FOREIGN KEY (product_code) 
        REFERENCES dim_energy_product(product_code) ON DELETE RESTRICT,
    CONSTRAINT fk_energy_flow FOREIGN KEY (flow_code) 
        REFERENCES dim_energy_flow(flow_code) ON DELETE RESTRICT
) PARTITION BY RANGE (year);

COMMENT ON TABLE fact_energy_annual IS 'Annual energy data fact table, partitioned by year';
COMMENT ON COLUMN fact_energy_annual.value_gwh IS 'Energy value in Gigawatt-hours (harmonized unit)';
COMMENT ON COLUMN fact_energy_annual.source_table IS 'Source Eurostat table (e.g., nrg_bal_s, nrg_cb_e)';
COMMENT ON COLUMN fact_energy_annual.harmonisation_version IS 'Version of harmonization rules applied';

-- Create partitions for years 1990-2024
-- Note: Partitions for 2025+ will be created via Airflow DAG
DO $$
DECLARE
    year_val INTEGER;
BEGIN
    FOR year_val IN 1990..2024 LOOP
        EXECUTE format('
            CREATE TABLE IF NOT EXISTS fact_energy_annual_%s 
            PARTITION OF fact_energy_annual
            FOR VALUES FROM (%s) TO (%s)',
            year_val, year_val, year_val + 1
        );
    END LOOP;
END $$;

-- Indexes on partitioned table (will be created on each partition)
CREATE INDEX idx_energy_country_year ON fact_energy_annual(country_code, year);
CREATE INDEX idx_energy_year_product ON fact_energy_annual(year, product_code);
CREATE INDEX idx_energy_year_flow ON fact_energy_annual(year, flow_code);
CREATE INDEX idx_energy_source_table ON fact_energy_annual(source_table);
CREATE INDEX idx_energy_ingestion ON fact_energy_annual(ingestion_timestamp);

-- ----------------------------------------------------------------------------
-- fact_electricity_timeseries: High-frequency electricity data (future)
-- ----------------------------------------------------------------------------
CREATE TABLE fact_electricity_timeseries (
    id BIGSERIAL PRIMARY KEY,
    timestamp_utc TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    country_code VARCHAR(2) NOT NULL,
    production_type_code VARCHAR(20),
    load_mw NUMERIC(12,2),
    generation_mw NUMERIC(12,2),
    source_system VARCHAR(50) DEFAULT 'entsoe',
    ingestion_timestamp TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    CONSTRAINT chk_timestamp_valid CHECK (timestamp_utc >= '2000-01-01'::timestamp),
    CONSTRAINT fk_electricity_country FOREIGN KEY (country_code) 
        REFERENCES dim_country(country_code) ON DELETE RESTRICT,
    CONSTRAINT fk_electricity_production_type FOREIGN KEY (production_type_code) 
        REFERENCES dim_production_type(code) ON DELETE RESTRICT
);

COMMENT ON TABLE fact_electricity_timeseries IS 'High-frequency electricity data (hourly/15-minute) from ENTSO-E (future)';
COMMENT ON COLUMN fact_electricity_timeseries.timestamp_utc IS 'Timestamp in UTC (ISO 8601)';

-- Indexes
CREATE INDEX idx_electricity_timestamp ON fact_electricity_timeseries(timestamp_utc);
CREATE INDEX idx_electricity_country_timestamp ON fact_electricity_timeseries(country_code, timestamp_utc);
CREATE INDEX idx_electricity_production_type ON fact_electricity_timeseries(production_type_code);

-- ----------------------------------------------------------------------------
-- fact_geo_region_features: Geospatial region features (future - OSM data)
-- ----------------------------------------------------------------------------
CREATE TABLE fact_geo_region_features (
    id BIGSERIAL PRIMARY KEY,
    region_id VARCHAR(10) NOT NULL,
    feature_name VARCHAR(100) NOT NULL,
    feature_value NUMERIC(15,3),
    snapshot_year INTEGER NOT NULL,
    source_system VARCHAR(50) DEFAULT 'osm',
    ingestion_timestamp TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    CONSTRAINT chk_snapshot_year CHECK (snapshot_year >= 2000 AND snapshot_year <= 2100),
    CONSTRAINT fk_geo_region FOREIGN KEY (region_id) 
        REFERENCES dim_region(region_id) ON DELETE RESTRICT,
    CONSTRAINT uk_geo_region_feature_year UNIQUE (region_id, feature_name, snapshot_year)
);

COMMENT ON TABLE fact_geo_region_features IS 'Geospatial features aggregated by NUTS region (future - OSM data)';
COMMENT ON COLUMN fact_geo_region_features.feature_name IS 'Feature name (e.g., building_residential_count, road_length_km)';

-- Indexes
CREATE INDEX idx_geo_region_year ON fact_geo_region_features(region_id, snapshot_year);
CREATE INDEX idx_geo_feature_name ON fact_geo_region_features(feature_name);

-- ============================================================================
-- METADATA TABLES
-- ============================================================================

-- ----------------------------------------------------------------------------
-- meta_ingestion_log: Data ingestion logging
-- ----------------------------------------------------------------------------
CREATE TABLE meta_ingestion_log (
    id BIGSERIAL PRIMARY KEY,
    source_system VARCHAR(50) NOT NULL,
    table_name VARCHAR(100) NOT NULL,
    ingestion_timestamp TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    records_inserted INTEGER DEFAULT 0,
    records_updated INTEGER DEFAULT 0,
    records_failed INTEGER DEFAULT 0,
    status VARCHAR(20) NOT NULL,  -- 'success', 'partial', 'failed'
    error_message TEXT,
    execution_time_seconds NUMERIC(10,2),
    metadata JSONB,  -- Additional metadata (source URL, parameters, etc.)
    CONSTRAINT chk_status CHECK (status IN ('success', 'partial', 'failed'))
);

COMMENT ON TABLE meta_ingestion_log IS 'Log of all data ingestion operations';
COMMENT ON COLUMN meta_ingestion_log.metadata IS 'JSON metadata (source URL, API parameters, etc.)';

-- Indexes
CREATE INDEX idx_ingestion_timestamp ON meta_ingestion_log(ingestion_timestamp);
CREATE INDEX idx_ingestion_source ON meta_ingestion_log(source_system);
CREATE INDEX idx_ingestion_status ON meta_ingestion_log(status);
CREATE INDEX idx_ingestion_table ON meta_ingestion_log(table_name);

-- ----------------------------------------------------------------------------
-- meta_data_quality: Data quality check results
-- ----------------------------------------------------------------------------
CREATE TABLE meta_data_quality (
    id BIGSERIAL PRIMARY KEY,
    table_name VARCHAR(100) NOT NULL,
    check_timestamp TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    quality_check VARCHAR(100) NOT NULL,  -- e.g., 'completeness', 'validity', 'consistency'
    result VARCHAR(20) NOT NULL,  -- 'pass', 'warn', 'fail'
    details JSONB,  -- Detailed results (counts, percentages, etc.)
    CONSTRAINT chk_result CHECK (result IN ('pass', 'warn', 'fail'))
);

COMMENT ON TABLE meta_data_quality IS 'Results of data quality validation checks';
COMMENT ON COLUMN meta_data_quality.details IS 'JSON with detailed quality metrics (e.g., missing_count, completeness_pct)';

-- Indexes
CREATE INDEX idx_quality_timestamp ON meta_data_quality(check_timestamp);
CREATE INDEX idx_quality_table ON meta_data_quality(table_name);
CREATE INDEX idx_quality_result ON meta_data_quality(result);

-- ============================================================================
-- VIEWS (Optional - for convenience)
-- ============================================================================

-- View: Energy data with country and product names
CREATE OR REPLACE VIEW v_energy_annual_enriched AS
SELECT 
    e.id,
    e.country_code,
    c.country_name,
    e.year,
    e.product_code,
    p.product_name,
    e.flow_code,
    f.flow_name,
    e.value_gwh,
    e.unit,
    e.source_table,
    e.ingestion_timestamp
FROM fact_energy_annual e
LEFT JOIN dim_country c ON e.country_code = c.country_code
LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
LEFT JOIN dim_energy_flow f ON e.flow_code = f.flow_code;

COMMENT ON VIEW v_energy_annual_enriched IS 'Energy annual data with dimension names joined';

-- ============================================================================
-- FUNCTIONS (Optional - for data quality checks)
-- ============================================================================

-- Function: Calculate data completeness for a table
CREATE OR REPLACE FUNCTION calculate_completeness(
    p_table_name TEXT,
    p_column_name TEXT,
    p_year INTEGER DEFAULT NULL
) RETURNS TABLE (
    total_rows BIGINT,
    non_null_rows BIGINT,
    completeness_pct NUMERIC
) AS $$
BEGIN
    RETURN QUERY
    EXECUTE format('
        SELECT 
            COUNT(*)::BIGINT as total_rows,
            COUNT(%I)::BIGINT as non_null_rows,
            ROUND(100.0 * COUNT(%I) / NULLIF(COUNT(*), 0), 2) as completeness_pct
        FROM %I
        WHERE ($1 IS NULL OR year = $1)',
        p_column_name, p_column_name, p_table_name
    ) USING p_year;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION calculate_completeness IS 'Calculate completeness percentage for a column in a table';

-- ============================================================================
-- GRANTS (Adjust based on your user setup)
-- ============================================================================

-- Example: Grant permissions to application users
-- GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO lianel_energy_writer;
-- GRANT SELECT ON ALL TABLES IN SCHEMA public TO lianel_energy_reader;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO lianel_energy_writer;

-- ============================================================================
-- END OF SCHEMA DDL
-- ============================================================================

