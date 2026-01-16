# Grafana Energy Dashboards Guide

**Date**: January 12, 2026  
**Status**: ✅ **DEPLOYED**

---

## Dashboard Access

The new energy dashboards are now available in Grafana at: **https://monitoring.lianel.se**

### How to Find the Dashboards

1. **Login to Grafana**
   - Navigate to: https://monitoring.lianel.se
   - Login with your Keycloak credentials (SSO enabled)

2. **Access Dashboards**
   - Click on **"Dashboards"** in the left sidebar (📊 icon)
   - Or go to: https://monitoring.lianel.se/dashboards

3. **Look for**:
   - **"Energy Metrics Dashboard"** (UID: `energy-metrics`)
   - **"Regional Energy Comparison"** (UID: `regional-comparison`)

---

## Available Dashboards

### 1. Energy Metrics Dashboard

**Title**: Energy Metrics Dashboard  
**UID**: `energy-metrics`  
**Direct URL**: https://monitoring.lianel.se/d/energy-metrics

**Panels**:
1. Total Energy Consumption by Country Over Time
2. Average Renewable Energy Percentage (Last 3 Years)
3. Year-over-Year Energy Change Percentage
4. Energy Density by Country (Latest Year)
5. Energy Consumption Distribution by Country (Latest Year)
6. Average Year-over-Year Change (Last 3 Years)

**Data Source**: PostgreSQL Energy  
**Refresh Rate**: 30 seconds  
**Time Range**: Last 10 years (2015-2024)

### 2. Regional Energy Comparison

**Title**: Regional Energy Comparison  
**UID**: `regional-comparison`  
**Direct URL**: https://monitoring.lianel.se/d/regional-comparison

**Panels**:
1. Total Energy Consumption by Country (Latest Year)
2. Energy Density Comparison (Latest Year)
3. Renewable Energy Percentage Over Time by Country
4. Renewable to Fossil Ratio (Latest Year)
5. 3-Year Rolling Average Energy Consumption
6. Energy by Regional Characteristics

**Data Source**: PostgreSQL Energy  
**Refresh Rate**: 30 seconds  
**Time Range**: Last 10 years (2015-2024)

---

## Data Source Configuration

**PostgreSQL Energy Datasource**:
- **Name**: PostgreSQL Energy
- **Type**: PostgreSQL
- **Host**: 172.18.0.1:5432
- **Database**: lianel_energy
- **User**: airflow
- **UID**: `postgres-energy`

**Note**: The password is configured via environment variable `${POSTGRES_PASSWORD}`.

---

## Troubleshooting

### Dashboards Not Visible?

1. **Refresh the browser** (Ctrl+F5 or Cmd+Shift+R)
2. **Check dashboard folder**: The dashboards are in the default folder (no subfolder)
3. **Search by name**: Use the search box in Grafana to search for "Energy" or "Regional"
4. **Check permissions**: Ensure you have Viewer or Editor role

### Data Not Loading?

1. **Check datasource connection**:
   - Go to Configuration → Data Sources
   - Click on "PostgreSQL Energy"
   - Click "Test" to verify connection

2. **Verify database access**:
   - Ensure PostgreSQL is accessible from Grafana container
   - Check that `airflow` user has SELECT permissions

3. **Check logs**:
   ```bash
   docker logs grafana --tail 50
   ```

### Dashboard Errors?

1. **Check panel errors**: Look for red error messages in panels
2. **Verify SQL queries**: Check if queries are valid for your database schema
3. **Check time range**: Ensure selected time range has data

---

## Dashboard Features

### Interactive Features
- **Time Range Selector**: Change time range in top-right corner
- **Auto-refresh**: Dashboards refresh every 30 seconds
- **Panel Interactions**: Click on legends to filter data
- **Zoom**: Click and drag on time series to zoom in

### Data Filters
- All panels show data for all 27 EU countries
- Time range can be adjusted (default: last 10 years)
- Latest year data is used for snapshot panels

---

## Next Steps

1. **Explore the dashboards** and verify data is loading correctly
2. **Customize as needed** (dashboards are editable)
3. **Create additional panels** if needed for specific metrics
4. **Set up alerts** for important thresholds (optional)

---

**Status**: ✅ **READY TO USE**  
**Last Updated**: January 12, 2026
