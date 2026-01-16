# User Guide: Grafana Dashboards

**Last Updated**: January 14, 2026  
**Access**: https://www.lianel.se/grafana

---

## Overview

Grafana dashboards provide visualizations of energy data, trends, and regional comparisons. All dashboards are accessible via SSO authentication.

---

## Available Dashboards

### 1. Energy Metrics Dashboard

**Purpose**: Overview of energy consumption, renewable percentages, and year-over-year changes.

**Key Panels**:
- **Total Energy Consumption**: Total energy by country over time
- **Renewable Energy Percentage**: Renewable energy share by country
- **Year-over-Year Change**: Percentage change in energy consumption
- **Average YoY Change**: Rolling average of changes

**Usage**:
1. Navigate to https://www.lianel.se/grafana
2. Login with SSO
3. Select "Energy Metrics" dashboard
4. Use time range selector to adjust period
5. Click on country names to filter by country

---

### 2. Regional Comparison Dashboard

**Purpose**: Compare energy metrics across countries and regions.

**Key Panels**:
- **Renewable Energy Percentage Over Time**: Trend comparison by country
- **Energy Density**: Energy consumption per km²
- **Country Rankings**: Top/bottom countries by various metrics

**Usage**:
1. Select "Regional Comparison" dashboard
2. Use country filter to select specific countries
3. Compare metrics across selected countries
4. Export data if needed

---

## Dashboard Features

### Time Range Selection
- Use the time picker in the top-right
- Select predefined ranges (Last 7 days, Last 30 days, etc.)
- Or set custom date range

### Filtering
- Click on legend items to show/hide series
- Use variable dropdowns to filter by country/region
- Apply multiple filters simultaneously

### Exporting Data
- Click panel title → "More" → "Export CSV"
- Or use "Share" button to export entire dashboard

---

## Data Quality Notes

### Historical Data
- Data from 2016-2017 may have incomplete fossil energy data
- Some countries may show 100% renewable during this period
- This is a data quality issue, not a calculation error

### Year-over-Year Changes
- 2018 YoY changes may be artificially high due to 2017 data quality issues
- These are filtered in the dashboard queries

---

## Troubleshooting

### Dashboard Not Loading
- Check SSO authentication
- Verify you have access permissions
- Try refreshing the page

### Missing Data
- Check time range selection
- Verify data has been ingested for selected period
- Check DAG execution status in Airflow

### Performance Issues
- Reduce time range
- Apply filters to reduce data volume
- Contact administrator if issues persist

---

**For more information**: See `GRAFANA-DASHBOARDS-GUIDE.md`