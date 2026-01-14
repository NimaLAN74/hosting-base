# User Guide: Energy Data API

**Version**: 1.0  
**Last Updated**: January 14, 2026  
**Base URL**: `https://www.lianel.se/api/v1`

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Authentication](#authentication)
3. [ML Dataset Endpoints](#ml-dataset-endpoints)
4. [ENTSO-E Electricity Endpoints](#entso-e-electricity-endpoints)
5. [OSM Geo Features Endpoints](#osm-geo-features-endpoints)
6. [Query Parameters](#query-parameters)
7. [Response Format](#response-format)
8. [Error Handling](#error-handling)
9. [Examples](#examples)
10. [Best Practices](#best-practices)

---

## Getting Started

### Base URL
All API endpoints are available at:
```
https://www.lianel.se/api/v1
```

### Interactive Documentation
For interactive API exploration, visit:
```
https://www.lianel.se/swagger-ui/
```

### Quick Test
```bash
# Health check (no auth required)
curl https://www.lianel.se/api/v1/health

# Get forecasting data (auth required)
curl -H "Authorization: Bearer <your-token>" \
  "https://www.lianel.se/api/v1/datasets/forecasting?limit=10"
```

---

## Authentication

All API endpoints require authentication via Keycloak Bearer token.

### Getting a Token

1. **Via Browser**: Login at https://www.lianel.se
2. **Via API**: Use Keycloak token endpoint
   ```bash
   curl -X POST "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token" \
     -d "client_id=frontend-client" \
     -d "username=your-username" \
     -d "password=your-password" \
     -d "grant_type=password"
   ```

### Using the Token

Include the token in the `Authorization` header:
```bash
curl -H "Authorization: Bearer <your-token>" \
  "https://www.lianel.se/api/v1/datasets/forecasting"
```

---

## ML Dataset Endpoints

### 1. Forecasting Dataset

**Endpoint**: `GET /api/v1/datasets/forecasting`

**Description**: Time-series dataset for forecasting models with lagged values, rolling statistics, and trend indicators.

**Query Parameters**:
- `cntr_code` (optional): Country code (e.g., "SE", "DE", "FR")
- `year` (optional): Year filter (e.g., 2024)
- `limit` (optional): Max records (default: 1000, max: 10000)
- `offset` (optional): Pagination offset (default: 0)

**Example**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/forecasting?cntr_code=SE&year=2024&limit=100"
```

**Response Fields**:
- `cntr_code`: Country code
- `year`: Year
- `total_energy_gwh`: Total energy consumption
- `renewable_energy_gwh`: Renewable energy
- `fossil_energy_gwh`: Fossil energy
- `pct_renewable`: Renewable percentage
- `yoy_change_total_energy_pct`: Year-over-year change
- `lag_1_year_total_energy_gwh`: Previous year value
- `rolling_3y_mean_total_energy_gwh`: 3-year rolling mean
- `trend_3y_slope`: 3-year trend slope
- `avg_hourly_load_mw`: Average hourly load (ENTSO-E)
- `peak_load_mw`: Peak load (ENTSO-E)

---

### 2. Clustering Dataset

**Endpoint**: `GET /api/v1/datasets/clustering`

**Description**: Dataset for clustering analysis with energy mix and spatial features.

**Query Parameters**: Same as forecasting endpoint

**Example**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/clustering?year=2024"
```

**Response Fields**:
- All fields from forecasting dataset
- `power_plant_count`: Number of power plants (OSM)
- `industrial_area_km2`: Industrial area (OSM)
- `power_plant_density_per_km2`: Power plant density

---

### 3. Geo Enrichment Dataset

**Endpoint**: `GET /api/v1/datasets/geo-enrichment`

**Description**: Geospatial enrichment dataset with regional features.

**Query Parameters**: Same as forecasting endpoint

**Example**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/geo-enrichment?cntr_code=SE"
```

---

## ENTSO-E Electricity Endpoints

### Electricity Timeseries

**Endpoint**: `GET /api/v1/electricity/timeseries`

**Description**: High-frequency electricity data from ENTSO-E (load and generation).

**Query Parameters**:
- `country_code` (optional): ISO country code (e.g., "SE", "DE")
- `start_date` (optional): Start date (YYYY-MM-DD)
- `end_date` (optional): End date (YYYY-MM-DD)
- `production_type` (optional): Production type code
- `limit` (optional): Max records (default: 1000)
- `offset` (optional): Pagination offset (default: 0)

**Example**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/electricity/timeseries?country_code=SE&start_date=2024-01-01&limit=100"
```

**Response Fields**:
- `timestamp_utc`: Timestamp (UTC)
- `country_code`: Country code
- `bidding_zone`: Bidding zone identifier
- `production_type`: Production type code
- `load_mw`: Load in megawatts
- `generation_mw`: Generation in megawatts
- `resolution`: Time resolution (PT15M, PT60M)
- `quality_flag`: Data quality flag

---

## OSM Geo Features Endpoints

### Geo Features

**Endpoint**: `GET /api/v1/geo/features`

**Description**: OpenStreetMap features aggregated by NUTS2 region.

**Query Parameters**:
- `region_id` (optional): NUTS2 region code (e.g., "SE11", "DE11")
- `feature_name` (optional): Feature name (e.g., "power_plant_count")
- `snapshot_year` (optional): Snapshot year
- `limit` (optional): Max records (default: 1000)
- `offset` (optional): Pagination offset (default: 0)

**Example**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/geo/features?region_id=SE11&snapshot_year=2024"
```

**Response Fields**:
- `region_id`: NUTS2 region code
- `feature_name`: Feature name (e.g., "power_plant_count", "industrial_area_km2")
- `feature_value`: Feature value
- `snapshot_year`: Year of data snapshot

**Available Feature Types**:
- `power_plant_count`: Number of power plants
- `power_generator_count`: Number of generators
- `power_substation_count`: Number of substations
- `industrial_area_km2`: Industrial area in km²
- `residential_building_count`: Residential buildings
- `commercial_building_count`: Commercial buildings
- `railway_station_count`: Railway stations
- `airport_count`: Airports
- `*_density_per_km2`: Density metrics

---

## Query Parameters

### Common Parameters

All endpoints support:

- **`limit`**: Maximum number of records (default: 1000, max: 10000)
- **`offset`**: Number of records to skip for pagination (default: 0)

### Pagination Example

```bash
# First page (records 0-99)
curl "https://www.lianel.se/api/v1/datasets/forecasting?limit=100&offset=0"

# Second page (records 100-199)
curl "https://www.lianel.se/api/v1/datasets/forecasting?limit=100&offset=100"
```

---

## Response Format

All endpoints return JSON in the following format:

```json
{
  "data": [
    {
      // Record fields...
    }
  ],
  "total": 270,
  "limit": 100,
  "offset": 0
}
```

**Fields**:
- `data`: Array of records
- `total`: Total number of records matching the query
- `limit`: Applied limit
- `offset`: Applied offset

---

## Error Handling

### HTTP Status Codes

- **200 OK**: Request successful
- **400 Bad Request**: Invalid query parameters
- **401 Unauthorized**: Missing or invalid authentication token
- **403 Forbidden**: Insufficient permissions
- **404 Not Found**: Endpoint not found
- **500 Internal Server Error**: Server error

### Error Response Format

```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": {}
}
```

---

## Examples

### Example 1: Get Recent Electricity Data for Sweden

```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/electricity/timeseries?country_code=SE&start_date=2024-01-01&limit=100"
```

### Example 2: Get Forecasting Data for 2024

```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/forecasting?year=2024&limit=1000"
```

### Example 3: Get Power Plant Counts by Region

```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/geo/features?feature_name=power_plant_count&snapshot_year=2024"
```

### Example 4: Python Client Example

```python
import requests

BASE_URL = "https://www.lianel.se/api/v1"
TOKEN = "your-bearer-token"

headers = {"Authorization": f"Bearer {TOKEN}"}

# Get forecasting data
response = requests.get(
    f"{BASE_URL}/datasets/forecasting",
    headers=headers,
    params={"cntr_code": "SE", "year": 2024, "limit": 100}
)

data = response.json()
print(f"Total records: {data['total']}")
for record in data['data']:
    print(f"{record['cntr_code']} {record['year']}: {record['total_energy_gwh']} GWh")
```

---

## Best Practices

### 1. Use Pagination
Always use `limit` and `offset` for large datasets:
```bash
# Good: Paginated request
curl "https://www.lianel.se/api/v1/datasets/forecasting?limit=1000&offset=0"

# Bad: Requesting all data at once
curl "https://www.lianel.se/api/v1/datasets/forecasting?limit=100000"
```

### 2. Filter Early
Use query parameters to filter data server-side:
```bash
# Good: Filtered request
curl "https://www.lianel.se/api/v1/datasets/forecasting?cntr_code=SE&year=2024"

# Bad: Fetch all and filter client-side
curl "https://www.lianel.se/api/v1/datasets/forecasting" | jq '.data[] | select(.cntr_code=="SE")'
```

### 3. Handle Errors Gracefully
Always check HTTP status codes:
```python
response = requests.get(url, headers=headers)
if response.status_code == 200:
    data = response.json()
elif response.status_code == 401:
    print("Authentication required")
else:
    print(f"Error: {response.status_code}")
```

### 4. Cache Tokens
Store and reuse authentication tokens (they expire after a period):
```python
# Cache token
token = get_token()
# Reuse for multiple requests
```

### 5. Use Swagger UI
Explore endpoints interactively at:
```
https://www.lianel.se/swagger-ui/
```

---

## Support

For issues or questions:
- Check Swagger UI: https://www.lianel.se/swagger-ui/
- Review API documentation: See `API-DOCUMENTATION.md`
- Contact support: [Add support contact]

---

**Last Updated**: January 14, 2026