# Phase 0 Quick Start Guide

**Version**: 1.0  
**Date**: 8 December 2025  
**Phase**: Documentation Enhancement & Requirements

## Overview

This guide provides actionable first steps to begin Phase 0 of the EU Energy & Geospatial Intelligence Platform implementation. These tasks will fill critical documentation gaps needed before development can begin.

---

## Immediate Actions (This Week)

### Action 1: Eurostat API Exploration

**Goal**: Understand exactly what data is available and how to access it.

**Steps**:

1. **Visit Eurostat API Documentation**
   - Main API: https://ec.europa.eu/eurostat/web/json-and-unicode-web-services/getting-started/rest-request
   - Data Browser: https://ec.europa.eu/eurostat/databrowser/

2. **Identify Priority Tables**
   
   Start with these key tables:
   
   | Table Code | Description | Priority |
   |------------|-------------|----------|
   | `nrg_bal_c` | Complete energy balances | Critical |
   | `nrg_bal_s` | Simplified energy balances | High |
   | `nrg_cb_e` | Energy supply and consumption | High |
   | `nrg_ind_eff` | Energy efficiency indicators | Medium |
   | `nrg_ind_ren` | Renewable energy indicators | Medium |

3. **Test API Calls**
   
   Example API request format:
   ```bash
   # Get complete energy balance for Sweden, 2020-2023
   curl "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/nrg_bal_c?format=JSON&geo=SE&time=2020&time=2021&time=2022&time=2023"
   ```

4. **Document Findings**
   
   Create a table with:
   - Table code and name
   - API endpoint
   - Available dimensions (geo, time, nrg_bal, product, etc.)
   - Time range (earliest → latest year)
   - Sample response structure
   - Number of records estimate
   - Data quality notes

**Deliverable**: Draft of `02-data-inventory/01-eurostat-inventory-detailed.md`

**Time Estimate**: 4-6 hours

---

### Action 2: NUTS Geospatial Data Download

**Goal**: Identify exact data sources and test loading geospatial data.

**Steps**:

1. **Navigate to GISCO**
   - URL: https://ec.europa.eu/eurostat/web/gisco/geodata/reference-data/administrative-units-statistical-units/nuts

2. **Download NUTS Boundaries**
   
   For each NUTS level (0, 1, 2):
   - Scale: 1:1 million (for analysis) or 1:10 million (for visualization)
   - Year: 2021 (latest stable version)
   - Format: GeoJSON or Shapefile
   - CRS: EPSG:4326 (will transform to EPSG:3035 later)

3. **Test Loading**
   
   Using Python/GeoPandas:
   ```python
   import geopandas as gpd
   
   # Load NUTS2
   gdf = gpd.read_file("NUTS_RG_01M_2021_4326_LEVL_2.geojson")
   
   # Check structure
   print(gdf.columns)
   print(gdf.head())
   
   # Transform to EPSG:3035
   gdf_proj = gdf.to_crs("EPSG:3035")
   
   # Calculate area
   gdf_proj['area_km2'] = gdf_proj.geometry.area / 1_000_000
   
   print(gdf_proj[['NUTS_ID', 'CNTR_CODE', 'NAME_LATN', 'area_km2']].head())
   ```

4. **Document Findings**
   - Exact download URLs
   - File sizes
   - Number of features per level
   - Field names and meanings
   - Coordinate system details
   - Processing notes

**Deliverable**: Enhanced `02-data-inventory/04-nuts-geospatial-inventory-detailed.md`

**Time Estimate**: 2-3 hours

---

### Action 3: Storage Technology Decision

**Goal**: Choose database technology for the platform.

**Steps**:

1. **Review Options**
   
   | Option | Pros | Cons |
   |--------|------|------|
   | **PostgreSQL** | - Already in Lianel stack<br>- Well-known<br>- PostGIS for spatial data | - May need partitioning for time-series |
   | **TimescaleDB** | - Built on PostgreSQL<br>- Optimized for time-series<br>- Compression | - Additional complexity<br>- Learning curve |
   | **Hybrid** | - PostgreSQL for dimensions<br>- TimescaleDB for facts | - Most complex setup |

2. **Consider Criteria**
   - Compatibility with existing Lianel infrastructure
   - Team expertise
   - Data volume (estimate: 10M-100M rows)
   - Query patterns (analytical, mostly read-heavy)
   - Time-series nature of ENTSO-E data (future)
   - Geospatial requirements (PostGIS needed)

3. **Make Recommendation**
   
   **Suggested**: Start with **PostgreSQL + PostGIS** because:
   - Already running in Lianel infrastructure
   - Team likely familiar
   - PostGIS handles geospatial needs
   - Can add TimescaleDB extension later if needed
   - Partitioning by year handles time-series adequately

4. **Document Decision**
   - Technology choice
   - Rationale
   - Configuration requirements
   - Migration path if needed later

**Deliverable**: `08-implementation/03-storage-architecture.md`

**Time Estimate**: 1-2 hours (if using PostgreSQL)

---

## Next Actions (Week 2-3)

### Action 4: Complete Database Schema

**Steps**:

1. **Expand Logical Model**
   
   For each table in D10, add:
   - Primary key definition
   - Foreign keys with ON DELETE/UPDATE rules
   - Data types:
     ```sql
     country_code VARCHAR(2) NOT NULL
     year INTEGER NOT NULL CHECK (year >= 1960 AND year <= 2100)
     value_gwh NUMERIC(12,2)
     timestamp_utc TIMESTAMP WITHOUT TIME ZONE NOT NULL
     ```
   - Indexes:
     ```sql
     CREATE INDEX idx_energy_country_year ON fact_energy_annual(country_code, year);
     ```
   - Partitioning (example):
     ```sql
     CREATE TABLE fact_energy_annual (
       ...
     ) PARTITION BY RANGE (year);
     
     CREATE TABLE fact_energy_annual_2020 PARTITION OF fact_energy_annual
       FOR VALUES FROM (2020) TO (2021);
     ```

2. **Add Metadata Tables**
   ```sql
   CREATE TABLE meta_ingestion_log (
     id SERIAL PRIMARY KEY,
     source_system VARCHAR(50) NOT NULL,
     table_name VARCHAR(100) NOT NULL,
     ingestion_timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
     records_inserted INTEGER,
     records_updated INTEGER,
     status VARCHAR(20) NOT NULL,
     error_message TEXT
   );
   
   CREATE TABLE meta_data_quality (
     id SERIAL PRIMARY KEY,
     table_name VARCHAR(100) NOT NULL,
     check_timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
     quality_check VARCHAR(100) NOT NULL,
     result VARCHAR(20) NOT NULL,
     details JSONB
   );
   ```

3. **Create DDL File**
   
   Full SQL script with:
   - Schema creation
   - Table definitions
   - Indexes
   - Foreign keys
   - Views (if any)

**Deliverable**: 
- `04-data-model/02-logical-data-model-complete.md` (enhanced)
- `04-data-model/05-schema-ddl.sql` (new)

**Time Estimate**: 6-8 hours

---

### Action 5: Design Airflow DAGs

**Steps**:

1. **DAG 1: Eurostat Raw Ingestion**
   ```python
   # Pseudo-code structure
   
   with DAG('eurostat_ingest', schedule_interval='@weekly') as dag:
       
       start = DummyOperator(task_id='start')
       
       # For each priority table
       fetch_nrg_bal_c = PythonOperator(
           task_id='fetch_nrg_bal_c',
           python_callable=fetch_eurostat_data,
           op_kwargs={'table': 'nrg_bal_c', 'start_year': 2020}
       )
       
       validate_nrg_bal_c = PythonOperator(
           task_id='validate_nrg_bal_c',
           python_callable=validate_data,
           op_kwargs={'table': 'nrg_bal_c'}
       )
       
       load_staging = PythonOperator(
           task_id='load_staging',
           python_callable=load_to_staging
       )
       
       end = DummyOperator(task_id='end')
       
       start >> fetch_nrg_bal_c >> validate_nrg_bal_c >> load_staging >> end
   ```

2. **Define Error Handling**
   ```python
   default_args = {
       'retries': 3,
       'retry_delay': timedelta(minutes=5),
       'retry_exponential_backoff': True,
       'on_failure_callback': send_alert_to_slack,
   }
   ```

3. **Design Idempotency**
   - Use `UPSERT` (INSERT ... ON CONFLICT UPDATE)
   - Check last ingestion timestamp
   - Skip if already processed

**Deliverable**: `08-implementation/04-airflow-dag-design.md`

**Time Estimate**: 8-10 hours

---

### Action 6: Define Data Quality Rules

**Example Rules**:

| Table | Field | Rule | Threshold |
|-------|-------|------|-----------|
| fact_energy_annual | value_gwh | NOT NULL | 100% |
| fact_energy_annual | value_gwh | >= 0 | 99% (allow some negative for imports/exports) |
| fact_energy_annual | year | BETWEEN 1960 AND CURRENT_YEAR | 100% |
| dim_country | country_code | IN (list of ISO codes) | 100% |
| dim_region | area_km2 | > 0 | 100% |

**Deliverable**: `08-implementation/06-data-quality-framework.md`

**Time Estimate**: 4-6 hours

---

## Phase 0 Checklist

Track your progress:

- [ ] Action 1: Eurostat API explored and documented
- [ ] Action 2: NUTS data downloaded and tested
- [ ] Action 3: Storage technology decided
- [ ] Action 4: Complete database schema with DDL
- [ ] Action 5: Airflow DAG designs complete
- [ ] Action 6: Data quality rules defined
- [ ] All critical gaps from gap analysis addressed
- [ ] Team review of Phase 0 deliverables
- [ ] Approval to proceed to Phase 1

**Target Completion**: 2-3 weeks from start

---

## Getting Help

### Resources

- **Eurostat API**: https://ec.europa.eu/eurostat/web/json-and-unicode-web-services
- **GISCO**: https://ec.europa.eu/eurostat/web/gisco
- **Airflow Docs**: https://airflow.apache.org/docs/
- **PostGIS**: https://postgis.net/documentation/
- **GeoPandas**: https://geopandas.org/

### Questions to Answer

As you work through these actions, document answers to:

1. What is the total data volume from Eurostat we'll ingest?
2. How often do Eurostat tables update? (weekly, monthly, annually?)
3. Do we need all EU27 countries or a subset?
4. What is the earliest year we care about? (1990? 2000? 2010?)
5. Can we use the existing PostgreSQL instance in Lianel, or do we need a separate database?
6. What monitoring metrics matter most for data pipelines?

---

## Success Criteria

Phase 0 is complete when:

1. ✅ You can make successful Eurostat API calls and understand the response format
2. ✅ You can load and process NUTS geospatial data
3. ✅ Database technology is selected with clear rationale
4. ✅ Complete DDL exists for all tables
5. ✅ DAG designs are reviewed and approved
6. ✅ Data quality framework is defined
7. ✅ Team is confident implementation can begin

---

## Next Phase Preview

Once Phase 0 is complete, Phase 1 will involve:
- Provisioning database infrastructure
- Setting up Airflow connections
- Creating monitoring dashboards
- Building development environment

**Estimated Start**: 3 weeks from now

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-12-08 | AI Assistant | Initial quick-start guide |

