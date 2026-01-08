# Architecture Constraints & Guidelines

**Version**: 1.0  
**Date**: January 7, 2026  
**Status**: Active Guidelines

---

## Core Architecture Principles

### 1. Backend Services: Rust APIs Only

**Rule**: All backend API services must be implemented in Rust.

**Rationale**:
- Performance and memory efficiency
- Type safety and reliability
- Consistency with existing Profile Service
- Small binary size and fast startup

**Existing Example**:
- `lianel/dc/profile-service/` - Rust + Axum API service
- Provides user profile management endpoints
- Integrates with Keycloak Admin API

**Future Services**:
- Data ingestion APIs (if needed)
- Data query APIs
- ML model serving APIs
- Any REST/GraphQL endpoints

**Technology Stack**:
- **Language**: Rust
- **Web Framework**: Axum (current standard)
- **HTTP Client**: Reqwest (for external APIs)
- **Serialization**: Serde
- **API Documentation**: utoipa + Swagger UI

---

### 2. Scheduled Tasks: Airflow DAGs Only

**Rule**: All scheduled/recurring tasks must be implemented as Airflow DAGs.

**Rationale**:
- Centralized orchestration
- Built-in scheduling, retries, and monitoring
- Integration with existing Airflow infrastructure
- Visibility in Airflow UI

**Existing Example**:
- `lianel/dc/dags/hello_world_dag.py` - Simple example DAG
- Airflow configured with PostgreSQL backend
- Monitoring integrated with Prometheus/Grafana

**Future DAGs**:
- Eurostat data ingestion
- NUTS boundary updates
- Data harmonization pipelines
- ML dataset generation
- Data quality checks
- Any recurring data processing

**DAG Structure**:
```python
from airflow import DAG
from airflow.operators.python import PythonOperator
from datetime import datetime, timedelta

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'retries': 3,
    'retry_delay': timedelta(minutes=5),
}

with DAG(
    'eurostat_ingestion',
    default_args=default_args,
    description='Ingest Eurostat energy data',
    schedule=timedelta(days=7),  # Weekly
    catchup=False,
    tags=['data-ingestion', 'eurostat'],
) as dag:
    # Tasks here
    pass
```

---

### 3. Python Scripts: Temporary/Testing Only

**Rule**: Python scripts should only be used for:
- Temporary testing and exploration
- One-off data analysis
- Development/debugging tools
- Scripts that will be converted to Rust APIs or Airflow DAGs

**Current Temporary Scripts** (in `lianel/dc/scripts/`):
- `test-eurostat-api.py` - API testing (temporary)
- `test-nuts-gisco.py` - GISCO testing (temporary)
- `test-eurostat-coverage-optimized.py` - Coverage testing (temporary)

**Migration Path**:
1. **Testing Phase**: Use Python scripts for rapid prototyping
2. **Production Phase**: Convert to:
   - **Rust API** if it's a backend service
   - **Airflow DAG** if it's a scheduled task

**Example Migration**:
```
# Temporary (Python script)
scripts/test-eurostat-api.py

# Production (Airflow DAG)
dags/eurostat_ingestion_dag.py
  └─> Calls Rust service or PythonOperator with production logic
```

---

## Implementation Guidelines

### When to Use Rust APIs

✅ **Use Rust APIs for**:
- REST/GraphQL endpoints
- Backend business logic
- Data processing services
- Integration with external APIs (as a service)
- Real-time data queries
- ML model inference endpoints

❌ **Don't use Rust APIs for**:
- Scheduled batch jobs (use Airflow DAGs)
- One-off scripts (use temporary Python)
- Data exploration (use temporary Python)

### When to Use Airflow DAGs

✅ **Use Airflow DAGs for**:
- Scheduled data ingestion
- ETL pipelines
- Data transformation workflows
- Data quality checks
- ML dataset generation
- Any recurring task

❌ **Don't use Airflow DAGs for**:
- Real-time API endpoints (use Rust APIs)
- User-facing services (use Rust APIs)
- One-off manual tasks (use temporary Python)

### When to Use Temporary Python Scripts

✅ **Use Python scripts for**:
- Initial data exploration
- API testing and validation
- Proof-of-concept development
- One-off data analysis
- Development/debugging tools

❌ **Don't use Python scripts for**:
- Production backend services (migrate to Rust)
- Production scheduled tasks (migrate to Airflow DAGs)
- Long-term solutions

---

## Migration Checklist

When converting a temporary Python script to production:

### To Rust API:
- [ ] Define API endpoints (REST/GraphQL)
- [ ] Implement in Rust using Axum
- [ ] Add OpenAPI/Swagger documentation
- [ ] Add health check endpoint
- [ ] Add error handling and logging
- [ ] Add Dockerfile
- [ ] Add to docker-compose configuration
- [ ] Add monitoring/metrics
- [ ] Update documentation

### To Airflow DAG:
- [ ] Define DAG structure and schedule
- [ ] Break into logical tasks
- [ ] Add error handling and retries
- [ ] Add data validation tasks
- [ ] Add monitoring/logging
- [ ] Add idempotency checks
- [ ] Test with sample data
- [ ] Document in DAG docstring
- [ ] Add to Airflow DAGs folder

---

## Current Architecture

### Existing Services

1. **Profile Service** (Rust API)
   - Location: `lianel/dc/profile-service/`
   - Framework: Axum
   - Purpose: User profile management
   - Endpoints: `/api/profile`, `/api/profile/change-password`, `/health`

2. **Airflow** (Orchestration)
   - Location: `lianel/dc/dags/`
   - Purpose: Data pipeline orchestration
   - Example: `hello_world_dag.py`

3. **Temporary Scripts** (Python)
   - Location: `lianel/dc/scripts/`
   - Purpose: Testing and exploration
   - Status: Will be migrated to Rust APIs or Airflow DAGs

---

## Future Services (Planned)

### Rust APIs (Backend Services)
- **Data Query API**: Query energy data from database
- **ML Inference API**: Serve ML model predictions
- **Data Validation API**: Validate data quality
- **Metadata API**: Query data source metadata

### Airflow DAGs (Scheduled Tasks)
- **Eurostat Ingestion DAG**: Weekly ingestion of Eurostat data
- **NUTS Update DAG**: Annual update of NUTS boundaries
- **Data Harmonization DAG**: Transform and harmonize data
- **ML Dataset Generation DAG**: Create ML-ready datasets
- **Data Quality Check DAG**: Run quality validation

---

## References

- **Profile Service**: `lianel/dc/profile-service/README.md`
- **Airflow Setup**: `lianel/dc/docker-compose.airflow.yaml`
- **Project Goals**: `lianel/docs/PROJECT-GOALS-AND-PLAN.md`

---

**Note**: This document should be referenced when designing new features or services to ensure architectural consistency.

