-- Create fact_energy_annual table if it doesn't exist (simplified version without partitions)
-- Run this directly on the database

CREATE TABLE IF NOT EXISTS fact_energy_annual (
    id BIGSERIAL PRIMARY KEY,
    country_code VARCHAR(2) NOT NULL,
    year INTEGER NOT NULL,
    product_code VARCHAR(20),
    flow_code VARCHAR(20),
    sector_code VARCHAR(50),
    value_gwh NUMERIC(15,3) NOT NULL,
    unit VARCHAR(10) DEFAULT 'GWh',
    source_system VARCHAR(50) NOT NULL DEFAULT 'eurostat',
    source_table VARCHAR(50),
    harmonisation_version VARCHAR(20),
    ingestion_timestamp TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
    CONSTRAINT chk_year CHECK (year >= 1960 AND year <= 2100),
    CONSTRAINT chk_value_positive CHECK (value_gwh >= 0)
);

-- Create indexes if they don't exist
CREATE INDEX IF NOT EXISTS idx_energy_country_year ON fact_energy_annual(country_code, year);
CREATE INDEX IF NOT EXISTS idx_energy_year_product ON fact_energy_annual(year, product_code);
CREATE INDEX IF NOT EXISTS idx_energy_year_flow ON fact_energy_annual(year, flow_code);
CREATE INDEX IF NOT EXISTS idx_energy_source_table ON fact_energy_annual(source_table);
CREATE INDEX IF NOT EXISTS idx_energy_ingestion ON fact_energy_annual(ingestion_timestamp);

-- Grant permissions to airflow user
GRANT SELECT, INSERT, UPDATE ON fact_energy_annual TO airflow;
GRANT USAGE, SELECT ON SEQUENCE fact_energy_annual_id_seq TO airflow;
