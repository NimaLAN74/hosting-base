# Troubleshooting Guide
**Last Updated**: January 15, 2026

---

## Table of Contents

1. [Quick Diagnostics](#quick-diagnostics)
2. [Service Issues](#service-issues)
3. [API Issues](#api-issues)
4. [Database Issues](#database-issues)
5. [Authentication Issues](#authentication-issues)
6. [Performance Issues](#performance-issues)
7. [Data Pipeline Issues](#data-pipeline-issues)

---

## Quick Diagnostics

### Check All Services

```bash
# List all containers
docker ps -a

# Check service status
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

# Check resource usage
docker stats --no-stream
```

### Check Logs

```bash
# View recent logs
docker logs <container-name> --tail 100

# Follow logs
docker logs <container-name> -f

# Check all logs for errors
docker ps -q | xargs -I {} docker logs {} 2>&1 | grep -i error
```

### Check Network

```bash
# Test connectivity
curl -I https://www.lianel.se
curl -I https://auth.lianel.se
curl -I https://airflow.lianel.se

# Check DNS
nslookup www.lianel.se
nslookup auth.lianel.se
```

---

## Service Issues

### Service Not Starting

**Symptoms**: Container exits immediately or shows "unhealthy"

**Diagnosis**:
```bash
# Check container status
docker ps -a | grep <service-name>

# Check logs
docker logs <service-name>

# Check resource limits
docker stats <service-name>
```

**Common Causes**:
1. **Missing environment variables**
   ```bash
   # Check .env file
   cat .env | grep <VARIABLE_NAME>
   ```

2. **Port conflicts**
   ```bash
   # Check port usage
   sudo netstat -tulpn | grep :80
   sudo netstat -tulpn | grep :443
   ```

3. **Resource limits**
   ```bash
   # Check system resources
   free -h
   df -h
   ```

**Solutions**:
- Verify all required environment variables are set
- Check for port conflicts and resolve
- Increase resource limits if needed
- Review service logs for specific errors

---

### Service Restarting Continuously

**Symptoms**: Container status shows "Restarting" repeatedly

**Diagnosis**:
```bash
# Check restart count
docker ps -a | grep <service-name>

# Check logs for crash reasons
docker logs <service-name> --tail 50
```

**Common Causes**:
1. **Database connection failure**
   - Check database is running
   - Verify connection credentials
   - Check network connectivity

2. **Configuration errors**
   - Verify configuration files
   - Check for syntax errors
   - Validate environment variables

3. **Out of memory (OOM)**
   - Check memory usage
   - Review resource limits
   - Increase memory if needed

**Solutions**:
- Fix database connection issues
- Correct configuration errors
- Increase memory limits
- Review runbooks for specific services

---

## API Issues

### API Returns 404

**Symptoms**: API endpoint returns "404 Not Found"

**Diagnosis**:
```bash
# Test endpoint
curl -v https://www.lianel.se/api/v1/health

# Check Nginx routing
docker logs nginx-proxy | grep -i "404"

# Verify service is running
docker ps | grep energy-service
```

**Common Causes**:
1. **Incorrect URL path**
   - Verify endpoint path
   - Check API documentation

2. **Nginx routing issue**
   - Check Nginx configuration
   - Verify location blocks

3. **Service not running**
   - Check container status
   - Verify service health

**Solutions**:
- Verify correct API endpoint
- Check Nginx configuration
- Restart service if needed
- Review API documentation

---

### API Returns 401 Unauthorized

**Symptoms**: API returns "401 Unauthorized"

**Diagnosis**:
```bash
# Test without token
curl https://www.lianel.se/api/v1/datasets/forecasting

# Test with token
curl -H "Authorization: Bearer $TOKEN" \
  https://www.lianel.se/api/v1/datasets/forecasting
```

**Common Causes**:
1. **Missing or invalid token**
   - Verify token is included
   - Check token expiration
   - Validate token format

2. **Token expired**
   - Refresh token
   - Request new token

3. **Authentication service down**
   - Check Keycloak status
   - Verify OAuth2 Proxy

**Solutions**:
- Include valid Bearer token
- Refresh expired tokens
- Verify authentication services
- Check token format (Bearer <token>)

---

### API Returns 500 Internal Server Error

**Symptoms**: API returns "500 Internal Server Error"

**Diagnosis**:
```bash
# Check service logs
docker logs lianel-energy-service --tail 100

# Check database connection
docker logs lianel-energy-service | grep -i "database\|connection"

# Check for errors
docker logs lianel-energy-service | grep -i "error\|exception"
```

**Common Causes**:
1. **Database connection failure**
   - Check database is running
   - Verify connection credentials
   - Check network connectivity

2. **Application errors**
   - Review application logs
   - Check for exceptions
   - Verify data format

3. **Resource exhaustion**
   - Check memory usage
   - Review CPU usage
   - Check disk space

**Solutions**:
- Fix database connection
- Review application logs
- Increase resources if needed
- Restart service

---

## Database Issues

### Connection Refused

**Symptoms**: Services cannot connect to database

**Diagnosis**:
```bash
# Check PostgreSQL status
sudo systemctl status postgresql

# Test connection
psql -h localhost -U airflow -d airflow

# Check port
sudo netstat -tulpn | grep :5432
```

**Common Causes**:
1. **PostgreSQL not running**
   - Start PostgreSQL service
   - Check service status

2. **Connection credentials incorrect**
   - Verify username/password
   - Check .env file

3. **Network issues**
   - Check firewall rules
   - Verify network configuration

**Solutions**:
- Start PostgreSQL: `sudo systemctl start postgresql`
- Verify credentials in .env file
- Check firewall rules
- Review network configuration

---

### Database Performance Issues

**Symptoms**: Slow queries, timeouts

**Diagnosis**:
```bash
# Check active connections
psql -h localhost -U airflow -d airflow -c "SELECT count(*) FROM pg_stat_activity;"

# Check slow queries
psql -h localhost -U airflow -d airflow -c "SELECT * FROM pg_stat_activity WHERE state = 'active';"

# Check table sizes
psql -h localhost -U airflow -d airflow -c "SELECT schemaname, tablename, pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size FROM pg_tables ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC LIMIT 10;"
```

**Common Causes**:
1. **Missing indexes**
   - Check index usage
   - Review query plans

2. **Too many connections**
   - Check connection pool settings
   - Review connection limits

3. **Large tables**
   - Consider partitioning
   - Review data retention

**Solutions**:
- Add missing indexes
- Optimize queries
- Adjust connection pool
- Consider table partitioning

---

## Authentication Issues

### Cannot Login to Keycloak

**Symptoms**: Login page shows errors or redirects

**Diagnosis**:
```bash
# Check Keycloak status
docker logs keycloak --tail 100

# Test Keycloak endpoint
curl -I https://auth.lianel.se

# Check database connection
docker logs keycloak | grep -i "database\|connection"
```

**Common Causes**:
1. **Keycloak not running**
   - Check container status
   - Review logs

2. **Database connection failure**
   - Verify database is accessible
   - Check credentials

3. **Configuration errors**
   - Verify environment variables
   - Check configuration files

**Solutions**:
- Restart Keycloak: `docker restart keycloak`
- Verify database connection
- Check configuration
- Review Keycloak logs

---

### SSO Not Working

**Symptoms**: Cannot access protected resources, redirect loops

**Diagnosis**:
```bash
# Check OAuth2 Proxy
docker logs oauth2-proxy --tail 100

# Test OAuth2 Proxy
curl -I https://www.lianel.se

# Check Keycloak clients
# Access Keycloak admin console
```

**Common Causes**:
1. **OAuth2 Proxy misconfiguration**
   - Verify client secret
   - Check redirect URIs

2. **Keycloak client issues**
   - Verify client configuration
   - Check redirect URIs match

3. **Cookie issues**
   - Check cookie domain
   - Verify cookie settings

**Solutions**:
- Verify OAuth2 Proxy configuration
- Check Keycloak client settings
- Review cookie configuration
- Restart OAuth2 Proxy

---

## Performance Issues

### High CPU Usage

**Symptoms**: System slow, high CPU usage

**Diagnosis**:
```bash
# Check CPU usage
docker stats --no-stream

# Check system load
top
htop

# Identify high CPU containers
docker stats --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}"
```

**Common Causes**:
1. **Resource limits too high**
   - Review container limits
   - Adjust CPU limits

2. **Inefficient queries**
   - Review database queries
   - Optimize slow queries

3. **Too many processes**
   - Check process count
   - Review concurrency settings

**Solutions**:
- Adjust resource limits
- Optimize queries
- Reduce concurrency
- Scale services if needed

---

### High Memory Usage

**Symptoms**: OOM kills, system slow

**Diagnosis**:
```bash
# Check memory usage
docker stats --no-stream

# Check system memory
free -h

# Check for OOM kills
dmesg | grep -i "out of memory"
journalctl -k | grep -i "oom"
```

**Common Causes**:
1. **Memory leaks**
   - Monitor memory over time
   - Review application code

2. **Too many concurrent processes**
   - Reduce concurrency
   - Adjust worker settings

3. **Insufficient memory limits**
   - Increase container limits
   - Add system memory

**Solutions**:
- Fix memory leaks
- Reduce concurrency
- Increase memory limits
- Add system memory

---

## Data Pipeline Issues

### DAG Not Running

**Symptoms**: DAG shows as "None" or not scheduled

**Diagnosis**:
```bash
# Check Airflow scheduler
docker logs airflow-scheduler --tail 100

# Check DAG status
# Access Airflow UI: https://airflow.lianel.se

# Check for errors
docker logs airflow-scheduler | grep -i "error\|exception"
```

**Common Causes**:
1. **DAG syntax errors**
   - Check DAG file syntax
   - Review Airflow logs

2. **Missing dependencies**
   - Verify Python packages
   - Check import errors

3. **Scheduler issues**
   - Check scheduler status
   - Review scheduler logs

**Solutions**:
- Fix DAG syntax errors
- Install missing dependencies
- Restart scheduler
- Review DAG configuration

---

### DAG Failing

**Symptoms**: DAG tasks show as "Failed"

**Diagnosis**:
```bash
# Check task logs
# Access Airflow UI → DAG → Task → Logs

# Check for OOM kills
docker logs airflow-worker | grep -i "oom\|killed"

# Check resource usage
docker stats airflow-worker
```

**Common Causes**:
1. **Out of memory (OOM)**
   - Check memory usage
   - Increase memory limits

2. **Task errors**
   - Review task logs
   - Check for exceptions

3. **External API failures**
   - Check API availability
   - Review rate limits

**Solutions**:
- Increase memory limits
- Fix task errors
- Handle API failures
- Review runbooks

---

## Getting Help

### Information to Collect

When reporting issues, include:
1. **Service logs**: `docker logs <service-name>`
2. **System status**: `docker ps -a`
3. **Resource usage**: `docker stats`
4. **Error messages**: Full error text
5. **Steps to reproduce**: What you did before the issue

### Resources

- **Runbooks**: `/lianel/dc/runbooks/`
- **Documentation**: `/lianel/dc/*.md`
- **Monitoring**: `https://monitoring.lianel.se`
- **Airflow**: `https://airflow.lianel.se`

---

**Status**: Active  
**Last Review**: January 15, 2026
