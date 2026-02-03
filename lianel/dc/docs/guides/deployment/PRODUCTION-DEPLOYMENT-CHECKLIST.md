# Production Deployment Checklist
**Last Updated**: January 15, 2026

---

## Pre-Deployment Checklist

### Environment Setup
- [ ] Server meets minimum requirements (8GB RAM, 2 CPU cores, 100GB disk)
- [ ] Operating system is Ubuntu 20.04+ or Debian 11+
- [ ] Docker and Docker Compose are installed and up to date
- [ ] Firewall (UFW) is configured (ports 22, 80, 443 only)
- [ ] DNS records are configured and propagated
- [ ] SSL certificates are obtained and configured

### Security
- [ ] Security audit completed (run `scripts/security-audit.sh`)
- [ ] Vulnerability scan completed (run `scripts/vulnerability-scan.sh`)
- [ ] All security vulnerabilities addressed
- [ ] `.env` file is configured with strong passwords
- [ ] `.env` file is in `.gitignore` and not committed
- [ ] No hardcoded secrets in code
- [ ] SSL/TLS configuration is secure (TLS 1.2+ only)
- [ ] Security headers are configured in Nginx
- [ ] Rate limiting is configured
- [ ] Firewall rules are in place

### Configuration
- [ ] All environment variables are set in `.env`
- [ ] Database passwords are strong and unique
- [ ] Keycloak admin password is set
- [ ] OAuth2 client secrets are generated
- [ ] Airflow Fernet key is generated
- [ ] PostgreSQL databases are created
- [ ] Docker network is created

### Monitoring
- [ ] Monitoring stack is deployed
- [ ] Grafana dashboards are provisioned
- [ ] Prometheus alerts are configured
- [ ] Log aggregation is working (Loki)
- [ ] Health checks are configured

---

## Deployment Steps

### Step 1: Infrastructure Services
- [ ] Deploy Nginx and Keycloak
  ```bash
  docker-compose -f docker-compose.infra.yaml up -d
  ```
- [ ] Verify Keycloak is accessible
- [ ] Wait for Keycloak to be fully ready (30 seconds)

### Step 2: Authentication Setup
- [ ] Configure Keycloak realm
- [ ] Create OAuth2 clients
- [ ] Generate client secrets
- [ ] Update `.env` with client secrets
- [ ] Deploy OAuth2 Proxy
  ```bash
  docker-compose -f docker-compose.oauth2-proxy.yaml up -d
  ```

### Step 3: Application Services
- [ ] Deploy frontend
  ```bash
  docker-compose -f docker-compose.yaml up -d frontend
  ```
- [ ] Deploy backend services
  ```bash
  docker-compose -f docker-compose.yaml up -d
  ```
- [ ] Verify services are running
  ```bash
  docker ps
  ```

### Step 4: Data Pipeline
- [ ] Deploy Airflow
  ```bash
  docker-compose -f docker-compose.airflow.yaml up -d
  ```
- [ ] Wait for Airflow initialization
- [ ] Verify Airflow webserver is accessible
- [ ] Configure Airflow variables (e.g., ENTSOE_API_TOKEN)

### Step 5: Monitoring
- [ ] Deploy monitoring stack
  ```bash
  docker-compose -f docker-compose.monitoring.yaml up -d
  ```
- [ ] Verify Grafana is accessible
- [ ] Verify Prometheus is collecting metrics
- [ ] Verify Loki is collecting logs

---

## Post-Deployment Verification

### Service Health
- [ ] All containers are running
  ```bash
  docker ps
  ```
- [ ] No containers are restarting
- [ ] All services respond to health checks
  ```bash
  curl https://www.lianel.se/api/v1/health
  ```

### Endpoints
- [ ] Frontend is accessible: https://www.lianel.se
- [ ] API is accessible: https://www.lianel.se/api/v1/health
- [ ] Keycloak is accessible: https://auth.lianel.se
- [ ] Airflow is accessible: https://airflow.lianel.se
- [ ] Grafana is accessible: https://monitoring.lianel.se

### SSL/TLS
- [ ] All endpoints use HTTPS
- [ ] SSL certificates are valid
- [ ] No mixed content warnings
- [ ] HSTS header is present

### Authentication
- [ ] SSO login works
- [ ] API authentication works
- [ ] Token refresh works
- [ ] Logout works

### Functionality
- [ ] API endpoints return data
- [ ] Dashboards display data
- [ ] Airflow DAGs can be triggered
- [ ] Data pipelines run successfully

### Monitoring
- [ ] Grafana dashboards show data
- [ ] Prometheus metrics are collected
- [ ] Logs are aggregated in Loki
- [ ] Alerts are configured and working

### Performance
- [ ] API response times are acceptable (< 500ms p95)
- [ ] No high CPU usage
- [ ] No high memory usage
- [ ] No disk space issues

---

## Rollback Preparation

### Before Deployment
- [ ] Backup databases
  ```bash
  ./scripts/backup-database.sh
  ```
- [ ] Backup configuration files
  ```bash
  tar -czf backup-$(date +%d-%m-%Y).tar.gz .env docker-compose*.yaml
  ```
- [ ] Document current version/commit
  ```bash
  git log -1 > deployment-version.txt
  ```

### Rollback Triggers
- [ ] Define rollback criteria (e.g., > 5% error rate)
- [ ] Set up monitoring for rollback triggers
- [ ] Document rollback decision process

---

## Post-Deployment Tasks

### Immediate (First Hour)
- [ ] Monitor service logs for errors
- [ ] Check Grafana dashboards for anomalies
- [ ] Verify all critical endpoints
- [ ] Test authentication flows
- [ ] Review error rates

### Short-term (First Day)
- [ ] Monitor system performance
- [ ] Review access logs
- [ ] Check for security alerts
- [ ] Verify backup procedures
- [ ] Test disaster recovery procedures

### Ongoing
- [ ] Regular security audits
- [ ] Regular vulnerability scans
- [ ] Monitor system health
- [ ] Review and update documentation
- [ ] Plan for capacity scaling

---

## Emergency Contacts

- **On-call Engineer**: [Add contact]
- **Security Team**: [Add contact]
- **Database Administrator**: [Add contact]
- **Infrastructure Team**: [Add contact]

---

## Notes

- Document any issues encountered during deployment
- Record deployment duration
- Note any deviations from standard procedures
- Update this checklist based on lessons learned

---

**Status**: Active  
**Last Review**: January 15, 2026
