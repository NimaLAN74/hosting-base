-- Phase 5: Additional Sources Integration
-- Migration 007: Add ENTSO-E and OSM columns to ML dataset tables
-- Date: 2026-01-13

-- Add ENTSO-E columns to forecasting dataset table
DO $$
BEGIN
    -- Check and add columns if they don't exist
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'ml_dataset_forecasting_v1' 
        AND column_name = 'avg_hourly_load_mw'
    ) THEN
        ALTER TABLE ml_dataset_forecasting_v1
        ADD COLUMN avg_hourly_load_mw NUMERIC(12,2),
        ADD COLUMN peak_load_mw NUMERIC(12,2),
        ADD COLUMN load_variability_mw NUMERIC(12,2),
        ADD COLUMN avg_renewable_generation_mw NUMERIC(12,2),
        ADD COLUMN avg_fossil_generation_mw NUMERIC(12,2),
        ADD COLUMN renewable_generation_pct NUMERIC(5,2),
        ADD COLUMN fossil_generation_pct NUMERIC(5,2);
        
        RAISE NOTICE 'Added ENTSO-E columns to ml_dataset_forecasting_v1';
    ELSE
        RAISE NOTICE 'ENTSO-E columns already exist in ml_dataset_forecasting_v1';
    END IF;
END $$;

-- Add OSM columns to clustering dataset table
DO $$
BEGIN
    -- Check and add columns if they don't exist
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'ml_dataset_clustering_v1' 
        AND column_name = 'power_plant_count'
    ) THEN
        ALTER TABLE ml_dataset_clustering_v1
        ADD COLUMN power_plant_count INTEGER DEFAULT 0,
        ADD COLUMN power_generator_count INTEGER DEFAULT 0,
        ADD COLUMN power_substation_count INTEGER DEFAULT 0,
        ADD COLUMN industrial_area_km2 DOUBLE PRECISION DEFAULT 0,
        ADD COLUMN residential_building_count INTEGER DEFAULT 0,
        ADD COLUMN commercial_building_count INTEGER DEFAULT 0,
        ADD COLUMN railway_station_count INTEGER DEFAULT 0,
        ADD COLUMN airport_count INTEGER DEFAULT 0,
        ADD COLUMN power_plant_density_per_km2 DOUBLE PRECISION DEFAULT 0,
        ADD COLUMN industrial_density_per_km2 DOUBLE PRECISION DEFAULT 0;
        
        RAISE NOTICE 'Added OSM columns to ml_dataset_clustering_v1';
    ELSE
        RAISE NOTICE 'OSM columns already exist in ml_dataset_clustering_v1';
    END IF;
END $$;
