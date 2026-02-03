# Frequently Asked Questions (FAQ)
**Last Updated**: January 15, 2026

---

## Table of Contents

1. [General Questions](#general-questions)
2. [API Questions](#api-questions)
3. [Authentication Questions](#authentication-questions)
4. [Dashboard Questions](#dashboard-questions)
5. [Data Questions](#data-questions)
6. [Deployment Questions](#deployment-questions)
7. [Troubleshooting Questions](#troubleshooting-questions)

---

## General Questions

### What is the Lianel platform?

The Lianel platform is an EU Energy & Geospatial Intelligence platform that provides:
- Energy consumption and renewable energy data
- Geospatial features and regional analysis
- Machine learning datasets for forecasting and clustering
- REST APIs for data access
- Interactive dashboards for visualization

### What services are included?

- **Frontend**: React web application
- **Backend APIs**: Energy and Profile services (Rust)
- **Authentication**: Keycloak (OAuth2/OIDC)
- **Data Pipelines**: Apache Airflow
- **Monitoring**: Grafana, Prometheus, Loki
- **Reverse Proxy**: Nginx

### How do I access the platform?

- **Frontend**: https://www.lianel.se
- **API**: https://www.lianel.se/api/v1
- **Airflow**: https://airflow.lianel.se
- **Grafana**: https://monitoring.lianel.se
- **Keycloak**: https://auth.lianel.se

### Is the platform free to use?

The platform is available for authorized users. Contact your administrator for access.

---

## API Questions

### How do I get an API token?

1. Register a client in Keycloak (contact administrator)
2. Use client credentials to get an access token
3. Include token in API requests as `Authorization: Bearer <token>`

See [Authentication Guide](./AUTHENTICATION-GUIDE.md) for details.

### How long do tokens last?

Access tokens expire after 5 minutes. Use refresh tokens or request new tokens as needed.

### What is the rate limit?

Rate limits are configured per client. Check response headers for rate limit information:
- `X-RateLimit-Remaining`: Remaining requests
- `X-RateLimit-Reset`: Reset time

### How do I handle pagination?

Use `limit` and `offset` parameters:
```bash
# First page
GET /api/v1/datasets/forecasting?limit=100&offset=0

# Second page
GET /api/v1/datasets/forecasting?limit=100&offset=100
```

### What data formats are supported?

- **Request**: JSON (Content-Type: application/json)
- **Response**: JSON

### How do I filter data?

Use query parameters:
```bash
# Filter by country and year
GET /api/v1/energy/annual?country_code=DE&year=2023

# Filter by date range
GET /api/v1/energy/annual?year_min=2020&year_max=2023
```

### Where can I find API documentation?

- **Swagger UI**: https://www.lianel.se/swagger-ui/
- **User Guide**: [API User Guide](./USER-GUIDE-API.md)
- **Examples**: [API Examples](./API-EXAMPLES.md)

---

## Authentication Questions

### How do I login?

Use SSO (Single Sign-On) via Keycloak:
1. Navigate to any protected resource
2. You'll be redirected to Keycloak login
3. Enter your credentials
4. You'll be redirected back after authentication

### I forgot my password. How do I reset it?

1. Go to https://auth.lianel.se
2. Click "Forgot Password"
3. Enter your email
4. Follow the reset link

### How do I create a new account?

Contact your administrator to create a new account in Keycloak.

### Why am I getting 401 Unauthorized?

Possible causes:
- Token expired (refresh or request new token)
- Invalid token (check token format)
- Missing token (include Authorization header)

See [Authentication Guide](./AUTHENTICATION-GUIDE.md) for details.

### Can I use API keys instead of OAuth2?

Currently, only OAuth2/OIDC authentication is supported. API keys are not available.

---

## Dashboard Questions

### How do I access dashboards?

1. Navigate to https://monitoring.lianel.se
2. Login with SSO
3. Browse available dashboards from the left menu

### What dashboards are available?

- **Energy Metrics**: Energy consumption and renewable percentages
- **Regional Comparison**: Compare countries and regions
- **System Health**: Monitor service status and resources
- **Pipeline Status**: Track Airflow DAG execution
- **Web Analytics**: Website traffic and access patterns
- **SLA Monitoring**: Service level agreement tracking
- **Data Quality**: Data freshness and completeness

### How do I customize dashboards?

1. Click on panel title → "Edit"
2. Modify queries, visualizations, or settings
3. Click "Save" to save changes

**Note**: Changes are saved to your user account. To make permanent changes, update dashboard JSON files.

### Why is a dashboard showing no data?

Possible causes:
- Time range has no data
- Data source is unavailable
- Query filters are too restrictive
- Data pipeline hasn't run

**Solutions**:
- Adjust time range
- Check data source status
- Review query filters
- Verify pipeline execution

### How do I export dashboard data?

1. Click panel title → "Share"
2. Select "Export CSV" or "Export PNG"
3. Download the file

---

## Data Questions

### What data sources are used?

- **Eurostat**: Annual energy statistics
- **ENTSO-E**: Electricity generation and consumption (hourly/15-min)
- **OpenStreetMap**: Geospatial features (power plants, renewable installations)
- **NUTS**: Regional boundaries and metadata

### How often is data updated?

- **Eurostat**: Annual (when available)
- **ENTSO-E**: Hourly/15-minute (real-time)
- **OSM**: Weekly (scheduled DAG)
- **NUTS**: Static (updated periodically)

### Why is data missing for some countries/regions?

Possible reasons:
- Data not available from source
- Ingestion pipeline hasn't run
- Data quality issues
- Filter too restrictive

**Solutions**:
- Check data source availability
- Verify pipeline execution
- Review data quality alerts
- Adjust query filters

### How do I interpret energy data?

See [Data Interpretation Guide](./DATA-INTERPRETATION-GUIDE.md) for detailed explanations.

### What units are used?

- **Energy**: GWh (Gigawatt-hours) or TJ (Terajoules)
- **Percentage**: % (0-100)
- **Area**: km²
- **Density**: GWh/km² or TJ/km²

### How accurate is the data?

Data accuracy depends on the source:
- **Eurostat**: Official statistics (high accuracy)
- **ENTSO-E**: Real-time grid data (high accuracy)
- **OSM**: Community-sourced (variable accuracy)

Always verify critical data with official sources.

---

## Deployment Questions

### What are the system requirements?

- **OS**: Ubuntu 20.04+ or Debian 11+
- **RAM**: 8 GB minimum (16 GB recommended)
- **CPU**: 2 cores minimum (4 cores recommended)
- **Disk**: 100 GB minimum
- **Network**: Public IP address

### How do I deploy the platform?

See [Deployment Guide](./DEPLOYMENT-GUIDE-SIMPLE.md) for step-by-step instructions.

### How do I update services?

```bash
# Pull latest code
git pull

# Rebuild and restart
docker-compose -f docker-compose.yaml build
docker-compose -f docker-compose.yaml up -d
```

### How do I backup data?

```bash
# Backup databases
./scripts/backup-database.sh

# Backup configuration
tar -czf backup-$(date +%d-%m-%Y).tar.gz .env docker-compose*.yaml
```

### How do I restore from backup?

1. Restore database backup
2. Restore configuration files
3. Restart services

See [Backup and Recovery Runbook](./runbooks/BACKUP-RECOVERY.md) for details.

---

## Troubleshooting Questions

### A service is not starting. What should I do?

1. Check service logs: `docker logs <service-name>`
2. Verify environment variables
3. Check resource limits
4. Review [Troubleshooting Guide](./TROUBLESHOOTING-GUIDE.md)

### I'm getting 502 Bad Gateway. What's wrong?

Possible causes:
- Upstream service is down
- Nginx configuration error
- Network connectivity issue

**Solutions**:
- Check upstream service status
- Review Nginx logs
- Verify network configuration

### The API is slow. How can I improve performance?

1. Use pagination for large datasets
2. Filter data server-side
3. Cache responses when possible
4. Review [API Best Practices](./API-BEST-PRACTICES.md)

### DAGs are failing. What should I do?

1. Check DAG logs in Airflow UI
2. Review task error messages
3. Check for OOM kills
4. Follow [DAG Failure Recovery Runbook](./runbooks/DAG-FAILURE-RECOVERY.md)

### How do I check system health?

1. Access Grafana: https://monitoring.lianel.se
2. Open "System Health" dashboard
3. Review service status, CPU, memory, and disk usage

### Where can I find more help?

- **Documentation**: `/lianel/dc/*.md`
- **Runbooks**: `/lianel/dc/runbooks/`
- **Troubleshooting Guide**: [Troubleshooting Guide](./TROUBLESHOOTING-GUIDE.md)
- **Monitoring**: https://monitoring.lianel.se

---

## Additional Resources

### Documentation

- [API User Guide](./USER-GUIDE-API.md)
- [API Examples](./API-EXAMPLES.md)
- [API Best Practices](./API-BEST-PRACTICES.md)
- [Authentication Guide](./AUTHENTICATION-GUIDE.md)
- [Dashboard Tutorials](./DASHBOARD-TUTORIALS.md)
- [Data Interpretation Guide](./DATA-INTERPRETATION-GUIDE.md)
- [Deployment Guide](./DEPLOYMENT-GUIDE-SIMPLE.md)
- [Troubleshooting Guide](./TROUBLESHOOTING-GUIDE.md)

### Runbooks

- [DAG Failure Recovery](./runbooks/DAG-FAILURE-RECOVERY.md)
- [Service Restart Procedures](./runbooks/SERVICE-RESTART-PROCEDURES.md)
- [Database Maintenance](./runbooks/DATABASE-MAINTENANCE.md)
- [Data Quality Issues](./runbooks/DATA-QUALITY-ISSUES.md)
- [Performance Troubleshooting](./runbooks/PERFORMANCE-TROUBLESHOOTING.md)
- [Backup and Recovery](./runbooks/BACKUP-RECOVERY.md)

---

**Status**: Active  
**Last Review**: January 15, 2026
