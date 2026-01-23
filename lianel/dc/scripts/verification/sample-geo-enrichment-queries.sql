-- Sample SQL queries for geo-enrichment dataset analysis
-- These queries demonstrate how to analyze energy data with OSM features

-- 1. Regions with highest power plant density
SELECT 
    region_id,
    country_code,
    year,
    total_final_energy_gwh,
    osm_power_plants,
    osm_feature_count,
    CASE 
        WHEN total_final_energy_gwh > 0 THEN osm_power_plants::numeric / total_final_energy_gwh
        ELSE NULL
    END as power_plants_per_gwh
FROM ml_dataset_geo_enrichment_v1
WHERE osm_power_plants > 0
ORDER BY osm_power_plants DESC
LIMIT 20;

-- 2. Industrial areas comparison across regions
SELECT 
    country_code,
    COUNT(*) as region_count,
    AVG(osm_industrial_areas) as avg_industrial_areas,
    SUM(osm_industrial_areas) as total_industrial_areas,
    AVG(total_final_energy_gwh) as avg_energy_gwh
FROM ml_dataset_geo_enrichment_v1
WHERE osm_industrial_areas > 0
GROUP BY country_code
ORDER BY avg_industrial_areas DESC;

-- 3. Transport infrastructure analysis
SELECT 
    region_id,
    country_code,
    year,
    osm_transport,
    osm_buildings,
    total_final_energy_gwh,
    CASE 
        WHEN osm_buildings > 0 THEN osm_transport::numeric / osm_buildings
        ELSE NULL
    END as transport_per_building
FROM ml_dataset_geo_enrichment_v1
WHERE osm_transport > 0
ORDER BY osm_transport DESC
LIMIT 20;

-- 4. Energy vs OSM features correlation
SELECT 
    country_code,
    year,
    COUNT(*) as region_count,
    AVG(total_final_energy_gwh) as avg_energy,
    AVG(osm_feature_count) as avg_osm_features,
    AVG(osm_power_plants) as avg_power_plants,
    AVG(osm_industrial_areas) as avg_industrial,
    AVG(osm_buildings) as avg_buildings
FROM ml_dataset_geo_enrichment_v1
WHERE osm_feature_count > 0
GROUP BY country_code, year
ORDER BY country_code, year;

-- 5. Regions with high energy but low OSM features (potential data gaps)
SELECT 
    region_id,
    country_code,
    year,
    total_final_energy_gwh,
    osm_feature_count,
    CASE 
        WHEN total_final_energy_gwh > 1000 AND osm_feature_count = 0 THEN 'High Energy, No OSM'
        WHEN total_final_energy_gwh > 1000 AND osm_feature_count < 10 THEN 'High Energy, Low OSM'
        ELSE 'Normal'
    END as data_quality_flag
FROM ml_dataset_geo_enrichment_v1
WHERE total_final_energy_gwh > 1000
ORDER BY total_final_energy_gwh DESC
LIMIT 20;

-- 6. OSM feature completeness by country
SELECT 
    country_code,
    COUNT(*) as total_records,
    COUNT(CASE WHEN osm_feature_count > 0 THEN 1 END) as records_with_osm,
    ROUND(100.0 * COUNT(CASE WHEN osm_feature_count > 0 THEN 1 END) / COUNT(*), 2) as osm_coverage_pct,
    AVG(osm_feature_count) as avg_osm_features,
    MAX(osm_feature_count) as max_osm_features
FROM ml_dataset_geo_enrichment_v1
GROUP BY country_code
ORDER BY osm_coverage_pct DESC;

-- 7. Year-over-year OSM feature growth
SELECT 
    year,
    COUNT(*) as region_count,
    SUM(osm_feature_count) as total_osm_features,
    AVG(osm_feature_count) as avg_osm_features,
    SUM(osm_power_plants) as total_power_plants,
    SUM(osm_industrial_areas) as total_industrial,
    SUM(osm_buildings) as total_buildings
FROM ml_dataset_geo_enrichment_v1
WHERE osm_feature_count > 0
GROUP BY year
ORDER BY year;

-- 8. Top regions by combined energy and infrastructure
SELECT 
    region_id,
    country_code,
    year,
    total_final_energy_gwh,
    osm_feature_count,
    osm_power_plants + osm_industrial_areas + osm_buildings as total_infrastructure,
    (total_final_energy_gwh * 0.3 + (osm_power_plants + osm_industrial_areas + osm_buildings) * 0.7) as combined_score
FROM ml_dataset_geo_enrichment_v1
WHERE osm_feature_count > 0 AND total_final_energy_gwh > 0
ORDER BY combined_score DESC
LIMIT 20;
