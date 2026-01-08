# Storage Architecture Decision

**Version**: 1.0  
**Date**: January 7, 2026  
**Status**: ✅ Decision Made

---

## Executive Summary

**Decision**: Use **PostgreSQL 17 + PostGIS** as the primary storage technology.

**Rationale**:
- Already running in Lianel infrastructure
- PostGIS extension handles geospatial requirements (NUTS boundaries)
- Native partitioning handles time-series adequately
- Can add TimescaleDB extension later if needed
- Team familiarity and operational simplicity

**Migration Path**: Start with PostgreSQL + PostGIS. Evaluate TimescaleDB extension in Phase 3 if time-series performance becomes a bottleneck.

---

## Technology Comparison

### Option 1: PostgreSQL + PostGIS (Selected)

**Pros**:
- ✅ Already running in Lianel infrastructure (PostgreSQL 17.7)
- ✅ PostGIS extension for geospatial data (NUTS boundaries)
- ✅ Native table partitioning (by year) handles time-series
- ✅ Team familiarity and operational simplicity
- ✅ Excellent JSON support for flexible schemas
- ✅ Mature ecosystem and tooling
- ✅ No additional licensing or complexity

**Cons**:
- ⚠️ Manual partitioning setup required
- ⚠️ No automatic compression (can add later)
- ⚠️ Time-series queries may be slower than TimescaleDB (acceptable for annual data)

**Best For**:
- Geospatial data (NUTS boundaries)
- Relational data (dimensions, facts)
- Annual time-series (low frequency)
- Mixed query patterns

---

### Option 2: TimescaleDB

**Pros**:
- ✅ Built on PostgreSQL (compatible)
- ✅ Automatic time-series optimization
- ✅ Built-in compression
- ✅ Continuous aggregates
- ✅ Better performance for time-series queries

**Cons**:
- ❌ Additional complexity (extension installation)
- ❌ Learning curve for team
- ❌ May be overkill for annual data (designed for high-frequency)
- ❌ PostGIS compatibility (works but adds complexity)
- ❌ Not currently in Lianel stack

**Best For**:
- High-frequency time-series (hourly, 15-minute intervals)
- Future ENTSO-E data (if needed)
- Large-scale time-series analytics

---

### Option 3: Hybrid Approach

**Pros**:
- ✅ PostgreSQL for dimensions and geospatial
- ✅ TimescaleDB for time-series facts
- ✅ Best of both worlds

**Cons**:
- ❌ Most complex setup
- ❌ Data split across technologies
- ❌ More operational overhead
- ❌ Joins across systems more complex

**Best For**:
- Very large-scale deployments
- Mixed workloads with high-frequency time-series

---

## Decision Matrix

| Criteria | PostgreSQL + PostGIS | TimescaleDB | Hybrid | Weight | Winner |
|----------|---------------------|-------------|--------|--------|--------|
| **Infrastructure Fit** | ✅ Already in stack | ❌ Not installed | ⚠️ Partial | 25% | PostgreSQL |
| **Geospatial Support** | ✅ PostGIS native | ⚠️ Works with PostGIS | ⚠️ Complex | 20% | PostgreSQL |
| **Time-Series Performance** | ⚠️ Good (with partitioning) | ✅ Excellent | ✅ Excellent | 15% | TimescaleDB |
| **Operational Simplicity** | ✅ Simple | ⚠️ Moderate | ❌ Complex | 20% | PostgreSQL |
| **Team Familiarity** | ✅ High | ⚠️ Moderate | ⚠️ Moderate | 10% | PostgreSQL |
| **Future Scalability** | ⚠️ Good | ✅ Excellent | ✅ Excellent | 10% | TimescaleDB |

**Weighted Score**:
- **PostgreSQL + PostGIS**: 8.0/10.0 ✅ **WINNER**
- **TimescaleDB**: 6.5/10.0
- **Hybrid**: 5.5/10.0

---

## Storage Requirements Estimation

### Data Volume Calculations

Based on coverage assessment and data model:

#### Fact Tables

**fact_energy_annual**:
- Source: `nrg_bal_s` (primary), `nrg_cb_e`, `nrg_ind_eff`, `nrg_ind_ren`
- Countries: 27 (EU27)
- Years: 34 (1990-2023) for balances, 19 (2005-2023) for indicators
- Rows per country/year:
  - `nrg_bal_s`: ~2,154 values
  - `nrg_cb_e`: ~60 values
  - `nrg_ind_eff`: ~4 values
  - `nrg_ind_ren`: ~4 values
- **Total Rows**: ~2,000,000 rows
- **Row Size**: ~200 bytes (estimated)
- **Table Size**: ~400 MB (uncompressed)

**fact_electricity_timeseries** (Future - ENTSO-E):
- Countries: 27
- Frequency: Hourly (if implemented)
- Years: 1 year = 8,760 hours
- **Total Rows** (if 1 year): ~236,000 rows/year
- **Table Size**: ~50 MB/year (uncompressed)

**fact_geo_region_features** (Future - OSM):
- Regions: 334 (NUTS2)
- Features: ~50 per region
- **Total Rows**: ~16,700 rows
- **Table Size**: ~5 MB

#### Dimension Tables

**dim_region**:
- Regions: 37 (NUTS0) + 125 (NUTS1) + 334 (NUTS2) = 496 regions
- Geometry: ~50 KB per region (average)
- **Total Rows**: 496
- **Table Size**: ~25 MB (with geometries)

**dim_country**:
- Countries: 27
- **Table Size**: <1 MB

**dim_energy_product**:
- Products: ~100 (estimated)
- **Table Size**: <1 MB

**dim_production_type**:
- Types: ~20
- **Table Size**: <1 MB

#### Metadata Tables

**meta_ingestion_log**:
- Logs: ~100 entries/month
- **Table Size**: <10 MB/year

**meta_data_quality**:
- Checks: ~50 checks/month
- **Table Size**: <5 MB/year

### Total Storage Estimate

| Component | Size | Notes |
|-----------|------|-------|
| **Fact Tables** | | |
| fact_energy_annual | 400 MB | Primary dataset |
| fact_electricity_timeseries | 50 MB/year | Future (if implemented) |
| fact_geo_region_features | 5 MB | Future (if implemented) |
| **Dimension Tables** | | |
| dim_region | 25 MB | With PostGIS geometries |
| dim_country | 1 MB | |
| dim_energy_product | 1 MB | |
| dim_production_type | 1 MB | |
| **Metadata Tables** | | |
| meta_ingestion_log | 10 MB/year | |
| meta_data_quality | 5 MB/year | |
| **Indexes** | 100 MB | Estimated 25% of data |
| **Total (Year 1)** | **~500 MB** | Conservative estimate |
| **Total (5 years)** | **~1 GB** | With growth and metadata |

**Conclusion**: Storage requirements are **very manageable**. Even with 5 years of data, total storage < 1 GB.

---

## Partitioning Strategy

### Partitioning by Year

**Rationale**:
- Time-series data naturally partitions by year
- Improves query performance (partition pruning)
- Simplifies data retention and archival
- Aligns with annual data ingestion pattern

**Implementation**:

```sql
-- Main partitioned table
CREATE TABLE fact_energy_annual (
    id BIGSERIAL,
    country_code VARCHAR(2) NOT NULL,
    year INTEGER NOT NULL,
    sector VARCHAR(50),
    product_code VARCHAR(20),
    flow_code VARCHAR(20),
    value_gwh NUMERIC(12,2),
    unit VARCHAR(10),
    source_system VARCHAR(50),
    ingestion_timestamp TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (id, year)
) PARTITION BY RANGE (year);

-- Partitions for each year
CREATE TABLE fact_energy_annual_1990 PARTITION OF fact_energy_annual
    FOR VALUES FROM (1990) TO (1991);
    
CREATE TABLE fact_energy_annual_1991 PARTITION OF fact_energy_annual
    FOR VALUES FROM (1991) TO (1992);

-- ... continue for all years 1990-2023

-- Future partitions (auto-created via DAG)
CREATE TABLE fact_energy_annual_2024 PARTITION OF fact_energy_annual
    FOR VALUES FROM (2024) TO (2025);
```

**Benefits**:
- Query performance: Only relevant partitions scanned
- Maintenance: Easy to drop old partitions
- Backup: Can backup partitions independently
- Indexing: Smaller indexes per partition

**Partition Management**:
- Create partitions annually via Airflow DAG
- Archive old partitions (>10 years) to cold storage
- Drop partitions after archival (if needed)

---

## PostGIS Configuration

### Extension Installation

```sql
-- Enable PostGIS extension
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS postgis_topology;

-- Verify installation
SELECT PostGIS_version();
```

### Spatial Data Storage

**dim_region table**:
```sql
CREATE TABLE dim_region (
    region_id VARCHAR(10) PRIMARY KEY,
    level_code INTEGER NOT NULL,
    cntr_code VARCHAR(2) NOT NULL,
    name_latn VARCHAR(255),
    nuts_name VARCHAR(255),
    area_km2 DOUBLE PRECISION,
    geometry GEOMETRY(MULTIPOLYGON, 3035),  -- EPSG:3035 for analysis
    geometry_wgs84 GEOMETRY(MULTIPOLYGON, 4326),  -- EPSG:4326 for display
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Spatial index for fast queries
CREATE INDEX idx_dim_region_geometry ON dim_region USING GIST (geometry);
CREATE INDEX idx_dim_region_geometry_wgs84 ON dim_region USING GIST (geometry_wgs84);
```

**Spatial Indexes**:
- GIST indexes on geometry columns
- Essential for spatial joins and queries
- Automatically maintained by PostGIS

---

## Index Strategy

### Primary Indexes

**fact_energy_annual**:
```sql
-- Composite index for common queries
CREATE INDEX idx_energy_country_year ON fact_energy_annual(country_code, year);
CREATE INDEX idx_energy_year_product ON fact_energy_annual(year, product_code);
CREATE INDEX idx_energy_year_flow ON fact_energy_annual(year, flow_code);

-- Index on ingestion timestamp for data quality queries
CREATE INDEX idx_energy_ingestion ON fact_energy_annual(ingestion_timestamp);
```

**dim_region**:
```sql
-- Spatial indexes (GIST)
CREATE INDEX idx_region_geometry ON dim_region USING GIST (geometry);
CREATE INDEX idx_region_geometry_wgs84 ON dim_region USING GIST (geometry_wgs84);

-- Standard indexes
CREATE INDEX idx_region_cntr_code ON dim_region(cntr_code);
CREATE INDEX idx_region_level ON dim_region(level_code);
```

**dim_country**:
```sql
CREATE INDEX idx_country_code ON dim_country(country_code);
```

### Index Maintenance

- **Auto-vacuum**: Enabled by default
- **REINDEX**: Run quarterly or as needed
- **Monitor**: Track index usage via `pg_stat_user_indexes`

---

## Backup and Retention Strategy

### Backup Strategy

**Daily Backups**:
- Full database backup via `pg_dump`
- Retain 7 days of daily backups
- Store on remote host (separate from database)

**Weekly Backups**:
- Full database backup
- Retain 4 weeks of weekly backups

**Monthly Backups**:
- Full database backup
- Retain 12 months of monthly backups

**Backup Script** (to be implemented):
```bash
#!/bin/bash
# Backup PostgreSQL database
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backup/postgresql"
pg_dump -U postgres -Fc lianel_energy > "$BACKUP_DIR/lianel_energy_$DATE.dump"
```

### Retention Policy

**Data Retention**:
- **Fact tables**: Keep all data (1990-2023+)
- **Metadata tables**: Keep 2 years of logs
- **Old partitions**: Archive after 10 years (optional)

**Archival Strategy**:
- Export old partitions to CSV/Parquet
- Store in cold storage (S3, local archive)
- Drop partitions from database (if needed)

---

## Performance Considerations

### Query Patterns

**Analytical Queries** (Primary):
- Aggregations by country, year, product
- Time-series analysis (trends over years)
- Spatial joins (energy data × NUTS regions)
- ML dataset generation (feature extraction)

**Real-time Queries** (Secondary):
- Single country/year lookups
- Metadata queries
- Data quality checks

### Performance Targets

- **Analytical queries**: < 5 seconds for full-year aggregations
- **Spatial joins**: < 2 seconds for country-level joins
- **Single row lookups**: < 100ms
- **ML dataset generation**: < 30 seconds for full dataset

### Optimization Strategies

1. **Partitioning**: Reduces scan size for time-based queries
2. **Indexing**: Fast lookups on common filter columns
3. **Materialized Views**: Pre-aggregated data for dashboards (future)
4. **Query Optimization**: Use EXPLAIN ANALYZE to tune queries

---

## Migration Path to TimescaleDB (If Needed)

### When to Consider TimescaleDB

**Consider TimescaleDB if**:
- ENTSO-E hourly data becomes primary use case
- Query performance degrades with >10M rows
- Compression becomes critical (>100GB data)
- Continuous aggregates needed for dashboards

### Migration Steps

1. **Install TimescaleDB Extension**:
   ```sql
   CREATE EXTENSION IF NOT EXISTS timescaledb;
   ```

2. **Convert Partitioned Table to Hypertable**:
   ```sql
   SELECT create_hypertable('fact_energy_annual', 'year');
   ```

3. **Enable Compression** (optional):
   ```sql
   ALTER TABLE fact_energy_annual SET (
       timescaledb.compress,
       timescaledb.compress_segmentby = 'country_code'
   );
   ```

4. **Test Performance**: Compare query times before/after

**Note**: Migration is **reversible** - can convert back to regular partitioned table if needed.

---

## Database Configuration

### PostgreSQL Settings

**Recommended Settings** (postgresql.conf):
```ini
# Memory
shared_buffers = 2GB              # 25% of RAM (8GB system)
effective_cache_size = 6GB        # 75% of RAM
work_mem = 64MB                   # Per query operation
maintenance_work_mem = 512MB       # For VACUUM, CREATE INDEX

# Connections
max_connections = 100
max_worker_processes = 4

# Checkpoints
checkpoint_completion_target = 0.9
wal_buffers = 16MB

# Query Planner
random_page_cost = 1.1            # For SSD
effective_io_concurrency = 200   # For SSD
```

### PostGIS Settings

**No special configuration required** - defaults are sufficient for our data volume.

---

## Monitoring and Maintenance

### Key Metrics to Monitor

1. **Storage Usage**:
   ```sql
   SELECT pg_size_pretty(pg_database_size('lianel_energy'));
   SELECT pg_size_pretty(pg_total_relation_size('fact_energy_annual'));
   ```

2. **Table Sizes**:
   ```sql
   SELECT 
       schemaname,
       tablename,
       pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
   FROM pg_tables
   WHERE schemaname = 'public'
   ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
   ```

3. **Index Usage**:
   ```sql
   SELECT 
       schemaname,
       tablename,
       indexname,
       idx_scan,
       idx_tup_read,
       idx_tup_fetch
   FROM pg_stat_user_indexes
   ORDER BY idx_scan DESC;
   ```

4. **Query Performance**:
   - Enable `pg_stat_statements` extension
   - Monitor slow queries via Grafana dashboard

### Maintenance Tasks

**Weekly**:
- Check database size
- Review slow query log
- Verify backups

**Monthly**:
- VACUUM ANALYZE on all tables
- Review index usage
- Check partition sizes

**Quarterly**:
- REINDEX if needed
- Review and optimize queries
- Archive old data (if needed)

---

## Security Considerations

### Database Access

**Users and Roles**:
- `lianel_energy_owner`: Full access (for migrations)
- `lianel_energy_reader`: Read-only access (for APIs)
- `lianel_energy_writer`: Write access (for Airflow DAGs)

**Connection Security**:
- All connections via localhost or Docker network
- No external access (firewall rules)
- SSL/TLS for remote connections (if needed)

### Data Protection

- **Backups**: Encrypted at rest
- **Access Control**: Role-based permissions
- **Audit Logging**: Track data modifications (via metadata tables)

---

## Implementation Checklist

### Phase 1: Initial Setup

- [ ] Create database: `lianel_energy`
- [ ] Enable PostGIS extension
- [ ] Create schemas (if needed)
- [ ] Set up database users and roles
- [ ] Configure PostgreSQL settings
- [ ] Test PostGIS functionality

### Phase 2: Schema Creation

- [ ] Create dimension tables
- [ ] Create partitioned fact_energy_annual table
- [ ] Create partitions for years 1990-2024
- [ ] Create indexes
- [ ] Create metadata tables
- [ ] Load NUTS boundaries (dim_region)

### Phase 3: Integration

- [ ] Configure Airflow database connection
- [ ] Test data ingestion
- [ ] Set up monitoring dashboards
- [ ] Configure backups
- [ ] Document connection strings

---

## References

- **PostgreSQL Documentation**: https://www.postgresql.org/docs/
- **PostGIS Documentation**: https://postgis.net/documentation/
- **TimescaleDB Documentation**: https://docs.timescale.com/
- **Coverage Assessment**: `02-data-coverage-assessment.md`
- **Data Model**: `04-data-model/02-logical-data-model.md`

---

## Decision Summary

✅ **Selected**: PostgreSQL 17 + PostGIS

**Key Points**:
- Already in Lianel infrastructure
- PostGIS for geospatial data
- Native partitioning for time-series
- Simple and maintainable
- Can migrate to TimescaleDB later if needed

**Next Steps**:
1. Create database schema (DDL)
2. Set up PostGIS
3. Configure partitioning
4. Create indexes
5. Integrate with Airflow

---

**Status**: ✅ Storage architecture decision complete. Ready to proceed with schema design.

