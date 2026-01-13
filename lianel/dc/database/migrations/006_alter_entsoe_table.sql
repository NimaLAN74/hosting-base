-- Phase 5: Additional Sources Integration
-- Migration 006: Alter existing fact_electricity_timeseries table to add missing columns
-- Date: 2026-01-13

-- Add missing columns to existing fact_electricity_timeseries table
ALTER TABLE fact_electricity_timeseries 
    ADD COLUMN IF NOT EXISTS bidding_zone VARCHAR(10),
    ADD COLUMN IF NOT EXISTS resolution VARCHAR(10) DEFAULT 'PT60M',
    ADD COLUMN IF NOT EXISTS quality_flag VARCHAR(20),
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

-- Rename production_type_code to production_type for consistency
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'fact_electricity_timeseries' 
        AND column_name = 'production_type_code'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'fact_electricity_timeseries' 
        AND column_name = 'production_type'
    ) THEN
        ALTER TABLE fact_electricity_timeseries 
        RENAME COLUMN production_type_code TO production_type;
    END IF;
END $$;

-- Add resolution constraint if not exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'chk_resolution'
        AND table_name = 'fact_electricity_timeseries'
    ) THEN
        ALTER TABLE fact_electricity_timeseries 
        ADD CONSTRAINT chk_resolution CHECK (resolution IN ('PT15M', 'PT60M', 'PT30M', 'PT1H'));
    END IF;
END $$;

-- Update resolution for existing rows
UPDATE fact_electricity_timeseries 
SET resolution = 'PT60M' 
WHERE resolution IS NULL;

-- Make resolution NOT NULL after setting defaults
ALTER TABLE fact_electricity_timeseries 
    ALTER COLUMN resolution SET NOT NULL;

-- Add missing indexes
CREATE INDEX IF NOT EXISTS idx_electricity_bidding_zone ON fact_electricity_timeseries(bidding_zone) WHERE bidding_zone IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_electricity_country_type_time ON fact_electricity_timeseries(country_code, production_type, timestamp_utc) WHERE production_type IS NOT NULL;

-- Update foreign key constraint name if needed (production_type instead of production_type_code)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'fk_electricity_production_type'
        AND table_name = 'fact_electricity_timeseries'
    ) THEN
        -- Drop old constraint if it references production_type_code
        ALTER TABLE fact_electricity_timeseries 
        DROP CONSTRAINT IF EXISTS fk_electricity_production_type;
        
        -- Add new constraint referencing production_type
        ALTER TABLE fact_electricity_timeseries 
        ADD CONSTRAINT fk_electricity_production_type 
        FOREIGN KEY (production_type) REFERENCES dim_production_type(code);
    END IF;
END $$;
