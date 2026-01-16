# Dashboard Tutorials
**Last Updated**: January 15, 2026  
**Access**: https://www.lianel.se/grafana

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Energy Metrics Dashboard](#energy-metrics-dashboard)
3. [Regional Comparison Dashboard](#regional-comparison-dashboard)
4. [System Health Dashboard](#system-health-dashboard)
5. [Pipeline Status Dashboard](#pipeline-status-dashboard)
6. [Web Analytics Dashboard](#web-analytics-dashboard)
7. [Tips and Tricks](#tips-and-tricks)

---

## Getting Started

### Accessing Dashboards

1. Navigate to: `https://www.lianel.se/grafana`
2. Login with SSO (Keycloak)
3. Browse available dashboards from the left menu

### Dashboard Navigation

- **Time Range Selector**: Top-right corner - adjust time period
- **Refresh**: Click refresh icon to update data
- **Panel Options**: Click panel title → "Edit" to customize
- **Export**: Click panel title → "Share" to export data

---

## Energy Metrics Dashboard

### Purpose
Monitor energy consumption, renewable energy percentages, and trends over time.

### Key Panels

#### 1. Total Energy Consumption
**What it shows**: Total energy consumption by country over time

**How to use**:
1. Select time range (e.g., last 5 years)
2. Click on country names in legend to show/hide countries
3. Hover over data points to see exact values

**Interpretation**:
- Upward trend = increasing energy consumption
- Downward trend = decreasing energy consumption
- Compare countries to identify patterns

#### 2. Renewable Energy Percentage
**What it shows**: Percentage of renewable energy by country

**How to use**:
1. Select countries to compare
2. Use time range to see trends
3. Look for countries with high renewable percentages

**Interpretation**:
- Higher percentage = more renewable energy
- Increasing trend = transition to renewables
- Compare with EU average

#### 3. Year-over-Year Change
**What it shows**: Percentage change in energy consumption compared to previous year

**How to use**:
1. Select specific countries
2. Identify positive/negative changes
3. Compare with historical averages

**Interpretation**:
- Positive = increased consumption
- Negative = decreased consumption
- Large changes may indicate data issues or events

### Tutorial: Analyzing Energy Trends

**Step 1**: Open Energy Metrics Dashboard

**Step 2**: Select time range "Last 5 years"

**Step 3**: In "Total Energy Consumption" panel:
- Click on "Germany" to highlight it
- Observe the trend (increasing/decreasing)
- Note any significant changes

**Step 4**: Switch to "Renewable Energy Percentage" panel:
- Compare Germany's renewable percentage with EU average
- Identify countries with highest renewable percentages

**Step 5**: Export data:
- Click panel title → "Share" → "Export CSV"
- Use for further analysis

---

## Regional Comparison Dashboard

### Purpose
Compare energy metrics across countries and regions.

### Key Panels

#### 1. Renewable Energy Percentage Over Time
**What it shows**: Trend comparison of renewable energy by country

**How to use**:
1. Select multiple countries
2. Compare trends
3. Identify leaders in renewable energy

#### 2. Energy Density
**What it shows**: Energy consumption per km²

**How to use**:
1. Compare countries by density
2. Consider country size when interpreting
3. Identify high-density regions

#### 3. Country Rankings
**What it shows**: Top/bottom countries by various metrics

**How to use**:
1. Select metric (renewable %, consumption, etc.)
2. View rankings
3. Compare with previous periods

### Tutorial: Comparing Countries

**Step 1**: Open Regional Comparison Dashboard

**Step 2**: In "Renewable Energy Percentage Over Time":
- Select "Germany", "France", "Italy", "Spain"
- Compare their renewable energy trends
- Identify which country is leading

**Step 3**: In "Energy Density" panel:
- Compare energy consumption per area
- Consider that smaller countries may have higher density

**Step 4**: In "Country Rankings" panel:
- Select "Renewable Percentage"
- View top 10 countries
- Compare with previous year

---

## System Health Dashboard

### Purpose
Monitor overall system health, resource usage, and service status.

### Key Panels

#### 1. Service Status
**What it shows**: Status of all services (up/down)

**How to use**:
1. Check all services are "Up" (green)
2. Investigate any "Down" services (red)
3. Click on service for details

**Interpretation**:
- Green = Service is running
- Red = Service is down (investigate immediately)
- Yellow = Service is degraded

#### 2. CPU Usage
**What it shows**: CPU usage by container/service

**How to use**:
1. Monitor CPU usage over time
2. Identify services with high CPU usage
3. Set up alerts for high usage

**Interpretation**:
- < 50% = Normal
- 50-80% = Moderate (monitor)
- > 80% = High (investigate)

#### 3. Memory Usage
**What it shows**: Memory usage by container

**How to use**:
1. Monitor memory trends
2. Identify memory leaks
3. Check for OOM (Out of Memory) issues

**Interpretation**:
- Stable = Normal
- Increasing trend = Possible memory leak
- Near limit = Risk of OOM

### Tutorial: Monitoring System Health

**Step 1**: Open System Health Dashboard

**Step 2**: Check "Service Status" panel:
- Verify all services are green
- Note any red services

**Step 3**: Review "CPU Usage" panel:
- Identify services with highest CPU usage
- Check if usage is within normal range

**Step 4**: Review "Memory Usage" panel:
- Check for any services near memory limits
- Look for increasing trends (memory leaks)

**Step 5**: Set up alerts:
- Click panel title → "Alert" → "Create Alert"
- Configure thresholds (e.g., CPU > 80%)

---

## Pipeline Status Dashboard

### Purpose
Monitor Airflow DAG execution, task status, and pipeline health.

### Key Panels

#### 1. Running DAGs
**What it shows**: Currently running DAGs and their status

**How to use**:
1. Monitor active DAGs
2. Check execution duration
3. Identify long-running DAGs

**Interpretation**:
- Green = Running successfully
- Red = Failed (investigate)
- Yellow = Warning

#### 2. Failed Tasks
**What it shows**: Number of failed tasks in last 24 hours

**How to use**:
1. Monitor failure rate
2. Click on failed tasks for details
3. Investigate root causes

**Interpretation**:
- 0 failures = Normal
- Few failures = May be transient
- Many failures = Investigate immediately

#### 3. Success Rate
**What it shows**: Percentage of successful task executions

**How to use**:
1. Monitor success rate over time
2. Identify trends (improving/degrading)
3. Set alerts for low success rates

**Interpretation**:
- > 95% = Good
- 90-95% = Acceptable (monitor)
- < 90% = Poor (investigate)

### Tutorial: Monitoring Pipelines

**Step 1**: Open Pipeline Status Dashboard

**Step 2**: Check "Running DAGs" panel:
- Verify expected DAGs are running
- Check execution times

**Step 3**: Review "Failed Tasks" panel:
- Identify any failed tasks
- Click on task for error details

**Step 4**: Monitor "Success Rate" panel:
- Check if success rate is above 95%
- Identify any downward trends

**Step 5**: Investigate failures:
- Click on failed task
- Review error logs
- Follow runbook for recovery

---

## Web Analytics Dashboard

### Purpose
Monitor website traffic, user behavior, and access patterns.

### Key Panels

#### 1. Unique Visitors (by IP)
**What it shows**: Number of unique visitors in selected time period

**How to use**:
1. Select time range (e.g., last 24 hours)
2. Compare with previous periods
3. Identify traffic patterns

**Interpretation**:
- Increasing = Growing traffic
- Decreasing = May indicate issues
- Compare with marketing campaigns

#### 2. Requests by Host
**What it shows**: Number of requests per hostname

**How to use**:
1. Identify most accessed hosts
2. Monitor traffic distribution
3. Detect unusual patterns

**Interpretation**:
- High requests = Popular endpoints
- Unusual spikes = May indicate attacks
- Compare with expected patterns

#### 3. Recent Access Logs
**What it shows**: Recent API requests with details

**How to use**:
1. Review recent requests
2. Identify patterns
3. Debug issues

**Interpretation**:
- 200 status = Success
- 4xx status = Client errors
- 5xx status = Server errors

### Tutorial: Analyzing Web Traffic

**Step 1**: Open Web Analytics Dashboard

**Step 2**: Check "Unique Visitors" panel:
- Select "Last 24 hours"
- Compare with previous day
- Note any significant changes

**Step 3**: Review "Requests by Host" panel:
- Identify most accessed hosts
- Check for unusual patterns

**Step 4**: Review "Recent Access Logs" panel:
- Check for errors (4xx, 5xx)
- Identify problematic endpoints
- Review user agents

---

## Tips and Tricks

### 1. Custom Time Ranges
- Create custom time ranges for specific analysis
- Save frequently used ranges

### 2. Panel Linking
- Link panels to drill down into details
- Create navigation between dashboards

### 3. Variables
- Use dashboard variables for filtering
- Create dynamic dashboards

### 4. Annotations
- Add annotations for important events
- Mark deployments, incidents, etc.

### 5. Alerts
- Set up alerts for critical metrics
- Configure notification channels

### 6. Export Data
- Export panel data as CSV
- Use for external analysis

### 7. Share Dashboards
- Share dashboards with team members
- Create public links (if allowed)

### 8. Keyboard Shortcuts
- `Ctrl/Cmd + K`: Open command palette
- `Ctrl/Cmd + S`: Save dashboard
- `Ctrl/Cmd + E`: Toggle edit mode

---

## Common Issues

### Dashboard Not Loading
- Check internet connection
- Verify SSO authentication
- Clear browser cache

### No Data Showing
- Check time range
- Verify data source is available
- Check for errors in panel

### Slow Performance
- Reduce time range
- Limit number of series
- Check data source performance

---

**Status**: Active  
**Last Review**: January 15, 2026
