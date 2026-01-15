# Canary Release Guide
**Last Updated**: January 15, 2026

---

## Overview

Canary releases gradually roll out new versions to a small percentage of users, allowing for early detection of issues before full deployment.

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
    │  Stable (90%) │           │ Canary (10%) │
    │  Environment  │           │  Environment  │
    └───────────────┘           └────────────────┘
```

---

## Deployment Strategy

### Phase 1: Initial Canary (5%)

#### 1.1 Deploy Canary Environment
```bash
# Deploy canary version
docker-compose -f docker-compose.canary.yaml build
docker-compose -f docker-compose.canary.yaml up -d

# Verify canary is running
docker ps | grep canary
curl http://canary-service:3001/health
```

#### 1.2 Configure Traffic Splitting
```nginx
# Nginx configuration for 5% canary
upstream energy_service {
    # 95% to stable
    server lianel-energy-service-stable:3001 weight=95;
    # 5% to canary
    server lianel-energy-service-canary:3001 weight=5;
}
```

#### 1.3 Monitor Canary
```bash
# Monitor error rates
docker logs canary-service --tail 100 | grep -i error

# Monitor metrics
# Check Grafana for canary-specific metrics
# Compare error rates, latency, etc.
```

**Success Criteria**:
- [ ] Error rate < 1%
- [ ] Response time acceptable
- [ ] No critical issues
- [ ] Monitoring shows healthy metrics

**Duration**: 30 minutes to 1 hour

---

### Phase 2: Expand Canary (25%)

#### 2.1 Update Traffic Splitting
```nginx
# Increase canary traffic to 25%
upstream energy_service {
    # 75% to stable
    server lianel-energy-service-stable:3001 weight=75;
    # 25% to canary
    server lianel-energy-service-canary:3001 weight=25;
}

# Reload Nginx
docker exec nginx-proxy nginx -s reload
```

#### 2.2 Monitor Canary
- [ ] Monitor error rates
- [ ] Monitor response times
- [ ] Check for user-reported issues
- [ ] Compare metrics with stable

**Success Criteria**:
- [ ] Error rate < 1%
- [ ] Response time acceptable
- [ ] No user complaints
- [ ] Metrics comparable to stable

**Duration**: 1-2 hours

---

### Phase 3: Expand Canary (50%)

#### 3.1 Update Traffic Splitting
```nginx
# Increase canary traffic to 50%
upstream energy_service {
    # 50% to stable
    server lianel-energy-service-stable:3001 weight=50;
    # 50% to canary
    server lianel-energy-service-canary:3001 weight=50;
}

# Reload Nginx
docker exec nginx-proxy nginx -s reload
```

#### 3.2 Monitor Canary
- [ ] Monitor error rates
- [ ] Monitor response times
- [ ] Check for user-reported issues
- [ ] Compare metrics with stable

**Success Criteria**:
- [ ] Error rate < 1%
- [ ] Response time acceptable
- [ ] No user complaints
- [ ] Metrics comparable to stable

**Duration**: 2-4 hours

---

### Phase 4: Full Rollout (100%)

#### 4.1 Update Traffic Splitting
```nginx
# Route 100% to canary (now stable)
upstream energy_service {
    # 100% to canary
    server lianel-energy-service-canary:3001;
}

# Reload Nginx
docker exec nginx-proxy nginx -s reload
```

#### 4.2 Stop Old Environment
```bash
# Stop old stable environment
docker-compose -f docker-compose.stable.yaml down

# Rename canary to stable for next release
# (Or keep canary as new stable)
```

**Success Criteria**:
- [ ] All traffic on new version
- [ ] Error rate < 1%
- [ ] Response time acceptable
- [ ] No user complaints

---

## Rollback Procedures

### Immediate Rollback
```bash
# If issues detected, rollback immediately
# Route 100% to stable
upstream energy_service {
    server lianel-energy-service-stable:3001;
}

# Reload Nginx
docker exec nginx-proxy nginx -s reload

# Stop canary
docker-compose -f docker-compose.canary.yaml down
```

### Gradual Rollback
```bash
# Reduce canary traffic gradually
# 50% -> 25% -> 5% -> 0%
# Update Nginx configuration at each step
```

---

## Traffic Splitting Methods

### 1. Weight-Based (Round Robin)
```nginx
upstream energy_service {
    server lianel-energy-service-stable:3001 weight=90;
    server lianel-energy-service-canary:3001 weight=10;
}
```

### 2. IP-Based (Hash)
```nginx
upstream energy_service {
    ip_hash;
    server lianel-energy-service-stable:3001;
    server lianel-energy-service-canary:3001;
}
```

### 3. Header-Based
```nginx
map $http_x_canary $backend {
    default lianel-energy-service-stable:3001;
    "true" lianel-energy-service-canary:3001;
}

location /api/v1/ {
    proxy_pass http://$backend;
}
```

### 4. Cookie-Based
```nginx
map $cookie_canary $backend {
    default lianel-energy-service-stable:3001;
    "true" lianel-energy-service-canary:3001;
}

location /api/v1/ {
    proxy_pass http://$backend;
}
```

---

## Monitoring

### Key Metrics to Monitor

#### Error Rates
- **Target**: < 1% error rate
- **Action**: Rollback if error rate > 2%

#### Response Times
- **Target**: p95 < 500ms
- **Action**: Rollback if p95 > 1000ms

#### Throughput
- **Target**: Comparable to stable
- **Action**: Investigate if throughput drops > 10%

#### Resource Usage
- **Target**: Comparable to stable
- **Action**: Investigate if resource usage increases > 20%

---

## Automation Scripts

### Canary Deployment Script
```bash
#!/bin/bash
# canary-deploy.sh

CANARY_PERCENT=${1:-5}  # Default 5%

echo "Deploying canary at ${CANARY_PERCENT}%"

# Build canary
docker-compose -f docker-compose.canary.yaml build
docker-compose -f docker-compose.canary.yaml up -d

# Wait for health check
sleep 30
curl http://canary-service:3001/health

# Update Nginx configuration
STABLE_WEIGHT=$((100 - CANARY_PERCENT))
sed -i "s/weight=[0-9]*/weight=${STABLE_WEIGHT}/g" nginx/config/nginx.conf
sed -i "s/canary.*weight=[0-9]*/canary.*weight=${CANARY_PERCENT}/g" nginx/config/nginx.conf

# Reload Nginx
docker exec nginx-proxy nginx -s reload

echo "Canary deployed at ${CANARY_PERCENT}%"
```

### Canary Expansion Script
```bash
#!/bin/bash
# canary-expand.sh

NEW_PERCENT=${1:-25}  # Default 25%

echo "Expanding canary to ${NEW_PERCENT}%"

# Update Nginx configuration
STABLE_WEIGHT=$((100 - NEW_PERCENT))
sed -i "s/weight=[0-9]*/weight=${STABLE_WEIGHT}/g" nginx/config/nginx.conf
sed -i "s/canary.*weight=[0-9]*/canary.*weight=${NEW_PERCENT}/g" nginx/config/nginx.conf

# Reload Nginx
docker exec nginx-proxy nginx -s reload

echo "Canary expanded to ${NEW_PERCENT}%"
```

---

## Best Practices

### 1. Gradual Rollout
- Start with 5% traffic
- Increase gradually (5% -> 25% -> 50% -> 100%)
- Monitor at each stage
- Rollback if issues detected

### 2. Monitoring
- Monitor error rates continuously
- Compare metrics with stable
- Set up alerts for anomalies
- Review user feedback

### 3. Testing
- Run smoke tests before canary
- Test in staging environment first
- Verify database compatibility
- Test integration with other services

### 4. Rollback Readiness
- Have rollback plan ready
- Test rollback procedures
- Monitor for quick rollback
- Document rollback triggers

---

## Advantages

1. **Early Detection**: Issues detected with small user base
2. **Reduced Risk**: Limited impact if issues occur
3. **Gradual Rollout**: Smooth transition to new version
4. **User Feedback**: Early feedback from canary users

---

## Disadvantages

1. **Complexity**: More complex deployment process
2. **Monitoring**: Requires monitoring both versions
3. **Database**: Shared database or replication needed
4. **Configuration**: More complex configuration management

---

## When to Use

- **High-Risk Changes**: Significant changes or refactoring
- **New Features**: Major new features
- **Performance Changes**: Performance optimizations
- **User-Facing Changes**: Changes affecting user experience

---

## Success Criteria

- [ ] Canary deployed successfully
- [ ] Traffic splitting configured
- [ ] Monitoring in place
- [ ] Rollback procedures tested
- [ ] Gradual rollout completed
- [ ] No critical issues during rollout

---

**Status**: Active  
**Last Review**: January 15, 2026
