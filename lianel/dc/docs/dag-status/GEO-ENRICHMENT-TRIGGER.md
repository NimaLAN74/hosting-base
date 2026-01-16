# Geo-Enrichment DAG Trigger
**Date**: January 15, 2026

---

## ✅ Action Taken

Triggered the `ml_dataset_geo_enrichment` DAG to regenerate the dataset with OSM features.

---

## Expected Results

After the DAG completes, the `ml_dataset_geo_enrichment_v1` table will include:

### New OSM Feature Columns
- `power_plant_count` - Number of power plants per region
- `power_generator_count` - Number of power generators
- `power_substation_count` - Number of power substations
- `industrial_area_km2` - Industrial area in square kilometers
- `railway_station_count` - Number of railway stations
- `airport_count` - Number of airports

### OSM Feature Densities
- `power_plant_density_per_km2` - Power plants per km²
- `power_generator_density_per_km2` - Generators per km²
- `power_substation_density_per_km2` - Substations per km²
- `industrial_area_pct` - Industrial area as percentage of total area

### Metadata
- `osm_feature_count` - Total number of OSM features for the region

---

## NUTS Level Support

The dataset will now include:
- **NUTS0** (country level) - as before
- **NUTS2** (regional level) - NEW - enables regional analysis

---

## Verification Steps

After DAG completion, verify:

1. **Check OSM features are included**:
   ```sql
   SELECT COUNT(*) as rows_with_osm, 
          COUNT(DISTINCT region_id) as regions
   FROM ml_dataset_geo_enrichment_v1 
   WHERE osm_feature_count > 0;
   ```

2. **Sample data with OSM features**:
   ```sql
   SELECT region_id, level_code, 
          power_plant_count, power_generator_count,
          industrial_area_km2, railway_station_count, airport_count
   FROM ml_dataset_geo_enrichment_v1
   WHERE level_code = 2 AND osm_feature_count > 0
   LIMIT 10;
   ```

3. **Check NUTS2 regions**:
   ```sql
   SELECT COUNT(*) as nuts2_regions
   FROM ml_dataset_geo_enrichment_v1
   WHERE level_code = 2;
   ```

---

## Status

⏳ **DAG RUNNING** - Waiting for completion

---

**Next**: Monitor DAG execution and verify data after completion
