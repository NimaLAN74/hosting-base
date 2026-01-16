# Phase 5: Additional Sources Integration

**Date**: January 13, 2026  
**Status**: 🚀 **IN PROGRESS**  
**Duration**: 6-8 weeks

---

## Objectives

1. Integrate ENTSO-E for high-frequency electricity data
2. Add OSM for geospatial enrichment
3. Fill identified data gaps
4. Enhance ML datasets with new features

---

## Phase 5.1: ENTSO-E Implementation

### 5.1.1 Set Up ENTSO-E API Authentication

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Research ENTSO-E Transparency Platform API
- [ ] Register for API access (if required)
- [ ] Obtain API credentials/tokens
- [ ] Test API connectivity
- [ ] Document API endpoints and rate limits
- [ ] Store credentials securely (environment variables)

**Deliverables**:
- API access configured
- Authentication working
- API documentation

---

### 5.1.2 Design Time-Series Storage Strategy

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Design `fact_electricity_timeseries` table schema
- [ ] Define partitioning strategy (by year/month)
- [ ] Plan indexes for time-series queries
- [ ] Design data retention policy
- [ ] Consider TimescaleDB extension (optional)

**Schema Design**:
```sql
CREATE TABLE fact_electricity_timeseries (
    id BIGSERIAL PRIMARY KEY,
    timestamp_utc TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    country_code VARCHAR(2) NOT NULL,
    bidding_zone VARCHAR(10),
    production_type VARCHAR(50),
    load_mw NUMERIC(12,2),
    generation_mw NUMERIC(12,2),
    resolution VARCHAR(10) NOT NULL, -- PT60M, PT15M
    quality_flag VARCHAR(20),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_country FOREIGN KEY (country_code) REFERENCES dim_country(country_code)
);

-- Partitioning by year
CREATE INDEX idx_electricity_timestamp ON fact_electricity_timeseries(timestamp_utc);
CREATE INDEX idx_electricity_country_time ON fact_electricity_timeseries(country_code, timestamp_utc);
CREATE INDEX idx_electricity_production_type ON fact_electricity_timeseries(production_type);
```

**Deliverables**:
- Database schema created
- Indexes optimized
- Partitioning strategy implemented

---

### 5.1.3 Implement Ingestion DAG

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Create `entsoe_ingestion_dag.py`
- [ ] Implement API client for ENTSO-E
- [ ] Handle hourly data ingestion (PT60M)
- [ ] Handle 15-minute data ingestion (PT15M) - optional
- [ ] Implement incremental loading (checkpoint/resume)
- [ ] Add error handling and retry logic
- [ ] Handle missing periods gracefully
- [ ] Add data quality checks

**DAG Structure**:
```python
# entsoe_ingestion_dag.py
- Task 1: Fetch ENTSO-E data for date range
- Task 2: Transform and normalize data
- Task 3: Handle DST and timezone conversions
- Task 4: Load into fact_electricity_timeseries
- Task 5: Validate data quality
- Task 6: Update metadata
```

**Deliverables**:
- Operational ENTSO-E ingestion DAG
- Data loading into database
- Quality validation

---

### 5.1.4 Handle DST and Timezone Conversions

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Map country codes to timezones
- [ ] Convert local time to UTC
- [ ] Handle DST transitions (duplicate/missing hours)
- [ ] Mark DST-affected records
- [ ] Validate timezone conversions

**Deliverables**:
- Timezone conversion logic
- DST handling implemented
- All timestamps in UTC

---

## Phase 5.2: OSM Processing

### 5.2.1 Set Up Overpass API Access

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Research Overpass API endpoints
- [ ] Test Overpass API queries
- [ ] Implement rate limiting
- [ ] Design query patterns for feature extraction
- [ ] Handle API timeouts and errors

**OSM Features to Extract**:
- Energy infrastructure: `power=plant`, `power=generator`, `power=substation`
- Industrial: `landuse=industrial`, `man_made=works`
- Residential: `building=house`, `building=apartments`
- Commercial: `amenity=*`, `office=*`, `shop=*`
- Transport: `railway=station`, `aeroway=aerodrome`
- Land use: `landuse=*`, `natural=*`

**Deliverables**:
- Overpass API access configured
- Query patterns defined
- Rate limiting implemented

---

### 5.2.2 Define Feature Extraction Queries

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Create Overpass queries for each feature type
- [ ] Test queries on sample regions
- [ ] Optimize query performance
- [ ] Handle large result sets
- [ ] Document query patterns

**Example Query**:
```overpass
[out:json][timeout:25];
(
  way["power"="plant"]({{bbox}});
  way["power"="generator"]({{bbox}});
);
out geom;
```

**Deliverables**:
- Feature extraction queries
- Query documentation
- Test results

---

### 5.2.3 Implement Spatial Aggregation to NUTS2

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Design `fact_geo_region_features` table schema
- [ ] Implement spatial join (OSM features → NUTS2 regions)
- [ ] Calculate counts and densities per region
- [ ] Aggregate by feature type
- [ ] Handle features spanning multiple regions
- [ ] Add snapshot_year for temporal tracking

**Schema Design**:
```sql
CREATE TABLE fact_geo_region_features (
    id BIGSERIAL PRIMARY KEY,
    region_id VARCHAR(10) NOT NULL,
    feature_name VARCHAR(100) NOT NULL, -- e.g., 'power_plant_count', 'building_residential_area_km2'
    feature_value NUMERIC(12,2) NOT NULL,
    snapshot_year INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_region FOREIGN KEY (region_id) REFERENCES dim_region(region_id),
    UNIQUE(region_id, feature_name, snapshot_year)
);

CREATE INDEX idx_geo_region ON fact_geo_region_features(region_id);
CREATE INDEX idx_geo_feature ON fact_geo_region_features(feature_name);
CREATE INDEX idx_geo_year ON fact_geo_region_features(snapshot_year);
```

**Deliverables**:
- Database schema created
- Spatial aggregation implemented
- Features loaded per NUTS2 region

---

### 5.2.4 Create OSM Processing DAG

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Create `osm_feature_extraction_dag.py`
- [ ] Implement Overpass API client
- [ ] Process regions in batches
- [ ] Perform spatial joins
- [ ] Calculate aggregated features
- [ ] Load into `fact_geo_region_features`
- [ ] Add data quality checks

**DAG Structure**:
```python
# osm_feature_extraction_dag.py
- Task 1: Get list of NUTS2 regions
- Task 2: For each region, extract OSM features
- Task 3: Perform spatial aggregation
- Task 4: Calculate counts and densities
- Task 5: Load into fact_geo_region_features
- Task 6: Validate data quality
```

**Deliverables**:
- Operational OSM processing DAG
- Features loaded into database
- Quality validation

---

## Phase 5.3: Enhanced ML Datasets

### 5.3.1 Update Forecasting Dataset with ENTSO-E Features

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Analyze ENTSO-E data availability
- [ ] Design feature engineering for time-series
- [ ] Aggregate hourly data to annual/daily features
- [ ] Add features to `ml_dataset_forecasting_v1`:
  - Average hourly load (MW)
  - Peak load (MW)
  - Load variability (std dev)
  - Generation by production type
  - Renewable generation percentage
- [ ] Update forecasting DAG
- [ ] Validate enhanced dataset

**New Features**:
- `avg_hourly_load_mw`: Average electricity load
- `peak_load_mw`: Peak electricity load
- `load_variability`: Standard deviation of load
- `renewable_generation_pct`: Percentage renewable generation
- `fossil_generation_pct`: Percentage fossil generation

**Deliverables**:
- Enhanced forecasting dataset
- Updated DAG
- Feature documentation

---

### 5.3.2 Enrich Clustering Dataset with OSM Features

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Join OSM features with clustering dataset
- [ ] Add spatial features:
  - Power plant count
  - Industrial area (km²)
  - Residential building count
  - Commercial building count
  - Transport infrastructure count
- [ ] Calculate densities (per km²)
- [ ] Update clustering DAG
- [ ] Validate enhanced dataset

**New Features**:
- `power_plant_count`: Number of power plants
- `industrial_area_km2`: Industrial land use area
- `residential_building_count`: Residential buildings
- `commercial_building_count`: Commercial buildings
- `transport_infrastructure_count`: Transport facilities
- `power_plant_density`: Power plants per km²
- `industrial_density`: Industrial area per km²

**Deliverables**:
- Enhanced clustering dataset
- Updated DAG
- Feature documentation

---

### 5.3.3 Regenerate Geo-Enrichment Dataset

**Status**: ⏳ Not Started

**Tasks**:
- [ ] Combine energy, electricity, and OSM features
- [ ] Update `ml_dataset_geo_enrichment_v1`
- [ ] Add new derived features
- [ ] Update geo-enrichment DAG
- [ ] Validate complete dataset

**Deliverables**:
- Enhanced geo-enrichment dataset
- Updated DAG
- Complete feature set

---

## Implementation Order

### Week 1-2: ENTSO-E Foundation
1. API authentication setup
2. Database schema design and creation
3. Basic ingestion DAG
4. Timezone handling

### Week 3-4: ENTSO-E Completion
5. Full ingestion pipeline
6. Data quality validation
7. Performance optimization

### Week 5-6: OSM Foundation
8. Overpass API setup
9. Feature extraction queries
10. Database schema for geo features
11. Spatial aggregation logic

### Week 7-8: OSM Completion & ML Enhancement
12. OSM processing DAG
13. Enhanced ML datasets
14. Testing and validation
15. Documentation

---

## Success Criteria

- [ ] ENTSO-E data ingested for all EU countries
- [ ] OSM features extracted for all NUTS2 regions
- [ ] Enhanced ML datasets with new features
- [ ] All DAGs operational and tested
- [ ] Data quality checks passing
- [ ] Documentation complete

---

## Dependencies

- Phase 4 complete (ML datasets exist)
- Database access and permissions
- ENTSO-E API access
- Overpass API access
- GeoPandas for spatial operations
- Sufficient storage capacity

---

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| ENTSO-E API rate limits | High | Implement rate limiting and batching |
| OSM query timeouts | Medium | Process in smaller batches, optimize queries |
| Large data volumes | High | Implement partitioning and archiving |
| Timezone conversion errors | Medium | Thorough testing, validation |
| Spatial join performance | Medium | Optimize indexes, batch processing |

---

**Status**: 🚀 **Starting Phase 5.1.1 - ENTSO-E API Setup**
