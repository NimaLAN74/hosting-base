# Blue-Green Deployment Guide
**Last Updated**: January 15, 2026

---

## Overview

Blue-green deployment maintains two identical production environments (blue and green). Only one environment is live at a time, allowing for zero-downtime deployments and instant rollback.

---

## Architecture

```
                    ┌─────────────┐
                    │   Nginx     │
                    │  Load Balancer│
                    └──────┬──────┘
                           │
            ┌──────────────┴──────────────┐
            │                             │
    ┌───────▼───────┐           ┌────────▼──────┐
    │  Blue (Live)  │           │ Green (Staging)│
    │  Environment  │           │  Environment   │
    └───────────────┘           └────────────────┘
```

---

## Prerequisites

- Two identical environments (blue and green)
- Shared database (or database replication)
- Load balancer (Nginx) to route traffic
- DNS/configuration management for switching

---

## Deployment Strategy

### Phase 1: Preparation

#### 1.1 Identify Current Environment
```bash
# Check which environment is currently live
curl -I https://www.lianel.se/api/v1/health | grep -i "x-environment"

# Or check Nginx configuration
grep -i "blue\|green" /root/lianel/dc/nginx/config/nginx.conf
```

#### 1.2 Prepare Staging Environment
```bash
# If blue is live, prepare green
# If green is live, prepare blue

# Checkout new version
cd /root/hosting-base
git pull origin master
git checkout <new-version-tag>

# Update configuration
cd lianel/dc
cp .env.blue .env  # or .env.green
# Edit .env with environment-specific values
```

---

### Phase 2: Deploy to Staging

#### 2.1 Deploy Services to Staging Environment
```bash
# Deploy to green (if blue is live)
docker-compose -f docker-compose.green.yaml build
docker-compose -f docker-compose.green.yaml up -d

# Or deploy to blue (if green is live)
docker-compose -f docker-compose.blue.yaml build
docker-compose -f docker-compose.blue.yaml up -d
```

#### 2.2 Verify Staging Environment
```bash
# Check service health
docker ps | grep green  # or blue

# Test endpoints directly (bypassing load balancer)
curl http://green-service:3001/health  # or blue-service

# Check logs
docker logs green-service --tail 50  # or blue-service
```

---

### Phase 3: Smoke Testing

#### 3.1 Run Smoke Tests
```bash
# Test critical endpoints
curl http://green-service:3001/api/v1/health
curl http://green-service:3001/api/v1/datasets/forecasting?limit=10

# Test authentication
curl -H "Authorization: Bearer $TOKEN" \
  http://green-service:3001/api/v1/datasets/forecasting?limit=10

# Test database connectivity
docker exec green-service psql -h localhost -U airflow -d airflow -c "SELECT 1;"
```

#### 3.2 Verify Functionality
- [ ] Health checks pass
- [ ] API endpoints respond
- [ ] Authentication works
- [ ] Database connectivity works
- [ ] No errors in logs

---

### Phase 4: Switch Traffic

#### 4.1 Update Load Balancer Configuration
```bash
# Edit Nginx configuration
nano /root/lianel/dc/nginx/config/nginx.conf

# Change upstream from blue to green (or vice versa)
# Before:
#   set $energy_service_upstream "http://blue-energy-service:3001";
# After:
#   set $energy_service_upstream "http://green-energy-service:3001";

# Test configuration
docker exec nginx-proxy nginx -t

# Reload Nginx (zero downtime)
docker exec nginx-proxy nginx -s reload
```

#### 4.2 Verify Traffic Routing
```bash
# Check which environment is receiving traffic
curl -I https://www.lianel.se/api/v1/health | grep -i "x-environment"

# Monitor logs
docker logs green-service -f  # or blue-service
```

---

### Phase 5: Monitoring

#### 5.1 Monitor New Environment
```bash
# Monitor service health
watch -n 1 'docker ps | grep green'  # or blue

# Monitor error rates
docker logs green-service --tail 100 | grep -i error

# Monitor metrics in Grafana
# Check for increased error rates, latency, etc.
```

#### 5.2 Monitor for Issues
- [ ] Error rates are normal
- [ ] Response times are acceptable
- [ ] No memory leaks
- [ ] No CPU spikes
- [ ] Database connections are stable

---

### Phase 6: Rollback (If Needed)

#### 6.1 Immediate Rollback
```bash
# If issues are detected, rollback immediately
# Revert Nginx configuration
nano /root/lianel/dc/nginx/config/nginx.conf

# Change upstream back to previous environment
# Before:
#   set $energy_service_upstream "http://green-energy-service:3001";
# After:
#   set $energy_service_upstream "http://blue-energy-service:3001";

# Reload Nginx
docker exec nginx-proxy nginx -s reload
```

#### 6.2 Verify Rollback
```bash
# Verify traffic is routed to previous environment
curl -I https://www.lianel.se/api/v1/health

# Monitor previous environment
docker logs blue-service -f  # or green-service
```

---

### Phase 7: Cleanup

#### 7.1 Keep Previous Environment (Optional)
```bash
# Keep previous environment running for quick rollback
# Stop after 24-48 hours if no issues
```

#### 7.2 Stop Previous Environment
```bash
# Stop previous environment after successful deployment
docker-compose -f docker-compose.blue.yaml down  # or green.yaml

# Or keep it for next deployment cycle
```

---

## Docker Compose Configuration

### Blue Environment (docker-compose.blue.yaml)
```yaml
services:
  lianel-energy-service-blue:
    image: lianel-energy-service:latest
    container_name: lianel-energy-service-blue
    networks:
      - lianel-network
    # ... other configuration
```

### Green Environment (docker-compose.green.yaml)
```yaml
services:
  lianel-energy-service-green:
    image: lianel-energy-service:latest
    container_name: lianel-energy-service-green
    networks:
      - lianel-network
    # ... other configuration
```

### Nginx Configuration
```nginx
# Upstream configuration
upstream energy_service {
    # Switch between blue and green
    server lianel-energy-service-blue:3001;
    # server lianel-energy-service-green:3001;
}

# Location block
location /api/v1/ {
    proxy_pass http://energy_service;
    # ... other configuration
}
```

---

## Automation Scripts

### Deployment Script
```bash
#!/bin/bash
# blue-green-deploy.sh

CURRENT_ENV=$(curl -s https://www.lianel.se/api/v1/health | jq -r '.environment')
NEW_ENV=$([ "$CURRENT_ENV" == "blue" ] && echo "green" || echo "blue")

echo "Current environment: $CURRENT_ENV"
echo "Deploying to: $NEW_ENV"

# Build and deploy
docker-compose -f docker-compose.${NEW_ENV}.yaml build
docker-compose -f docker-compose.${NEW_ENV}.yaml up -d

# Wait for health check
sleep 30
curl http://lianel-energy-service-${NEW_ENV}:3001/health

# Switch traffic
sed -i "s/blue-energy-service/${NEW_ENV}-energy-service/g" nginx/config/nginx.conf
docker exec nginx-proxy nginx -s reload

echo "Deployment complete. New environment: $NEW_ENV"
```

---

## Best Practices

### 1. Database Considerations
- **Shared Database**: Both environments use same database
- **Database Migrations**: Run migrations before switching
- **Data Consistency**: Ensure data is consistent

### 2. Configuration Management
- **Environment Variables**: Use separate .env files
- **Secrets**: Rotate secrets when switching
- **Feature Flags**: Use feature flags for gradual rollout

### 3. Monitoring
- **Health Checks**: Monitor both environments
- **Metrics**: Compare metrics between environments
- **Alerts**: Set up alerts for both environments

### 4. Testing
- **Smoke Tests**: Run smoke tests before switching
- **Integration Tests**: Verify integration with other services
- **Performance Tests**: Compare performance metrics

---

## Advantages

1. **Zero Downtime**: Switch traffic instantly
2. **Instant Rollback**: Revert by switching traffic
3. **Safe Testing**: Test new version in production-like environment
4. **Reduced Risk**: Previous version remains available

---

## Disadvantages

1. **Resource Usage**: Requires double resources
2. **Database Complexity**: Shared database or replication needed
3. **Configuration Management**: More complex configuration
4. **Cost**: Higher infrastructure costs

---

## When to Use

- **Critical Services**: Services requiring zero downtime
- **High Traffic**: Services with high traffic volumes
- **Frequent Deployments**: Services deployed frequently
- **Risk Aversion**: Services where rollback is critical

---

## Success Criteria

- [ ] Zero downtime during deployment
- [ ] Instant rollback capability
- [ ] New environment tested before switch
- [ ] Monitoring in place for both environments
- [ ] Rollback procedures tested

---

**Status**: Active  
**Last Review**: January 15, 2026
