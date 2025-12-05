# Lianel Infrastructure - System Overview

## Executive Summary

The Lianel infrastructure is a containerized web application platform deployed on a single VPS server, providing a React frontend, Apache Airflow data orchestration, and comprehensive monitoring with Single Sign-On (SSO) authentication across all services.

## System Architecture

```mermaid
graph TB
    subgraph "External Access"
        User[User Browser]
        DNS[DNS - lianel.se]
    end
    
    subgraph "VPS Server - lianel.se (72.60.80.84)"
        subgraph "Nginx Reverse Proxy"
            Nginx[Nginx - Port 80/443]
        end
        
        subgraph "Authentication Layer"
            OAuth2[OAuth2 Proxy - Port 4180]
            Keycloak[Keycloak - Port 8080]
        end
        
        subgraph "Application Services"
            Frontend[React Frontend - Port 80]
            ProfileService[Profile Service - Rust API - Port 3000]
            Airflow[Airflow Web UI - Port 8080]
            Grafana[Grafana - Port 3000]
        end
        
        subgraph "Monitoring Stack"
            Prometheus[Prometheus - Port 9090]
            Loki[Loki - Port 3100]
            Promtail[Promtail]
            cAdvisor[cAdvisor - Port 8080]
            NodeExporter[Node Exporter - Port 9100]
        end
        
        subgraph "Data Layer"
            Postgres[(PostgreSQL - Host DB)]
            Redis[(Redis)]
        end
        
        subgraph "Airflow Workers"
            AirflowScheduler[Scheduler]
            AirflowWorker[Worker]
            AirflowTriggerer[Triggerer]
        end
    end
    
    User -->|HTTPS| DNS
    DNS -->|HTTPS| Nginx
    Nginx -->|Auth Check| OAuth2
    OAuth2 -->|OIDC| Keycloak
    Nginx -->|Proxy| Frontend
    Nginx -->|Proxy| ProfileService
    Frontend -->|API Calls| ProfileService
    ProfileService -->|Admin API| Keycloak
    Nginx -->|Proxy| Airflow
    Nginx -->|Proxy| Grafana
    
    Grafana -->|Query| Prometheus
    Grafana -->|Query| Loki
    Prometheus -->|Scrape| cAdvisor
    Prometheus -->|Scrape| NodeExporter
    Promtail -->|Push Logs| Loki
    
    Airflow -->|DB| Postgres
    Airflow -->|Queue| Redis
    Keycloak -->|DB| Postgres
```

## Domain Structure

| Domain | Purpose | Backend Service |
|--------|---------|----------------|
| `lianel.se` | Main website | React Frontend |
| `www.lianel.se` | Main website (canonical) | React Frontend |
| `www.lianel.se/monitoring` | Grafana dashboards | Grafana |
| `monitoring.lianel.se` | Grafana (subdomain) | Grafana |
| `airflow.lianel.se` | Airflow web UI | Airflow |
| `auth.lianel.se` | Keycloak SSO | Keycloak |

## Technology Stack

### Frontend
- **Framework**: React 18
- **Build Tool**: Create React App
- **Container**: Nginx serving static files

### Backend Services
- **Profile Service**: Rust-based API service for user profile management
- **Orchestration**: Apache Airflow 3.1.3
- **Authentication**: Keycloak (latest)
- **Auth Proxy**: OAuth2 Proxy (latest)

### Monitoring
- **Metrics**: Prometheus 2.54.1
- **Visualization**: Grafana 11.3.0
- **Logs**: Loki 3.0.0 + Promtail 3.0.0
- **Container Metrics**: cAdvisor 0.47.0
- **System Metrics**: Node Exporter 1.8.2

### Infrastructure
- **Reverse Proxy**: Nginx (latest)
- **Database**: PostgreSQL (host-level)
- **Cache/Queue**: Redis (latest)
- **Container Runtime**: Docker with Docker Compose
- **SSL**: Let's Encrypt certificates

## Network Architecture

```mermaid
graph LR
    subgraph "External Network"
        Internet[Internet]
    end
    
    subgraph "VPS - Public Interface"
        PublicIP[72.60.80.84 - Ports 80, 443]
    end
    
    subgraph "Docker Network: lianel-network (172.18.0.0/16)"
        Nginx[Nginx - 172.18.0.x]
        OAuth2Proxy[OAuth2 Proxy - 172.18.0.10]
        Keycloak[Keycloak - 172.18.0.x]
        Frontend[Frontend - 172.18.0.x]
        Grafana[Grafana - 172.18.0.x]
        Prometheus[Prometheus - 172.18.0.x]
        cAdvisor[cAdvisor - 172.18.0.3]
    end
    
    subgraph "Host Network"
        HostDB[(PostgreSQL - localhost 5432)]
        DockerDaemon[Docker Daemon - localhost 9323]
    end
    
    Internet -->|HTTPS:443| PublicIP
    PublicIP --> Nginx
    Nginx --> OAuth2Proxy
    OAuth2Proxy --> Keycloak
    Nginx --> Frontend
    Nginx --> Grafana
    
    Keycloak -->|host.docker.internal| HostDB
    Prometheus -->|host.docker.internal| DockerDaemon
```

## Security Architecture

```mermaid
graph TD
    User[User Request] -->|HTTPS| Nginx
    Nginx -->|Auth Check| OAuth2Proxy
    OAuth2Proxy -->|Valid Session?| Decision{Has Valid Session?}
    
    Decision -->|No| Redirect[Redirect to Keycloak]
    Redirect --> Keycloak[Keycloak Login]
    Keycloak -->|OIDC Token| OAuth2Proxy
    OAuth2Proxy -->|Set Cookie| SetCookie[Set Session Cookie Domain .lianel.se]
    
    Decision -->|Yes| PassHeaders[Pass User Headers]
    SetCookie --> PassHeaders
    PassHeaders -->|X-Email, X-User| Backend[Backend Service]
    Backend --> Response[Response to User]
```

## Deployment Model

- **Environment**: Single VPS server
- **OS**: Ubuntu/Debian Linux
- **Container Orchestration**: Docker Compose
- **Configuration Management**: Environment files (.env)
- **Service Discovery**: Docker DNS
- **Storage**: Docker volumes + host filesystem

## Key Features

1. **Single Sign-On (SSO)**: Unified authentication across all services via Keycloak
2. **OAuth2 Proxy**: Centralized authentication gateway
3. **SSL/TLS**: Automatic HTTPS with Let's Encrypt
4. **Monitoring**: Comprehensive metrics and logs collection
5. **High Availability**: Automatic container restart policies
6. **Scalability**: Airflow worker can be scaled horizontally

## Service Dependencies

```mermaid
graph TD
    Nginx[Nginx] --> OAuth2[OAuth2 Proxy]
    Nginx --> Frontend[Frontend]
    Nginx --> Grafana[Grafana]
    Nginx --> Airflow[Airflow]
    
    OAuth2 --> Keycloak[Keycloak]
    Keycloak --> PostgreSQL[(PostgreSQL)]
    
    Airflow --> PostgreSQL
    Airflow --> Redis[(Redis)]
    
    Grafana --> Prometheus[Prometheus]
    Grafana --> Loki[Loki]
    
    Prometheus --> cAdvisor[cAdvisor]
    Prometheus --> NodeExporter[Node Exporter]
    Prometheus --> DockerDaemon[Docker Daemon]
    
    Promtail[Promtail] --> Loki
```

## Data Flow

### User Authentication Flow
1. User accesses `https://www.lianel.se`
2. Nginx checks authentication via OAuth2 Proxy
3. If not authenticated, redirect to Keycloak login
4. User logs in with credentials
5. Keycloak issues OIDC token
6. OAuth2 Proxy validates token and sets session cookie
7. User is redirected back to original URL
8. Subsequent requests use session cookie

### Monitoring Data Flow
1. cAdvisor collects container metrics from Docker
2. Node Exporter collects system metrics
3. Prometheus scrapes metrics every 15 seconds
4. Promtail tails Docker container logs
5. Promtail pushes logs to Loki
6. Grafana queries both Prometheus and Loki
7. Users view dashboards via Grafana UI

## File Structure

```
lianel/dc/
├── docker-compose.yaml              # Main compose file
├── docker-compose.airflow.yaml      # Airflow services
├── docker-compose.frontend.yaml     # React frontend
├── docker-compose.monitoring.yaml   # Monitoring stack
├── docker-compose.oauth2-proxy.yaml # Auth services
├── docker-compose.infra.yaml        # Infrastructure
├── .env                             # Environment variables
├── nginx/
│   └── config/
│       └── nginx.conf               # Nginx configuration
├── monitoring/
│   ├── prometheus/
│   │   ├── prometheus.yml           # Prometheus config
│   │   └── recording-rules.yml      # Recording rules
│   ├── grafana/
│   │   └── provisioning/            # Dashboards & datasources
│   ├── loki/
│   └── promtail/
├── frontend/                        # React application
└── dags/                           # Airflow DAGs
```

## Operational Characteristics

- **Uptime Target**: 99.9%
- **Backup Strategy**: Database backups, volume snapshots
- **Update Strategy**: Rolling updates via Docker Compose
- **Monitoring**: 24/7 via Grafana dashboards
- **Log Retention**: 15 days (Prometheus), configurable (Loki)
- **SSL Certificate Renewal**: Automatic via Let's Encrypt

## Next Steps

For detailed information, see:
- [Technical Specifications](./02-TECHNICAL-SPECIFICATIONS.md)
- [Architecture Details](./03-ARCHITECTURE.md)
- [Deployment Guide](./04-DEPLOYMENT-GUIDE.md)
- [Monitoring Setup](./05-MONITORING-SETUP.md)
- [Security Configuration](./06-SECURITY-CONFIGURATION.md)
