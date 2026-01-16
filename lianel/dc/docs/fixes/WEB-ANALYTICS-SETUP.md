# Web Analytics Setup Guide

This guide explains how to set up web analytics monitoring for site visits and access patterns using Nginx logs, Promtail, Loki, and Grafana.

## Overview

The web analytics system collects and visualizes:
- **Site visits**: Requests per second, unique visitors
- **Access patterns**: Top pages, HTTP methods, status codes
- **Performance**: Response times, data transfer
- **User information**: User agents, referrers, IP addresses
- **Errors**: Error rates, failed requests

## Architecture

```
Nginx → Docker Logs (stdout) → Promtail → Loki → Grafana Dashboard
```

## Components

### 1. Nginx JSON Log Format
- **Location**: `nginx/config/nginx.conf`
- **Format**: `json_combined` - Structured JSON logs with all request details
- **Output**: Docker logs (stdout/stderr)

### 2. Promtail Log Parser
- **Location**: `monitoring/promtail/config.yml`
- **Function**: Parses nginx JSON logs and extracts labels
- **Labels extracted**:
  - `status_code`: HTTP status code
  - `http_method`: GET, POST, etc.
  - `hostname`: Domain name
  - `client_ip`: Client IP address
  - `path`: Request path (without query params)

### 3. Grafana Dashboard
- **Location**: `monitoring/grafana/provisioning/dashboards/web-analytics.json`
- **Name**: "Web Analytics - Site Visits & Access"
- **Access**: https://www.lianel.se/monitoring/ → Dashboards → Web Analytics

## Deployment Steps

### Step 1: Update Nginx Configuration

The nginx configuration has been updated with JSON log format. Deploy it:

```bash
ssh root@72.60.80.84
cd /root/lianel/dc
git pull
docker exec nginx-proxy nginx -t  # Test config
docker exec nginx-proxy nginx -s reload  # Reload nginx
```

### Step 2: Update Promtail Configuration

Promtail config has been updated to parse nginx logs:

```bash
cd /root/lianel/dc
docker-compose -f docker-compose.monitoring.yaml restart promtail
docker logs promtail --tail 20  # Check for errors
```

### Step 3: Verify Dashboard in Grafana

1. Go to: https://www.lianel.se/monitoring/
2. Login with SSO
3. Navigate to: **Dashboards** → **Web Analytics - Site Visits & Access**
4. Dashboard should auto-load

## Dashboard Panels

### 1. Requests per Second
- Real-time request rate
- Shows traffic patterns over time

### 2. HTTP Status Codes
- Pie chart of status codes (200, 302, 404, 500, etc.)
- Helps identify errors and redirects

### 3. Top Pages
- Table of most accessed pages
- Shows popular content

### 4. Response Time (p95/p50)
- 95th and 50th percentile response times
- Performance monitoring

### 5. Unique Visitors (by IP)
- Count of unique IP addresses
- Last hour window

### 6. Total Requests (1h)
- Total request count
- Last hour window

### 7. Data Transferred
- Total bytes sent to clients
- Bandwidth usage

### 8. Error Rate
- Percentage of 5xx errors
- Health indicator

### 9. Requests by Host
- Breakdown by domain (lianel.se, auth.lianel.se, etc.)
- Shows which services are accessed most

### 10. Top User Agents
- Most common browsers/clients
- Helps understand user base

### 11. Recent Access Logs
- Live log viewer
- Filterable and searchable

## LogQL Queries

You can create custom queries in Grafana Explore:

### Requests per minute by path
```logql
sum by (path) (count_over_time({container="nginx-proxy"} | json | path!="" | __error__="" [1m]))
```

### Error rate by status code
```logql
sum by (status_code) (count_over_time({container="nginx-proxy"} | json | status_code=~"4..|5.." | __error__="" [5m]))
```

### Top IP addresses
```logql
topk(10, sum by (client_ip) (count_over_time({container="nginx-proxy"} | json | client_ip!="" | __error__="" [1h])))
```

### Average response time
```logql
avg_over_time({container="nginx-proxy"} | json | unwrap request_time | __error__="" [5m])
```

## Troubleshooting

### Dashboard shows no data

1. **Check Promtail is collecting logs**:
   ```bash
   docker logs promtail --tail 50 | grep nginx-proxy
   ```

2. **Check Loki has logs**:
   ```bash
   # In Grafana Explore, query:
   {container="nginx-proxy"}
   ```

3. **Verify nginx is logging**:
   ```bash
   docker logs nginx-proxy --tail 20
   ```

4. **Check Promtail config**:
   ```bash
   docker exec promtail cat /etc/promtail/config.yml
   ```

### JSON parsing errors

- Nginx logs might be in standard format instead of JSON
- Check nginx config: `docker exec nginx-proxy nginx -T | grep log_format`
- Restart nginx: `docker exec nginx-proxy nginx -s reload`

### Missing labels

- Promtail pipeline might not be parsing correctly
- Check Promtail logs: `docker logs promtail`
- Verify labels in Grafana Explore: `{container="nginx-proxy"} | json`

## Advanced Configuration

### Add Custom Metrics

You can add Prometheus metrics from logs:

```yaml
# In promtail config.yml
- metrics:
    nginx_requests_total:
      type: Counter
      description: "Total nginx requests"
      source: status
```

### Filter Specific Paths

Exclude monitoring endpoints from analytics:

```logql
{container="nginx-proxy"} | json | path!~"/monitoring.*" | path!~"/oauth2.*"
```

### Geographic Analysis

Add GeoIP lookup (requires additional setup):

```yaml
# In promtail config.yml
- template:
    source: geoip_country
    template: '{{ .remote_addr | geoip }}'
```

## Performance Considerations

- **Log volume**: Nginx logs can be high-volume
- **Retention**: Configure Loki retention in `monitoring/loki/local-config.yaml`
- **Storage**: Monitor Loki disk usage
- **Query performance**: Use time ranges appropriately

## Security & Privacy

- **IP addresses**: Consider anonymizing IPs for privacy compliance
- **User agents**: May contain sensitive information
- **Log retention**: Follow data retention policies
- **Access control**: Dashboard is protected by SSO

## Next Steps

- Set up alerts for high error rates
- Add geographic visualization (if GeoIP configured)
- Create custom dashboards for specific use cases
- Export analytics data for reporting

