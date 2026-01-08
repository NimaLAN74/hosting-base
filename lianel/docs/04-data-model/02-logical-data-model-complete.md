# Complete Database Schema Design

**Version**: 2.0  
**Date**: January 7, 2026  
**Status**: ✅ Complete DDL Ready

---

## Executive Summary

This document provides the complete database schema specification for the EU Energy & Geospatial Intelligence Platform, including full DDL, data types, constraints, indexes, and partitioning strategy.

**Database**: PostgreSQL 17 + PostGIS  
**Schema File**: `05-schema-ddl.sql`  
**Partitioning**: By year (1990-2024+)

---

## Schema Overview

### Table Categories

1. **Dimension Tables** (5 tables):
   - `dim_country`: Country master data
   - `dim_region`: NUTS regions with geospatial boundaries
   - `dim_energy_product`: Energy product classification
   - `dim_energy_flow`: Energy balance flow classification
   - `dim_production_type`: Electricity production types (future)

2. **Fact Tables** (3 tables):
   - `fact_energy_annual`: Annual energy data (partitioned)
   - `fact_electricity_timeseries`: High-frequency electricity data (future)
   - `fact_geo_region_features`: Geospatial region features (future)

3. **Metadata Tables** (2 tables):
   - `meta_ingestion_log`: Data ingestion tracking
   - `meta_data_quality`: Data quality check results

**Total**: 10 tables + 1 view + 1 function

---

## Dimension Tables

### 1. dim_country

**Purpose**: Master data for countries (EU27 + others)

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `country_code` | VARCHAR(2) | PRIMARY KEY, NOT NULL | ISO 3166-1 alpha-2 code |
| `country_name` | VARCHAR(100) | NOT NULL | Country name |
| `nuts0_code` | VARCHAR(2) | NOT NULL | NUTS level 0 code |
| `iso_alpha3` | VARCHAR(3) | | ISO 3166-1 alpha-3 code |
| `eu_member` | BOOLEAN | DEFAULT TRUE | EU membership flag |
| `created_at` | TIMESTAMP | DEFAULT NOW() | Record creation timestamp |
| `updated_at` | TIMESTAMP | DEFAULT NOW() | Record update timestamp |

**Constraints**:
- `country_code` must be exactly 2 characters
- `nuts0_code` must be exactly 2 characters

**Indexes**:
- Primary key on `country_code`
- Index on `nuts0_code`
- Index on `country_name`

**Sample Data**:
```
country_code | country_name | nuts0_code | eu_member
-------------|--------------|------------|----------
SE           | Sverige      | SE         | true
DE           | Deutschland  | DE         | true
FR           | France       | FR         | true
```

---

### 2. dim_region

**Purpose**: NUTS regions with geospatial boundaries (PostGIS)

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `region_id` | VARCHAR(10) | PRIMARY KEY, NOT NULL | NUTS identifier |
| `level_code` | INTEGER | NOT NULL, CHECK (0-2) | NUTS level (0, 1, or 2) |
| `cntr_code` | VARCHAR(2) | NOT NULL, FK | Country code |
| `name_latn` | VARCHAR(255) | | Name in Latin script |
| `nuts_name` | VARCHAR(255) | | Official NUTS name |
| `mount_type` | VARCHAR(50) | | Mountain type classification |
| `urbn_type` | VARCHAR(50) | | Urban type classification |
| `coast_type` | VARCHAR(50) | | Coastal type classification |
| `area_km2` | DOUBLE PRECISION | | Area in square kilometers |
| `geometry` | GEOMETRY(MULTIPOLYGON, 3035) | | EPSG:3035 (analysis) |
| `geometry_wgs84` | GEOMETRY(MULTIPOLYGON, 4326) | | EPSG:4326 (display) |
| `created_at` | TIMESTAMP | DEFAULT NOW() | Record creation timestamp |
| `updated_at` | TIMESTAMP | DEFAULT NOW() | Record update timestamp |

**Constraints**:
- `level_code` must be 0, 1, or 2
- `region_id` length between 2 and 10 characters
- Foreign key to `dim_country(country_code)`

**Indexes**:
- Primary key on `region_id`
- Index on `cntr_code`
- Index on `level_code`
- **GIST index on `geometry`** (spatial)
- **GIST index on `geometry_wgs84`** (spatial)

**Sample Data**:
```
region_id | level_code | cntr_code | name_latn        | area_km2
----------|------------|-----------|------------------|----------
SE        | 0          | SE        | Sverige          | 450295
SE11      | 1          | SE        | Stockholm        | 6142
SE111     | 2          | SE        | Stockholm        | 6142
```

---

### 3. dim_energy_product

**Purpose**: Energy product classification (Eurostat SIEC codes)

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `product_code` | VARCHAR(20) | PRIMARY KEY, NOT NULL | Product code (SIEC) |
| `product_name` | VARCHAR(255) | NOT NULL | Product name |
| `category` | VARCHAR(50) | | Product category |
| `siec_code` | VARCHAR(10) | | Standard International Energy Classification |
| `renewable_flag` | BOOLEAN | DEFAULT FALSE | Renewable energy flag |
| `fossil_flag` | BOOLEAN | DEFAULT FALSE | Fossil fuel flag |
| `created_at` | TIMESTAMP | DEFAULT NOW() | Record creation timestamp |
| `updated_at` | TIMESTAMP | DEFAULT NOW() | Record update timestamp |

**Indexes**:
- Primary key on `product_code`
- Index on `category`
- Index on `renewable_flag`
- Index on `siec_code`

**Sample Data**:
```
product_code | product_name      | renewable_flag | fossil_flag
-------------|-------------------|----------------|-------------
C0000        | Total             | false          | false
C0350        | Natural gas       | false          | true
RA000        | Renewables total  | true           | false
RA100        | Hydro             | true           | false
RA200        | Wind              | true           | false
```

---

### 4. dim_energy_flow

**Purpose**: Energy balance flow classification (Eurostat nrg_bal codes)

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `flow_code` | VARCHAR(20) | PRIMARY KEY, NOT NULL | Flow code (e.g., PPRD, IMP) |
| `flow_name` | VARCHAR(255) | NOT NULL | Flow name |
| `flow_category` | VARCHAR(50) | | Category (supply, transformation, consumption) |
| `description` | TEXT | | Detailed description |
| `created_at` | TIMESTAMP | DEFAULT NOW() | Record creation timestamp |
| `updated_at` | TIMESTAMP | DEFAULT NOW() | Record update timestamp |

**Indexes**:
- Primary key on `flow_code`
- Index on `flow_category`

**Sample Data**:
```
flow_code | flow_name           | flow_category
----------|---------------------|---------------
PPRD      | Primary production  | supply
IMP       | Imports             | supply
EXP       | Exports             | supply
FC        | Final consumption   | consumption
```

---

### 5. dim_production_type

**Purpose**: Electricity production type classification (future - ENTSO-E)

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `code` | VARCHAR(20) | PRIMARY KEY, NOT NULL | Production type code |
| `name` | VARCHAR(255) | NOT NULL | Production type name |
| `renewable_flag` | BOOLEAN | DEFAULT FALSE | Renewable energy flag |
| `technology_type` | VARCHAR(50) | | Technology (solar, wind, etc.) |
| `created_at` | TIMESTAMP | DEFAULT NOW() | Record creation timestamp |
| `updated_at` | TIMESTAMP | DEFAULT NOW() | Record update timestamp |

**Indexes**:
- Primary key on `code`
- Index on `renewable_flag`
- Index on `technology_type`

---

## Fact Tables

### 1. fact_energy_annual

**Purpose**: Annual energy data from Eurostat (primary fact table)

**Partitioning**: By year (1990-2024+)

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | BIGSERIAL | PRIMARY KEY (with year) | Surrogate key |
| `country_code` | VARCHAR(2) | NOT NULL, FK | Country code |
| `year` | INTEGER | NOT NULL, CHECK (1960-2100) | Calendar year |
| `product_code` | VARCHAR(20) | FK | Energy product code |
| `flow_code` | VARCHAR(20) | FK | Energy flow code |
| `sector_code` | VARCHAR(50) | | Sector classification (optional) |
| `value_gwh` | NUMERIC(15,3) | NOT NULL, CHECK (>=0) | Energy value in GWh |
| `unit` | VARCHAR(10) | DEFAULT 'GWh' | Unit (always GWh after harmonization) |
| `source_system` | VARCHAR(50) | NOT NULL, DEFAULT 'eurostat' | Source system |
| `source_table` | VARCHAR(50) | | Source table (e.g., nrg_bal_s) |
| `harmonisation_version` | VARCHAR(20) | | Harmonization version |
| `ingestion_timestamp` | TIMESTAMP | DEFAULT NOW() | Ingestion timestamp |

**Constraints**:
- `year` between 1960 and 2100
- `value_gwh` must be non-negative
- Foreign keys to `dim_country`, `dim_energy_product`, `dim_energy_flow`

**Partitioning**:
- Partitioned by `year` using RANGE partitioning
- Partitions created for years 1990-2024
- Future partitions created via Airflow DAG

**Indexes** (created on each partition):
- Composite index on `(country_code, year)`
- Composite index on `(year, product_code)`
- Composite index on `(year, flow_code)`
- Index on `source_table`
- Index on `ingestion_timestamp`

**Estimated Size**:
- ~2,000,000 rows
- ~400 MB (uncompressed)

**Sample Data**:
```
id  | country_code | year | product_code | flow_code | value_gwh
----|--------------|------|--------------|-----------|----------
1   | SE           | 2022 | C0000        | FC        | 123456.789
2   | SE           | 2022 | RA000        | PPRD      | 45678.123
```

---

### 2. fact_electricity_timeseries

**Purpose**: High-frequency electricity data (future - ENTSO-E)

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | BIGSERIAL | PRIMARY KEY | Surrogate key |
| `timestamp_utc` | TIMESTAMP | NOT NULL, CHECK (>=2000-01-01) | UTC timestamp |
| `country_code` | VARCHAR(2) | NOT NULL, FK | Country code |
| `production_type_code` | VARCHAR(20) | FK | Production type code |
| `load_mw` | NUMERIC(12,2) | | Load in MW |
| `generation_mw` | NUMERIC(12,2) | | Generation in MW |
| `source_system` | VARCHAR(50) | DEFAULT 'entsoe' | Source system |
| `ingestion_timestamp` | TIMESTAMP | DEFAULT NOW() | Ingestion timestamp |

**Constraints**:
- `timestamp_utc` must be >= 2000-01-01
- Foreign keys to `dim_country`, `dim_production_type`

**Indexes**:
- Index on `timestamp_utc`
- Composite index on `(country_code, timestamp_utc)`
- Index on `production_type_code`

**Note**: This table is for future use when ENTSO-E data is integrated.

---

### 3. fact_geo_region_features

**Purpose**: Geospatial region features (future - OSM data)

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | BIGSERIAL | PRIMARY KEY | Surrogate key |
| `region_id` | VARCHAR(10) | NOT NULL, FK | NUTS region ID |
| `feature_name` | VARCHAR(100) | NOT NULL | Feature name |
| `feature_value` | NUMERIC(15,3) | | Feature value |
| `snapshot_year` | INTEGER | NOT NULL, CHECK (2000-2100) | Snapshot year |
| `source_system` | VARCHAR(50) | DEFAULT 'osm' | Source system |
| `ingestion_timestamp` | TIMESTAMP | DEFAULT NOW() | Ingestion timestamp |

**Constraints**:
- `snapshot_year` between 2000 and 2100
- Unique constraint on `(region_id, feature_name, snapshot_year)`
- Foreign key to `dim_region(region_id)`

**Indexes**:
- Composite index on `(region_id, snapshot_year)`
- Index on `feature_name`

**Note**: This table is for future use when OSM data is integrated.

---

## Metadata Tables

### 1. meta_ingestion_log

**Purpose**: Track all data ingestion operations

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | BIGSERIAL | PRIMARY KEY | Surrogate key |
| `source_system` | VARCHAR(50) | NOT NULL | Source system (eurostat, etc.) |
| `table_name` | VARCHAR(100) | NOT NULL | Target table name |
| `ingestion_timestamp` | TIMESTAMP | DEFAULT NOW() | Ingestion timestamp |
| `records_inserted` | INTEGER | DEFAULT 0 | Number of records inserted |
| `records_updated` | INTEGER | DEFAULT 0 | Number of records updated |
| `records_failed` | INTEGER | DEFAULT 0 | Number of records failed |
| `status` | VARCHAR(20) | NOT NULL, CHECK | Status (success, partial, failed) |
| `error_message` | TEXT | | Error message (if failed) |
| `execution_time_seconds` | NUMERIC(10,2) | | Execution time |
| `metadata` | JSONB | | Additional metadata (JSON) |

**Constraints**:
- `status` must be 'success', 'partial', or 'failed'

**Indexes**:
- Index on `ingestion_timestamp`
- Index on `source_system`
- Index on `status`
- Index on `table_name`

---

### 2. meta_data_quality

**Purpose**: Store data quality check results

**Columns**:

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | BIGSERIAL | PRIMARY KEY | Surrogate key |
| `table_name` | VARCHAR(100) | NOT NULL | Table name |
| `check_timestamp` | TIMESTAMP | DEFAULT NOW() | Check timestamp |
| `quality_check` | VARCHAR(100) | NOT NULL | Check name (e.g., completeness) |
| `result` | VARCHAR(20) | NOT NULL, CHECK | Result (pass, warn, fail) |
| `details` | JSONB | | Detailed results (JSON) |

**Constraints**:
- `result` must be 'pass', 'warn', or 'fail'

**Indexes**:
- Index on `check_timestamp`
- Index on `table_name`
- Index on `result`

---

## Views

### v_energy_annual_enriched

**Purpose**: Convenience view with dimension names joined

**Columns**: All columns from `fact_energy_annual` plus:
- `country_name` from `dim_country`
- `product_name` from `dim_energy_product`
- `flow_name` from `dim_energy_flow`

**Usage**:
```sql
SELECT * FROM v_energy_annual_enriched 
WHERE country_code = 'SE' AND year = 2022;
```

---

## Functions

### calculate_completeness()

**Purpose**: Calculate completeness percentage for a column

**Parameters**:
- `p_table_name`: Table name
- `p_column_name`: Column name
- `p_year`: Optional year filter

**Returns**: Table with `total_rows`, `non_null_rows`, `completeness_pct`

**Usage**:
```sql
SELECT * FROM calculate_completeness('fact_energy_annual', 'product_code', 2022);
```

---

## Relationships (Entity-Relationship Diagram)

```
dim_country (1) ──< (many) dim_region
dim_country (1) ──< (many) fact_energy_annual
dim_region (1) ──< (many) fact_geo_region_features
dim_energy_product (1) ──< (many) fact_energy_annual
dim_energy_flow (1) ──< (many) fact_energy_annual
dim_production_type (1) ──< (many) fact_electricity_timeseries
```

---

## Data Types Summary

| Category | Data Type | Usage |
|----------|-----------|-------|
| **Identifiers** | VARCHAR(2-20) | Country codes, region IDs, product codes |
| **Names** | VARCHAR(100-255) | Country names, product names, flow names |
| **Numeric** | NUMERIC(12-15,2-3) | Energy values (GWh), areas (km²) |
| **Integers** | INTEGER | Years, level codes, counts |
| **Booleans** | BOOLEAN | Flags (renewable, fossil, EU member) |
| **Timestamps** | TIMESTAMP | Ingestion timestamps, check timestamps |
| **Geospatial** | GEOMETRY(MULTIPOLYGON, SRID) | NUTS boundaries |
| **JSON** | JSONB | Metadata, quality check details |
| **Text** | TEXT | Descriptions, error messages |

---

## Constraints Summary

### Check Constraints

- `dim_country`: `country_code` and `nuts0_code` must be 2 characters
- `dim_region`: `level_code` must be 0, 1, or 2
- `fact_energy_annual`: `year` between 1960-2100, `value_gwh` >= 0
- `fact_electricity_timeseries`: `timestamp_utc` >= 2000-01-01
- `fact_geo_region_features`: `snapshot_year` between 2000-2100
- `meta_ingestion_log`: `status` in ('success', 'partial', 'failed')
- `meta_data_quality`: `result` in ('pass', 'warn', 'fail')

### Foreign Key Constraints

- `dim_region.cntr_code` → `dim_country.country_code`
- `fact_energy_annual.country_code` → `dim_country.country_code`
- `fact_energy_annual.product_code` → `dim_energy_product.product_code`
- `fact_energy_annual.flow_code` → `dim_energy_flow.flow_code`
- `fact_electricity_timeseries.country_code` → `dim_country.country_code`
- `fact_electricity_timeseries.production_type_code` → `dim_production_type.code`
- `fact_geo_region_features.region_id` → `dim_region.region_id`

### Unique Constraints

- `fact_geo_region_features`: `(region_id, feature_name, snapshot_year)`

---

## Index Strategy

### Primary Indexes

All tables have primary keys (single column or composite).

### Secondary Indexes

**Dimension Tables**:
- Foreign key columns (for joins)
- Frequently queried columns (names, flags)

**Fact Tables**:
- Composite indexes on common query patterns:
  - `(country_code, year)` - Country-specific queries
  - `(year, product_code)` - Product analysis
  - `(year, flow_code)` - Flow analysis
- Single column indexes on filters (source_table, ingestion_timestamp)

**Spatial Indexes** (PostGIS):
- GIST indexes on geometry columns in `dim_region`

---

## Partitioning Strategy

### fact_energy_annual

**Partitioning Method**: RANGE partitioning by `year`

**Partitions Created**:
- Years 1990-2024 (35 partitions)
- Future partitions created via Airflow DAG

**Benefits**:
- Query performance (partition pruning)
- Easy data archival
- Smaller indexes per partition

**Partition Management**:
- Create new partitions annually (via Airflow DAG)
- Archive old partitions (>10 years) if needed
- Drop partitions after archival (optional)

---

## Storage Estimates

| Table | Rows (Year 1) | Size (Year 1) | Growth |
|-------|---------------|---------------|--------|
| `fact_energy_annual` | ~2,000,000 | ~400 MB | ~10 MB/year |
| `dim_region` | 496 | ~25 MB | Minimal |
| `dim_country` | 27 | <1 MB | Minimal |
| `dim_energy_product` | ~100 | <1 MB | Minimal |
| `dim_energy_flow` | ~95 | <1 MB | Minimal |
| `meta_ingestion_log` | ~1,200/year | ~10 MB/year | Linear |
| `meta_data_quality` | ~600/year | ~5 MB/year | Linear |
| **Indexes** | - | ~100 MB | ~10 MB/year |
| **Total (Year 1)** | - | **~500 MB** | - |
| **Total (5 years)** | - | **~1 GB** | - |

---

## Implementation Checklist

### Phase 1: Database Setup
- [ ] Create database: `lianel_energy`
- [ ] Enable PostGIS extension
- [ ] Enable pg_stat_statements extension
- [ ] Run DDL script (`05-schema-ddl.sql`)

### Phase 2: Initial Data Load
- [ ] Load `dim_country` (27 EU countries)
- [ ] Load `dim_region` (NUTS boundaries from GISCO)
- [ ] Load `dim_energy_product` (Eurostat SIEC codes)
- [ ] Load `dim_energy_flow` (Eurostat nrg_bal codes)

### Phase 3: Integration
- [ ] Configure Airflow database connection
- [ ] Test data ingestion
- [ ] Set up monitoring
- [ ] Configure backups

---

## References

- **DDL Script**: `04-data-model/05-schema-ddl.sql`
- **Storage Architecture**: `08-implementation/03-storage-architecture.md`
- **Data Coverage**: `08-implementation/02-data-coverage-assessment.md`
- **PostgreSQL Documentation**: https://www.postgresql.org/docs/17/
- **PostGIS Documentation**: https://postgis.net/documentation/

---

**Status**: ✅ Complete database schema design ready for implementation.

