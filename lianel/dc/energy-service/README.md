# Lianel Energy Service

Backend service written in Rust that provides REST API endpoints for querying energy data from the `lianel_energy` database.

## Technology Stack

- **Language**: Rust
- **Web Framework**: Axum
- **Database**: PostgreSQL (via `sqlx`)
- **Authentication**: Keycloak (reuse existing validator pattern)
- **API Documentation**: OpenAPI/Swagger UI (utoipa)
- **Serialization**: Serde

## Features

- **Energy Data Queries**: Query annual energy data with filtering
- **Summary Statistics**: Aggregated summaries by country, year, product, or flow
- **Metadata Endpoints**: Lists of countries, products, flows
- **Service Information**: Database statistics and service info
- **OpenAPI Documentation**: Interactive Swagger UI

## API Endpoints

### Energy Data

- `GET /api/energy/annual` - Get annual energy data with filtering
- `GET /api/energy/annual/by-country/{country_code}` - Get data for specific country
- `GET /api/energy/annual/by-year/{year}` - Get data for specific year
- `GET /api/energy/annual/summary` - Get aggregated summary statistics

### Metadata

- `GET /api/info` - Service information and database statistics

### Health

- `GET /health` - Health check endpoint

### Documentation

- `GET /swagger-ui` - Swagger UI interactive documentation
- `GET /api-doc/openapi.json` - OpenAPI specification

## Environment Variables

- `PORT`: Server port (default: 3001)
- `POSTGRES_HOST`: Database host (default: 172.18.0.1)
- `POSTGRES_PORT`: Database port (default: 5432)
- `POSTGRES_USER`: Database user (default: postgres)
- `POSTGRES_PASSWORD`: Database password (required)
- `POSTGRES_DB`: Database name (default: lianel_energy)
- `DATABASE_URL`: Full PostgreSQL connection string (optional, overrides individual settings)
- `KEYCLOAK_URL`: Keycloak server URL (default: http://keycloak:8080)
- `KEYCLOAK_REALM`: Keycloak realm name (default: lianel)
- `RUST_LOG`: Logging level (default: info)

## Building

### Local Development

```bash
cd lianel/dc/energy-service
cargo build --release
cargo run
```

### Docker Build

```bash
docker build -t lianel-energy-service:latest .
```

## Deployment

The service is included in `docker-compose.backend.yaml` and will be deployed automatically via CI/CD.

## Example Queries

```bash
# Get all energy data
curl http://localhost:3001/api/energy/annual

# Get data for Germany
curl http://localhost:3001/api/energy/annual?country_code=DE

# Get data for 2021
curl http://localhost:3001/api/energy/annual?year=2021

# Get summary by country
curl http://localhost:3001/api/energy/annual/summary?group_by=country

# Health check
curl http://localhost:3001/health
```

## Status

🚧 **In Development** - Core endpoints implemented, ready for testing and deployment.

