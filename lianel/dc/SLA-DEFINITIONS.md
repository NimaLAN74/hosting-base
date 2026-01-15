# Service Level Agreement (SLA) Definitions
**Date**: January 15, 2026  
**Status**: Active

---

## 📊 SLA Targets

### 1. Service Uptime
- **Target**: >99.5% uptime
- **Measurement**: Percentage of time services are available
- **Services**: Airflow, Energy Service, Profile Service
- **Calculation**: `(sum(up) / count(up)) * 100`
- **Alert Threshold**: <99.5% for 15 minutes
- **Severity**: Critical

### 2. DAG Success Rate
- **Target**: >95% success rate
- **Measurement**: Percentage of successful DAG runs
- **Time Window**: 7 days rolling
- **Calculation**: `(successful_runs / total_runs) * 100`
- **Alert Threshold**: <95% for 1 hour
- **Severity**: Warning

### 3. DAG Completion Time
- **Target**: <2 hours (7200 seconds)
- **Measurement**: Average DAG execution duration
- **Time Window**: 7 days rolling
- **Calculation**: `AVG(end_date - start_date)`
- **Alert Threshold**: >2 hours for 5 minutes
- **Severity**: Warning

### 4. Data Freshness
- **Target**: <24 hours (86400 seconds)
- **Measurement**: Age of most recent data ingestion
- **Calculation**: `NOW() - MAX(ingestion_date)`
- **Alert Threshold**: >24 hours for 1 hour
- **Severity**: Warning

### 5. API Response Time
- **Target**: p95 <500ms (0.5 seconds)
- **Measurement**: 95th percentile response time
- **Endpoint**: Energy Service API
- **Calculation**: `histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))`
- **Alert Threshold**: >500ms for 5 minutes
- **Severity**: Warning

### 6. Query Performance
- **Target**: <5 seconds
- **Measurement**: Average query execution time
- **Scope**: Database queries
- **Alert Threshold**: >5 seconds for 5 minutes
- **Severity**: Warning

---

## 📈 SLA Monitoring

### Dashboard
- **Location**: Grafana - "SLA Monitoring" dashboard
- **Refresh**: 1 minute
- **Time Range**: 7 days default
- **URL**: `https://monitoring.lianel.se/d/sla-monitoring`

### Metrics Tracked
1. Service Uptime SLA (current and trend)
2. DAG Success Rate SLA (current and trend)
3. Average DAG Duration
4. Data Freshness
5. API Response Time (p95)
6. Query Performance
7. SLA Compliance by DAG (table)

---

## 🚨 SLA Alerts

### Configured Alerts
All SLA alerts are configured in `monitoring/prometheus/alerts/sla-alerts.yml`:

1. **APISLAViolation**: API response time >500ms
2. **DAGCompletionSLAViolation**: DAG duration >2 hours
3. **ServiceUptimeSLAViolation**: Uptime <99.5%
4. **DataFreshnessSLAViolation**: Data age >24 hours

### Alert Severity
- **Critical**: Service uptime violations
- **Warning**: Performance and data freshness violations

---

## 📝 SLA Reporting

### Weekly Reports
- Service uptime percentage
- DAG success rate
- Average DAG duration
- Data freshness status
- API performance metrics
- SLA violations summary

### Monthly Reviews
- SLA compliance trends
- Violation analysis
- Improvement recommendations
- Target adjustments if needed

---

## 🔧 SLA Maintenance

### Review Frequency
- **Weekly**: Review SLA compliance
- **Monthly**: Analyze trends and adjust targets
- **Quarterly**: Review and update SLA definitions

### Target Adjustments
SLA targets may be adjusted based on:
- System capabilities
- Business requirements
- Historical performance
- Infrastructure changes

---

## 📊 Current SLA Status

Check the SLA Monitoring dashboard for real-time status:
- Green: Meeting SLA
- Yellow: Approaching threshold
- Red: SLA violation

---

**Status**: SLA definitions active and monitored  
**Dashboard**: Available at `/d/sla-monitoring`  
**Alerts**: Configured and active
