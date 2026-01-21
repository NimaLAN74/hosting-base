-- Create fact_electricity_timeseries table if it doesn't exist
-- This table stores high-frequency electricity load and generation data from ENTSO-E

CREATE TABLE IF NOT EXISTS fact_electricity_timeseries (
    id BIGSERIAL PRIMARY KEY,
    timestamp_utc TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    country_code VARCHAR(2) NOT NULL,
    bidding_zone VARCHAR(50),
    production_type VARCHAR(10),
    load_mw NUMERIC(12, 3),
    generation_mw NUMERIC(12, 3),
    resolution VARCHAR(10) NOT NULL DEFAULT 'PT60M',
    quality_flag VARCHAR(20) DEFAULT 'actual',
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(timestamp_utc, country_code, bidding_zone, production_type)
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_electricity_timeseries_timestamp ON fact_electricity_timeseries(timestamp_utc DESC);
CREATE INDEX IF NOT EXISTS idx_electricity_timeseries_country ON fact_electricity_timeseries(country_code);
CREATE INDEX IF NOT EXISTS idx_electricity_timeseries_production_type ON fact_electricity_timeseries(production_type);
CREATE INDEX IF NOT EXISTS idx_electricity_timeseries_country_timestamp ON fact_electricity_timeseries(country_code, timestamp_utc DESC);
CREATE INDEX IF NOT EXISTS idx_electricity_timeseries_date_range ON fact_electricity_timeseries(timestamp_utc, country_code, production_type);

-- Add comment
COMMENT ON TABLE fact_electricity_timeseries IS 'High-frequency electricity load and generation data from ENTSO-E Transparency Platform';
COMMENT ON COLUMN fact_electricity_timeseries.timestamp_utc IS 'UTC timestamp of the measurement';
COMMENT ON COLUMN fact_electricity_timeseries.country_code IS 'ISO 3166-1 alpha-2 country code';
COMMENT ON COLUMN fact_electricity_timeseries.bidding_zone IS 'ENTSO-E bidding zone code (e.g., SE1, SE2)';
COMMENT ON COLUMN fact_electricity_timeseries.production_type IS 'Production type code (e.g., B16 for Solar, B19 for Wind Onshore)';
COMMENT ON COLUMN fact_electricity_timeseries.load_mw IS 'Electricity load/demand in megawatts';
COMMENT ON COLUMN fact_electricity_timeseries.generation_mw IS 'Electricity generation in megawatts';
COMMENT ON COLUMN fact_electricity_timeseries.resolution IS 'Time resolution (PT60M for hourly, PT15M for 15-minute)';
COMMENT ON COLUMN fact_electricity_timeseries.quality_flag IS 'Data quality flag (actual, forecast, etc.)';
