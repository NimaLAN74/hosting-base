# Technical Specifications

## Server Infrastructure

### VPS Specifications
- **Provider**: [Your VPS Provider]
- **IP Address**: 72.60.80.84
- **Hostname**: lianel.se
- **Operating System**: Ubuntu/Debian Linux
- **Kernel**: 6.8.0-88-generic
- **CPU**: 2 cores (AMD)
- **Memory**: 8 GB RAM
- **Storage**: ~100 GB
- **Network**: 1 Gbps

### Docker Environment
- **Docker Version**: Latest stable
- **Docker API Version**: 1.52
- **Storage Driver**: overlayfs
- **Cgroup Driver**: systemd
- **Network Driver**: bridge

## Service Specifications

### Nginx Reverse Proxy
```yaml
Image: nginx:latest
Ports:
  - 80:80 (HTTP)
  - 443:443 (HTTPS)
Volumes:
  - ./nginx/config:/etc/nginx
  - ./nginx/ssl:/etc/letsencrypt
  - ./nginx/html:/usr/share/nginx/html
Configuration:
  - SSL/TLS termination
  - Reverse proxy for all services
  - Rate limiting (10 req/s general, 5 req/m login)
  - Proxy buffer size: 128k-256k
```

### Keycloak (SSO Provider)
```yaml
Image: quay.io/keycloak/keycloak:latest
Port: 8080 (internal)
Database: PostgreSQL (host)
Environment:
  - KC_DB: postgres
  - KC_DB_URL: jdbc:postgresql://host.docker.internal:5432/keycloak
  - KC_HOSTNAME: auth.lianel.se
  - KC_PROXY_HEADERS: xforwarded
  - KC_HTTP_ENABLED: true
Realm: lianel
Features:
  - OIDC authentication
  - User management
  - Client applications (oauth2-proxy, grafana)
```

### OAuth2 Proxy
```yaml
Image: quay.io/oauth2-proxy/oauth2-proxy:latest
Port: 4180 (internal)
Provider: keycloak-oidc
Configuration:
  - OIDC Issuer: https://auth.lianel.se/realms/lianel
  - Cookie Domain: .lianel.se
  - Cookie Secure: true
  - Cookie SameSite: lax
  - PKCE: S256
  - Session Duration: 4 hours
  - Refresh Interval: 1 hour
```

### React Frontend
```yaml
Image: lianel-frontend:latest
Base: Node.js 18 (build) + Nginx (runtime)
Port: 80 (internal)
Build:
  - Framework: React 18
  - Build tool: Create React App
  - Output: Static files
Runtime:
  - Nginx serving static files
  - Gzip compression enabled
```

### Apache Airflow
```yaml
Image: apache/airflow:3.1.3
Components:
  - Webserver (port 8080)
  - Scheduler
  - Worker (CeleryExecutor)
  - Triggerer
  - DAG Processor
Database: PostgreSQL (host)
Executor: CeleryExecutor
Broker: Redis
Configuration:
  - AIRFLOW_UID: 50000
  - Load Examples: false
  - Fernet Key: Encrypted
Volumes:
  - ./dags:/opt/airflow/dags
  - ./logs:/opt/airflow/logs
  - ./plugins:/opt/airflow/plugins
```

### Grafana
```yaml
Image: grafana/grafana:11.3.0
Port: 3000 (internal)
Configuration:
  - Root URL: https://www.lianel.se/monitoring
  - Serve from sub-path: true
  - Auth Proxy: enabled
  - Auth Headers: X-Email, X-User
  - Auto sign-up: true
  - Default role: Viewer
Data Sources:
  - Prometheus
  - Loki
Dashboards:
  - Docker Container Monitoring
  - System Overview
  - Docker Overview
```

### Prometheus
```yaml
Image: prom/prometheus:v2.54.1
Port: 9090 (internal)
Configuration:
  - Scrape interval: 15s
  - Evaluation interval: 15s
  - Retention: 15 days
  - External URL: https://www.lianel.se/prometheus
Scrape Targets:
  - prometheus:9090 (self)
  - node-exporter:9100
  - cadvisor:8080
  - host.docker.internal:9323 (Docker daemon)
Recording Rules:
  - container_cpu_usage:rate5m
  - container_memory:bytes
  - container_network_receive:rate5m
  - container_network_transmit:rate5m
```

### Loki
```yaml
Image: grafana/loki:3.0.0
Port: 3100 (internal)
Configuration:
  - Config: /etc/loki/local-config.yaml
Storage:
  - Volume: loki-data
Features:
  - Log aggregation
  - Label-based indexing
  - LogQL query language
```

### Promtail
```yaml
Image: grafana/promtail:3.0.0
Configuration:
  - Config: /etc/promtail/config.yml
Volumes:
  - /var/lib/docker/containers:/var/lib/docker/containers:ro
  - /var/run/docker.sock:/var/run/docker.sock
Features:
  - Docker container log collection
  - Label extraction
  - Push to Loki
```

### cAdvisor
```yaml
Image: gcr.io/cadvisor/cadvisor:v0.47.0
Port: 8080 (internal)
Privileged: true
Volumes:
  - /:/rootfs:ro
  - /var/run:/var/run:ro
  - /sys:/sys:ro
  - /var/lib/docker:/var/lib/docker:ro
  - /dev/disk:/dev/disk:ro
Metrics:
  - Container CPU usage
  - Container memory usage
  - Container filesystem I/O
  - Limited network metrics (host-level only)
Limitations:
  - With systemd cgroups: no per-container network metrics
  - Container names not exposed (only systemd paths)
```

### Node Exporter
```yaml
Image: prom/node-exporter:v1.8.2
Port: 9100 (internal)
Volumes:
  - /:/host:ro,rslave
Metrics:
  - CPU usage
  - Memory usage
  - Disk I/O
  - Network I/O
  - Filesystem usage
  - System load
```

### PostgreSQL (Host-level)
```yaml
Location: Host system
Port: 5432
Databases:
  - airflow (user: airflow)
  - keycloak (user: keycloak)
Access: host.docker.internal from containers
```

### Redis
```yaml
Image: redis:latest
Port: 6379 (internal)
Purpose: Airflow CeleryExecutor message broker
Configuration:
  - Password protected
  - Persistence enabled
```

## Network Configuration

### Docker Network
```yaml
Name: lianel-network
Driver: bridge
Subnet: 172.18.0.0/16
Gateway: 172.18.0.1
DNS: Docker embedded DNS
```

### Port Mapping
| Service | Internal Port | External Port | Protocol |
|---------|--------------|---------------|----------|
| Nginx | 80, 443 | 80, 443 | HTTP/HTTPS |
| All others | Various | None (internal only) | HTTP |

### SSL/TLS Configuration
```yaml
Provider: Let's Encrypt
Certificates:
  - www.lianel.se (SAN: lianel.se)
  - auth.lianel.se
  - airflow.lianel.se
  - monitoring.lianel.se
Protocol: TLSv1.2, TLSv1.3
Cipher Suites: Mozilla Intermediate configuration
HSTS: max-age=31536000; includeSubDomains
```

## Storage Configuration

### Docker Volumes
```yaml
Volumes:
  - loki-data: Loki log storage
  - prometheus-data: Prometheus metrics storage
  - grafana-data: Grafana dashboards and settings
  - postgres-data: PostgreSQL databases (if containerized)
  - redis-data: Redis persistence
```

### Host Mounts
```yaml
Mounts:
  - ./nginx/config → /etc/nginx
  - ./nginx/ssl → /etc/letsencrypt
  - ./monitoring/prometheus → /etc/prometheus
  - ./monitoring/grafana/provisioning → /etc/grafana/provisioning
  - ./dags → /opt/airflow/dags
  - /var/run/docker.sock → /var/run/docker.sock (monitoring)
```

## Environment Variables

### Critical Secrets
```bash
# Airflow
AIRFLOW__CORE__FERNET_KEY=<encrypted>
_AIRFLOW_WWW_USER_PASSWORD=<hashed>
POSTGRES_PASSWORD=<secret>

# Keycloak
KEYCLOAK_ADMIN_PASSWORD=<secret>
KEYCLOAK_DB_PASSWORD=<secret>

# OAuth2 Proxy
OAUTH2_CLIENT_SECRET=<secret>
OAUTH2_COOKIE_SECRET=<secret>

# Grafana
GRAFANA_ADMIN_PASSWORD=<secret>
GRAFANA_OAUTH_CLIENT_SECRET=<secret>

# Redis
REDIS_PASSWORD=<secret>
```

## Performance Characteristics

### Resource Usage (Typical)
| Service | CPU | Memory | Disk I/O |
|---------|-----|--------|----------|
| Nginx | <5% | ~10 MB | Low |
| Keycloak | 5-10% | ~400 MB | Low |
| OAuth2 Proxy | <1% | ~20 MB | Low |
| Frontend | <1% | ~10 MB | Low |
| Airflow (all) | 10-30% | ~2 GB | Medium |
| Grafana | 2-5% | ~100 MB | Low |
| Prometheus | 5-10% | ~500 MB | Medium |
| Loki | 2-5% | ~100 MB | Medium |
| cAdvisor | 2-5% | ~50 MB | Low |
| Node Exporter | <1% | ~20 MB | Low |

### System Capacity
- **Total Memory Usage**: ~4.5 GB / 8 GB (55%)
- **Total CPU Usage**: ~25-30% average
- **Disk Usage**: ~20 GB / 100 GB
- **Network**: <10 Mbps typical

## Monitoring Metrics

### Prometheus Metrics Collected
```yaml
Container Metrics (cAdvisor):
  - container_cpu_usage_seconds_total
  - container_memory_usage_bytes
  - container_memory_max_usage_bytes
  - container_fs_reads_bytes_total
  - container_fs_writes_bytes_total
  - container_network_receive_bytes_total (host-level only)
  - container_network_transmit_bytes_total (host-level only)

System Metrics (Node Exporter):
  - node_cpu_seconds_total
  - node_memory_MemTotal_bytes
  - node_memory_MemAvailable_bytes
  - node_disk_read_bytes_total
  - node_disk_written_bytes_total
  - node_network_receive_bytes_total
  - node_network_transmit_bytes_total
  - node_filesystem_avail_bytes

Docker Daemon Metrics:
  - engine_daemon_container_states_containers
  - engine_daemon_container_actions_seconds
```

### Grafana Dashboards
1. **Docker Containers (Working)**
   - Container CPU usage (by container ID)
   - Container memory usage (by container ID)
   - Host network I/O (by interface)
   - Host system gauges

2. **System Overview**
   - Overall system metrics
   - Disk usage
   - Network traffic

3. **Docker Overview**
   - Container states
   - Docker daemon metrics

## Backup and Recovery

### Backup Strategy
```yaml
Databases:
  - PostgreSQL: Daily dumps
  - Retention: 7 days

Docker Volumes:
  - grafana-data: Weekly snapshots
  - prometheus-data: Not backed up (metrics are ephemeral)
  - loki-data: Weekly snapshots

Configuration:
  - All config files in Git repository
  - .env file: Encrypted backup
```

### Recovery Time Objectives
- **RTO (Recovery Time Objective)**: 1 hour
- **RPO (Recovery Point Objective)**: 24 hours

## Security Specifications

### Authentication
- **Method**: OIDC via Keycloak
- **Session Duration**: 4 hours
- **Token Refresh**: 1 hour
- **Password Policy**: Keycloak default (configurable)

### Network Security
- **Firewall**: UFW (ports 80, 443, 22 only)
- **SSL/TLS**: A+ rating (SSL Labs)
- **HSTS**: Enabled
- **Rate Limiting**: Enabled on Nginx

### Container Security
- **User Namespaces**: Not enabled
- **Privileged Containers**: cAdvisor only
- **Read-only Filesystems**: Where applicable
- **Security Scanning**: Manual (recommended: Trivy)

## Compliance and Standards

- **HTTPS**: Enforced for all external traffic
- **Authentication**: Required for all services
- **Logging**: Centralized via Loki
- **Monitoring**: 24/7 via Prometheus/Grafana
- **Updates**: Manual, tested in staging first
