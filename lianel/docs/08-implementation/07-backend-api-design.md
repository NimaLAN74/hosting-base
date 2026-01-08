# Backend API Design - Energy Data Service

**Date**: January 8, 2026  
**Status**: 🚧 In Design  
**Service**: `energy-service` (Rust)

---

## Overview

REST API service for querying energy data from the `lianel_energy` database. Follows the same architecture pattern as `profile-service`.

---

## Technology Stack

- **Language**: Rust
- **Web Framework**: Axum
- **Database**: PostgreSQL (via `sqlx` or `tokio-postgres`)
- **Authentication**: Keycloak (reuse existing validator pattern)
- **API Documentation**: OpenAPI/Swagger UI (utoipa)
- **Serialization**: Serde

---

## API Endpoints

### 1. Energy Data Queries

#### `GET /api/energy/annual`
Get annual energy data with filtering.

**Query Parameters**:
- `country_code` (optional): ISO country code (e.g., "DE", "FR")
- `year` (optional): Year (e.g., 2020, 2021)
- `product_code` (optional): Energy product code
- `flow_code` (optional): Energy flow code
- `source_table` (optional): Source table (e.g., "nrg_bal_s")
- `limit` (optional): Max records (default: 1000)
- `offset` (optional): Pagination offset (default: 0)

**Response**:
```json
{
  "data": [
    {
      "id": 1,
      "country_code": "DE",
      "country_name": "Germany",
      "year": 2021,
      "product_code": "C0000",
      "product_name": "Total",
      "flow_code": "FC_IND_E",
      "flow_name": "Final consumption - industry",
      "value_gwh": 123456.78,
      "unit": "GWh",
      "source_table": "nrg_bal_s",
      "ingestion_timestamp": "2026-01-08T08:00:00Z"
    }
  ],
  "total": 365,
  "limit": 1000,
  "offset": 0
}
```

#### `GET /api/energy/annual/summary`
Get aggregated summary statistics.

**Query Parameters**:
- `country_code` (optional): Filter by country
- `year` (optional): Filter by year
- `group_by` (optional): "country", "year", "product", "flow" (default: "country")

**Response**:
```json
{
  "summary": [
    {
      "group": "DE",
      "total_gwh": 1234567.89,
      "record_count": 45
    }
  ],
  "total_records": 365
}
```

#### `GET /api/energy/annual/by-country/{country_code}`
Get all energy data for a specific country.

**Path Parameters**:
- `country_code`: ISO country code

**Query Parameters**: Same as `/api/energy/annual`

#### `GET /api/energy/annual/by-year/{year}`
Get all energy data for a specific year.

**Path Parameters**:
- `year`: Year (e.g., 2021)

**Query Parameters**: Same as `/api/energy/annual`

### 2. Regional Data (Geospatial)

#### `GET /api/energy/regions`
Get NUTS regions with optional energy data aggregation.

**Query Parameters**:
- `nuts_level` (optional): 0, 1, or 2 (default: 2)
- `country_code` (optional): Filter by country
- `include_energy` (optional): Include aggregated energy data (default: false)

**Response**:
```json
{
  "regions": [
    {
      "nuts_code": "DE21",
      "nuts_name": "Brandenburg",
      "nuts_level": 2,
      "country_code": "DE",
      "area_km2": 29478.61,
      "geometry": "...",
      "energy_total_gwh": 12345.67  // if include_energy=true
    }
  ]
}
```

### 3. Metadata & Reference Data

#### `GET /api/energy/countries`
Get list of available countries.

**Response**:
```json
{
  "countries": [
    {
      "country_code": "DE",
      "country_name": "Germany",
      "eu_member": true,
      "record_count": 45
    }
  ]
}
```

#### `GET /api/energy/products`
Get list of energy products.

**Response**:
```json
{
  "products": [
    {
      "product_code": "C0000",
      "product_name": "Total",
      "record_count": 365
    }
  ]
}
```

#### `GET /api/energy/flows`
Get list of energy flows.

**Response**:
```json
{
  "flows": [
    {
      "flow_code": "FC_IND_E",
      "flow_name": "Final consumption - industry",
      "record_count": 200
    }
  ]
}
```

### 4. Data Quality

#### `GET /api/energy/quality`
Get latest data quality report.

**Response**:
```json
{
  "latest_report": {
    "check_timestamp": "2026-01-08T02:00:00Z",
    "overall_status": "warn",
    "quality_score": 90.0,
    "dimensions": {
      "completeness": "pass",
      "validity": "pass",
      "consistency": "pass",
      "accuracy": "fail"
    },
    "issues": [
      "⚠️  Found 12 outliers (3.29%) - exceeds threshold"
    ]
  }
}
```

### 5. Health & Info

#### `GET /health`
Health check endpoint.

**Response**:
```json
{
  "status": "ok",
  "service": "lianel-energy-service",
  "database": "connected",
  "version": "1.0.0"
}
```

#### `GET /api/info`
Service information and statistics.

**Response**:
```json
{
  "service": "lianel-energy-service",
  "version": "1.0.0",
  "database": {
    "connected": true,
    "total_records": 365,
    "countries": 27,
    "years": 4,
    "tables": ["nrg_bal_s"]
  }
}
```

---

## Database Connection

### Connection String
```
postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_HOST}:${POSTGRES_PORT}/${POSTGRES_DB}
```

### Environment Variables
- `DATABASE_URL`: Full PostgreSQL connection string
- `POSTGRES_HOST`: Database host (default: 172.18.0.1)
- `POSTGRES_PORT`: Database port (default: 5432)
- `POSTGRES_USER`: Database user (default: postgres)
- `POSTGRES_PASSWORD`: Database password (required)
- `POSTGRES_DB`: Database name (default: lianel_energy)

---

## Authentication

Reuse Keycloak validation pattern from `profile-service`:
- Token validation via Keycloak userinfo endpoint
- Support for Bearer tokens
- Optional: Support for OAuth2-proxy headers (X-User, X-Email)

---

## Error Handling

Standard error response format:
```json
{
  "error": "Error message",
  "details": "Optional detailed error information"
}
```

HTTP Status Codes:
- `200`: Success
- `400`: Bad Request (invalid parameters)
- `401`: Unauthorized (missing/invalid token)
- `404`: Not Found
- `500`: Internal Server Error

---

## Implementation Plan

### Phase 1: Core Endpoints (Week 1)
1. Database connection setup
2. Basic query endpoints (`/api/energy/annual`)
3. Health check
4. Keycloak integration

### Phase 2: Advanced Queries (Week 2)
1. Summary/aggregation endpoints
2. Regional data endpoints
3. Reference data endpoints

### Phase 3: Quality & Metadata (Week 3)
1. Data quality endpoint
2. Service info endpoint
3. OpenAPI documentation

### Phase 4: Optimization (Week 4)
1. Query optimization
2. Caching (if needed)
3. Rate limiting
4. Performance testing

---

## File Structure

```
energy-service/
├── Cargo.toml
├── Dockerfile
├── README.md
├── src/
│   ├── main.rs          # Main application setup
│   ├── handlers/        # Request handlers
│   │   ├── energy.rs    # Energy data endpoints
│   │   ├── regions.rs   # Regional data endpoints
│   │   ├── metadata.rs  # Reference data endpoints
│   │   └── quality.rs   # Quality check endpoints
│   ├── db/              # Database layer
│   │   ├── mod.rs
│   │   └── queries.rs   # SQL queries
│   ├── models/          # Data models
│   │   ├── mod.rs
│   │   ├── energy.rs
│   │   └── region.rs
│   ├── auth/            # Authentication
│   │   └── keycloak.rs  # Keycloak validator (reuse pattern)
│   └── config.rs        # Configuration
```

---

## Dependencies (Cargo.toml)

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls", "chrono"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
reqwest = { version = "0.11", features = ["json"] }
utoipa = { version = "4.2", features = ["axum_extras", "chrono"] }
utoipa-swagger-ui = { version = "5.9", features = ["axum"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
chrono = { version = "0.4", features = ["serde"] }
```

---

## Next Steps

1. Create service structure
2. Set up database connection
3. Implement basic endpoints
4. Add Keycloak authentication
5. Add OpenAPI documentation
6. Test with real data
7. Deploy to production

---

**Status**: Ready for implementation

