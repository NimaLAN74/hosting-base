# Energy Service API Documentation

**Version**: 1.0  
**Base URL**: `https://www.lianel.se/api/v1`  
**Authentication**: Keycloak Bearer Token (via Authorization header)

---

## Overview

The Energy Service provides REST API endpoints for accessing energy data and ML datasets. All endpoints support filtering, pagination, and return JSON responses.

---

## ML Dataset Endpoints

### 1. Forecasting Dataset

**Endpoint**: `GET /api/v1/datasets/forecasting`

**Description**: Retrieve forecasting dataset with time-based features, lagged values, rolling statistics, and trend indicators.

**Query Parameters**:
- `cntr_code` (optional): Filter by country code (e.g., "SE", "DE")
- `year` (optional): Filter by year (e.g., 2024)
- `limit` (optional): Maximum number of records (default: 1000, max: 10000)
- `offset` (optional): Number of records to skip (default: 0)

**Example Request**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/forecasting?cntr_code=SE&year=2024&limit=100"
```

**Response**:
```json
{
  "data": [
    {
      "cntr_code": "SE",
      "year": 2024,
      "total_energy_gwh": 8781015.65,
      "renewable_energy_gwh": 1491833.97,
      "fossil_energy_gwh": 87692.72,
      "pct_renewable": 16.99,
      "pct_fossil": 1.0,
      "yoy_change_total_energy_pct": 0.261,
      "yoy_change_renewable_pct": -2.97,
      "energy_density_gwh_per_km2": 19.5,
      "area_km2": 450295.0,
      "year_index": 10,
      "lag_1_year_total_energy_gwh": 8758167.22,
      "lag_2_year_total_energy_gwh": 8541514.31,
      "rolling_3y_mean_total_energy_gwh": 8693399.36,
      "rolling_5y_mean_total_energy_gwh": 8623456.78,
      "trend_3y_slope": 12345.67,
      "trend_5y_slope": 9876.54
    }
  ],
  "total": 270,
  "limit": 100,
  "offset": 0
}
```

---

### 2. Clustering Dataset

**Endpoint**: `GET /api/v1/datasets/clustering`

**Description**: Retrieve clustering dataset with energy mix calculations and spatial features.

**Query Parameters**: Same as forecasting endpoint

**Example Request**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/clustering?year=2024"
```

**Response**:
```json
{
  "data": [
    {
      "cntr_code": "SE",
      "year": 2024,
      "total_energy_gwh": 8781015.65,
      "renewable_energy_gwh": 1491833.97,
      "fossil_energy_gwh": 87692.72,
      "pct_renewable": 16.99,
      "pct_fossil": 1.0,
      "energy_density_gwh_per_km2": 19.5,
      "area_km2": 450295.0
    }
  ],
  "total": 270,
  "limit": 1000,
  "offset": 0
}
```

---

### 3. Geo-Enrichment Dataset

**Endpoint**: `GET /api/v1/datasets/geo-enrichment`

**Description**: Retrieve geo-enrichment dataset combining energy data with spatial features.

**Query Parameters**: Same as forecasting endpoint

**Example Request**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/geo-enrichment?cntr_code=DE"
```

**Response**:
```json
{
  "data": [
    {
      "cntr_code": "DE",
      "year": 2024,
      "total_energy_gwh": 43402410.53,
      "renewable_energy_gwh": 3184944.74,
      "fossil_energy_gwh": 4843221.91,
      "pct_renewable": 7.34,
      "energy_density_gwh_per_km2": 121.5,
      "area_km2": 357022.0
    }
  ],
  "total": 270,
  "limit": 1000,
  "offset": 0
}
```

---

## Energy Data Endpoints

### 4. Annual Energy Data

**Endpoint**: `GET /api/energy/annual`

**Description**: Retrieve annual energy data from fact tables.

**Query Parameters**:
- `country_code` (optional): Filter by country code
- `year` (optional): Filter by year
- `product_code` (optional): Filter by product code
- `flow_code` (optional): Filter by flow code
- `source_table` (optional): Filter by source table
- `limit` (optional): Maximum records (default: 1000, max: 10000)
- `offset` (optional): Records to skip (default: 0)

**Example Request**:
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/energy/annual?country_code=SE&year=2024"
```

---

## Authentication

All endpoints require Keycloak authentication via Bearer token in the Authorization header:

```
Authorization: Bearer <your-keycloak-token>
```

**Getting a Token**:
1. Login at `https://www.lianel.se`
2. OAuth2-proxy handles authentication
3. Frontend receives token automatically
4. Include token in API requests

---

## OpenAPI Documentation

Interactive API documentation is available at:
- **Swagger UI**: `https://www.lianel.se/api/energy/swagger-ui`
- **OpenAPI Spec**: `https://www.lianel.se/api/energy/api-doc/openapi.json`

---

## Error Responses

All endpoints return standard HTTP status codes:

- `200 OK`: Request successful
- `400 Bad Request`: Invalid query parameters
- `401 Unauthorized`: Missing or invalid authentication token
- `500 Internal Server Error`: Server error

**Error Response Format**:
```json
{
  "error": "Error message",
  "details": "Detailed error information"
}
```

---

## Rate Limiting

- **General endpoints**: 10 requests/second per IP
- **Burst**: 20 requests allowed
- Rate limit headers included in responses

---

## Pagination

All dataset endpoints support pagination:
- Use `limit` to control page size (max 10,000)
- Use `offset` to skip records
- Response includes `total` count for calculating pages

**Example Pagination**:
```bash
# Page 1 (first 100 records)
GET /api/v1/datasets/forecasting?limit=100&offset=0

# Page 2 (next 100 records)
GET /api/v1/datasets/forecasting?limit=100&offset=100
```

---

## Data Quality Notes

- **2015-2017**: Incomplete fossil data (excluded from some calculations)
- **2018**: Complete data but invalid YoY baseline (excluded from YoY charts)
- **2019-2024**: Complete and accurate data

Filter incomplete years in your queries if needed:
```bash
GET /api/v1/datasets/forecasting?year=2024  # Only complete years
```

---

## Examples

### Get all forecasting data for Sweden
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/forecasting?cntr_code=SE"
```

### Get clustering data for 2024
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/clustering?year=2024"
```

### Get geo-enrichment data with pagination
```bash
curl -H "Authorization: Bearer <token>" \
  "https://www.lianel.se/api/v1/datasets/geo-enrichment?limit=50&offset=0"
```

---

## Support

For issues or questions:
- Check Swagger UI documentation
- Review OpenAPI specification
- Check service logs for errors
