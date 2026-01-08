# D1 — Eurostat Energy Data Inventory  
Version 2.0  
**Last Updated**: January 7, 2026  
**Tested**: Remote Host (72.60.80.84)

## 1. Purpose
Inventory and describe Eurostat datasets relevant for EU energy consumption, production, transformation and indicators.

## 2. API Access

### 2.1 API Endpoint
```
Base URL: https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data
Format: JSON
Authentication: None required (public API)
Rate Limits: Not officially documented (recommend: <10 requests/second)
```

### 2.2 Request Format
```
GET {base_url}/{table_code}?format=JSON&geo={country}&time={year}
```

**Example Request**:
```bash
curl "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/nrg_bal_s?format=JSON&geo=SE&time=2022"
```

**Python Example**:
```python
import requests

url = "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/nrg_bal_s"
params = {
    "format": "JSON",
    "geo": "SE",      # Country code (ISO 2-letter)
    "time": "2022"    # Year
}

response = requests.get(url, params=params)
data = response.json()
```

### 2.3 Response Structure
```json
{
  "version": "2.0",
  "class": "dataset",
  "label": "Simplified energy balance - annual data",
  "source": "Eurostat",
  "updated": "2024-01-15",
  "value": {
    "0": 12345.67,
    "1": 23456.78,
    ...
  },
  "dimension": {
    "freq": {...},
    "nrg_bal": {...},
    "siec": {...},
    "unit": {...},
    "geo": {...},
    "time": {...}
  },
  "id": ["freq", "nrg_bal", "siec", "unit", "geo", "time"],
  "size": [1, 95, 50, 5, 1, 1]
}
```

## 3. Main Dataset Families

### 3.1 Priority Tables

| Table Code | Description | Status | Total Values | Dimensions | Notes |
|------------|-------------|---------|--------------|------------|-------|
| `nrg_bal_c` | Complete energy balances | ⚠️ Very large | ~15M+ | freq, nrg_bal, siec, unit, geo, time | **Requires filtered queries** |
| `nrg_bal_s` | Simplified energy balances | ✅ Tested | 2,847,222 | freq, nrg_bal, siec, unit, geo, time | Primary dataset for analysis |
| `nrg_cb_e` | Energy supply and consumption | ✅ Tested | 74,768 | freq, nrg_bal, siec, unit, geo, time | Detailed supply chain |
| `nrg_ind_eff` | Energy efficiency indicators | ✅ Tested | 3,076 | freq, nrg_bal, unit, geo, time | ML features |
| `nrg_ind_ren` | Renewable energy indicators | ✅ Tested | 3,152 | freq, nrg_bal, unit, geo, time | ML features |

### 3.2 Table Details

#### nrg_bal_s (Simplified Energy Balances) - **PRIMARY DATASET**
- **Description**: Annual simplified energy balances for all EU countries
- **Dimensions**: 
  - `freq`: Frequency (always "A" for annual)
  - `nrg_bal`: Energy balance flow (95 categories: PPRD, RCV_RCY, IMP, EXP, STK_CHG, etc.)
  - `siec`: Energy product classification (SIEC codes)
  - `unit`: Measurement unit (ktoe, TJ, GWh, etc.)
  - `geo`: Geographical area (country codes)
  - `time`: Time period (year)
- **Data Volume**: 2.8M+ total values
- **Sample Query Result**: Sweden 2022 = 2,154 values
- **Use Case**: Primary dataset for energy mix analysis, regional clustering

#### nrg_bal_c (Complete Energy Balances)
- **Description**: Complete detailed energy balances
- **Warning**: ⚠️ **Too large for unfiltered queries** - will cause memory/timeout issues
- **Recommendation**: Always use filtered queries (geo + time parameters)
- **Use Case**: Detailed analysis when simplified balances insufficient

#### nrg_cb_e (Energy Supply and Consumption)
- **Description**: Energy supply and consumption by product
- **Data Volume**: 74,768 total values
- **Sample Query Result**: Sweden 2022 = 60 values
- **Use Case**: Supply chain analysis

#### nrg_ind_eff (Energy Efficiency Indicators)
- **Description**: Energy efficiency indicators
- **Dimensions**: freq, nrg_bal, unit, geo, time (no siec dimension)
- **Data Volume**: 3,076 total values
- **Sample Query Result**: Sweden 2022 = 4 values
- **Use Case**: ML features for efficiency analysis

#### nrg_ind_ren (Renewable Energy Indicators)
- **Description**: Renewable energy share indicators
- **Dimensions**: freq, nrg_bal, unit, geo, time (no siec dimension)
- **Data Volume**: 3,152 total values
- **Sample Query Result**: Sweden 2022 = 4 values
- **Use Case**: ML features for renewable energy analysis

## 4. Key Dimensions

### 4.1 Common Dimensions
- **GEO (geo)**: Country code (ISO 2-letter: SE, DE, FR, etc.)
- **TIME (time)**: Year (format: "2020", "2021", etc.)
- **NRG_BAL (nrg_bal)**: Energy balance flow (e.g., PPRD, IMP, EXP, STK_CHG)
- **PRODUCT (siec)**: Energy product classification (SIEC codes)
- **UNIT (unit)**: Measurement unit (ktoe, TJ, GWh, etc.)
- **FREQ (freq)**: Frequency (always "A" for annual data)

### 4.2 Dimension Values

**nrg_bal (Energy Balance Flows)** - Sample values:
- `PPRD`: Primary production
- `RCV_RCY`: Recovered and recycled products
- `IMP`: Imports
- `EXP`: Exports
- `STK_CHG`: Stock changes
- ... (95 total categories in nrg_bal_s)

**siec (Energy Products)** - Sample values:
- Various SIEC codes for different energy products
- ~50 categories in nrg_bal_s

**unit (Units)** - Common values:
- `KTOE`: Kilotonnes of oil equivalent
- `TJ`: Terajoules
- `GWH`: Gigawatt-hours
- `PC`: Percentage

## 5. Time Coverage
- **Balances**: ~1960s → latest full year
- **Final consumption and indicators**: generally from 1990s onward
- **Latest Data**: Typically 1-2 years behind current year (e.g., 2024 data available in 2025)

## 6. Data Quality

### 6.1 Known Issues
- Sporadic gaps in early years for some countries
- Methodological updates over time (products and flows)
- Mixed units across tables (harmonised later via harmonization rules)
- Some countries have incomplete data for certain years

### 6.2 Data Completeness
- **EU27 Coverage**: Generally good from 1990s onward
- **Early Years**: Incomplete for some countries (especially newer EU members)
- **Recommendation**: Test coverage per country/year before full ingestion

## 7. Best Practices for Querying

### 7.1 Always Use Filtered Queries
❌ **Don't**: Query entire table without filters
```python
# BAD - Will timeout or use excessive memory
requests.get(f"{BASE_URL}/nrg_bal_c?format=JSON")
```

✅ **Do**: Always filter by geo and time
```python
# GOOD - Filtered query
params = {"format": "JSON", "geo": "SE", "time": "2022"}
requests.get(f"{BASE_URL}/nrg_bal_s", params=params)
```

### 7.2 Incremental Processing
- Process country by country
- Process year by year (or small year ranges)
- Store intermediate results
- Handle errors gracefully

### 7.3 Example Processing Strategy
```python
# Process all EU27 countries, year by year
countries = ["AT", "BE", "BG", ...]  # EU27
years = range(1990, 2024)

for country in countries:
    for year in years:
        try:
            data = fetch_eurostat("nrg_bal_s", country, str(year))
            process_and_store(data)
        except Exception as e:
            log_error(country, year, e)
            continue
```

## 8. Use in Platform

### 8.1 Primary Use Cases
- **Macro baseline**: Energy mix and sectoral patterns
- **Long-term trends**: Historical analysis and scenario analysis
- **ML Features**: Annual features for regional clustering and forecasting models
- **Regional Analysis**: NUTS-level aggregation (via country-level data)

### 8.2 Integration Points
- **Airflow DAGs**: Scheduled weekly/monthly ingestion
- **PostgreSQL**: Store in `fact_energy_annual` table
- **Harmonization**: Convert units to GWh (via harmonization rules)
- **ML Datasets**: Feature engineering for clustering and forecasting

## 9. Test Results

**Test Date**: January 7, 2026  
**Test Location**: Remote Host (72.60.80.84)  
**Test Script**: `/root/lianel/dc/scripts/test-eurostat-incremental.py`

### 9.1 Verified Tables
- ✅ `nrg_bal_s`: Accessible, 2.8M values
- ✅ `nrg_cb_e`: Accessible, 74K values
- ✅ `nrg_ind_eff`: Accessible, 3K values
- ✅ `nrg_ind_ren`: Accessible, 3K values
- ⚠️ `nrg_bal_c`: Too large for unfiltered queries

### 9.2 Sample Query Results
- Sweden 2022 (`nrg_bal_s`): 2,154 values
- Sweden 2022 (`nrg_cb_e`): 60 values
- Sweden 2022 (`nrg_ind_eff`): 4 values
- Sweden 2022 (`nrg_ind_ren`): 4 values

## 10. References

- **Eurostat API Documentation**: https://ec.europa.eu/eurostat/web/json-and-unicode-web-services
- **Data Browser**: https://ec.europa.eu/eurostat/databrowser/
- **Getting Started Guide**: https://ec.europa.eu/eurostat/web/json-and-unicode-web-services/getting-started/rest-request
- **Test Results**: `/root/lianel/dc/data/samples/eurostat-api-summary.md` (on remote host)
