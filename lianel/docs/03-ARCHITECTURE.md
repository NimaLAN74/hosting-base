# Architecture Documentation

## System Architecture Layers

### Layer 1: External Access Layer

```mermaid
graph LR
    User[User] -->|DNS Lookup| DNS[DNS Server]
    DNS -->|Returns IP| User
    User -->|HTTPS Request| CDN[Optional CDN]
    CDN -->|Forward| Server[VPS Server - 72.60.80.84]
```

**Purpose**: Handle external traffic and DNS resolution

**Components**:
- DNS provider managing lianel.se domain
- Let's Encrypt for SSL certificates
- Optional CDN (not currently implemented)

### Layer 2: Ingress Layer

```mermaid
graph TD
    Internet[Internet] -->|Port 443| Nginx[Nginx Reverse Proxy]
    Nginx -->|SSL Termination| SSL[SSL/TLS Handler]
    SSL -->|HTTP| Router[Request Router]
    
    Router -->|/| Frontend
    Router -->|/monitoring| Grafana
    Router -->|airflow.lianel.se| Airflow
    Router -->|auth.lianel.se| Keycloak
```

**Purpose**: SSL termination, routing, and load balancing

**Key Features**:
- SSL/TLS termination
- Virtual host routing
- Rate limiting
- Proxy buffering for large headers
- Security headers injection

### Layer 3: Authentication Layer

```mermaid
sequenceDiagram
    participant User
    participant Nginx
    participant OAuth2
    participant Keycloak
    participant Backend
    
    User->>Nginx: Request /monitoring
    Nginx->>OAuth2: Auth check (/oauth2/auth)
    
    alt Not Authenticated
        OAuth2-->>Nginx: 401 Unauthorized
        Nginx-->>User: 302 Redirect to /oauth2/sign_in
        User->>OAuth2: GET /oauth2/sign_in
        OAuth2-->>User: 302 Redirect to Keycloak
        User->>Keycloak: Login page
        User->>Keycloak: Submit credentials
        Keycloak-->>User: OIDC token + redirect
        User->>OAuth2: Callback with token
        OAuth2->>Keycloak: Validate token
        Keycloak-->>OAuth2: Token valid
        OAuth2-->>User: Set session cookie + redirect
    end
    
    User->>Nginx: Request /monitoring (with cookie)
    Nginx->>OAuth2: Auth check
    OAuth2-->>Nginx: 200 OK + headers (X-Email, X-User)
    Nginx->>Backend: Forward request + user headers
    Backend-->>User: Response
```

**Purpose**: Centralized authentication and authorization

**Components**:
- **OAuth2 Proxy**: Authentication gateway
- **Keycloak**: Identity provider (IdP)
- **Session Management**: Cookie-based sessions

**Security Features**:
- OIDC/OAuth2 protocol
- PKCE (Proof Key for Code Exchange)
- Secure cookies (HttpOnly, Secure, SameSite)
- Token refresh
- Session timeout

### Layer 4: Application Layer

```mermaid
graph TB
    subgraph "Frontend Services"
        Frontend[React Frontend]
    end
    
    subgraph "Data Orchestration"
        AirflowWeb[Airflow Webserver]
        AirflowScheduler[Airflow Scheduler]
        AirflowWorker[Airflow Worker]
        AirflowTriggerer[Airflow Triggerer]
        AirflowDAG[DAG Processor]
    end
    
    subgraph "Monitoring Services"
        Grafana[Grafana]
    end
    
    AirflowWeb --> AirflowScheduler
    AirflowScheduler --> AirflowWorker
    AirflowScheduler --> AirflowTriggerer
    AirflowScheduler --> AirflowDAG
```

**Purpose**: Business logic and user interfaces

**Characteristics**:
- Stateless (except Airflow state in DB)
- Horizontally scalable (Airflow workers)
- Container-based deployment
- Auto-restart on failure

### Layer 5: Data Layer

```mermaid
graph TB
    subgraph "Persistent Storage"
        Postgres[(PostgreSQL)]
        Redis[(Redis)]
    end
    
    subgraph "Time-Series Storage"
        Prometheus[(Prometheus TSDB)]
        Loki[(Loki)]
    end
    
    subgraph "Application Data"
        Airflow[Airflow] -->|Metadata| Postgres
        Airflow -->|Task Queue| Redis
        Keycloak[Keycloak] -->|Users/Realms| Postgres
    end
    
    subgraph "Monitoring Data"
        cAdvisor[cAdvisor] -->|Metrics| Prometheus
        NodeExporter[Node Exporter] -->|Metrics| Prometheus
        Promtail[Promtail] -->|Logs| Loki
        Grafana[Grafana] -->|Query| Prometheus
        Grafana -->|Query| Loki
    end
```

**Purpose**: Data persistence and retrieval

**Storage Types**:
- **Relational**: PostgreSQL for structured data
- **Cache/Queue**: Redis for temporary data
- **Time-Series**: Prometheus for metrics
- **Logs**: Loki for log aggregation

### Layer 6: Monitoring Layer

```mermaid
graph LR
    subgraph "Metric Collection"
        cAdvisor[cAdvisor - Container Metrics]
        NodeExporter[Node Exporter - System Metrics]
        DockerDaemon[Docker Daemon - Docker Metrics]
    end
    
    subgraph "Log Collection"
        Promtail[Promtail - Log Shipper]
        DockerLogs[Docker Container Logs]
    end
    
    subgraph "Storage"
        Prometheus[(Prometheus - Metrics DB)]
        Loki[(Loki - Log DB)]
    end
    
    subgraph "Visualization"
        Grafana[Grafana - Dashboards]
    end
    
    cAdvisor -->|Scrape| Prometheus
    NodeExporter -->|Scrape| Prometheus
    DockerDaemon -->|Scrape| Prometheus
    
    DockerLogs -->|Tail| Promtail
    Promtail -->|Push| Loki
    
    Prometheus -->|Query| Grafana
    Loki -->|Query| Grafana
```

**Purpose**: System observability and alerting

**Metrics Collected**:
- Container CPU, memory, disk I/O
- System CPU, memory, disk, network
- Docker daemon statistics
- Application logs

## Component Interactions

### Frontend Request Flow

```mermaid
sequenceDiagram
    participant Browser
    participant Nginx
    participant OAuth2
    participant Frontend
    
    Browser->>Nginx: GET /
    Nginx->>OAuth2: Check auth
    OAuth2-->>Nginx: Authenticated
    Nginx->>Frontend: Proxy request
    Frontend-->>Nginx: Static files
    Nginx-->>Browser: HTML/JS/CSS
    Browser->>Browser: Render React app
```

### Airflow Task Execution Flow

```mermaid
sequenceDiagram
    participant Scheduler
    participant DB as PostgreSQL
    participant Queue as Redis
    participant Worker
    participant DAG
    
    Scheduler->>DB: Check for tasks
    DB-->>Scheduler: Pending tasks
    Scheduler->>Queue: Enqueue task
    Worker->>Queue: Poll for tasks
    Queue-->>Worker: Task details
    Worker->>DAG: Execute task
    DAG-->>Worker: Task result
    Worker->>DB: Update task status
    Scheduler->>DB: Check status
```

### Monitoring Data Flow

```mermaid
sequenceDiagram
    participant Container
    participant cAdvisor
    participant Prometheus
    participant Grafana
    participant User
    
    loop Every 15 seconds
        Prometheus->>cAdvisor: Scrape /metrics
        cAdvisor->>Container: Read cgroup stats
        Container-->>cAdvisor: CPU, memory, I/O
        cAdvisor-->>Prometheus: Metrics
        Prometheus->>Prometheus: Store in TSDB
    end
    
    User->>Grafana: View dashboard
    Grafana->>Prometheus: PromQL query
    Prometheus-->>Grafana: Time-series data
    Grafana-->>User: Rendered charts
```

## Design Patterns

### 1. Reverse Proxy Pattern
**Implementation**: Nginx as single entry point

**Benefits**:
- Centralized SSL termination
- Single point for security policies
- Easy service routing
- Load balancing capability

### 2. Sidecar Pattern
**Implementation**: Promtail alongside containers

**Benefits**:
- Decoupled log collection
- No application code changes
- Centralized log aggregation

### 3. Gateway Pattern
**Implementation**: OAuth2 Proxy as auth gateway

**Benefits**:
- Centralized authentication
- No auth code in applications
- Consistent security policy
- Easy to add new services

### 4. Service Discovery Pattern
**Implementation**: Docker DNS

**Benefits**:
- Automatic service resolution
- No hardcoded IPs
- Dynamic container addressing

### 5. Configuration Management Pattern
**Implementation**: Environment variables + mounted configs

**Benefits**:
- Separation of config from code
- Easy environment-specific settings
- Secure secret management

## Scalability Considerations

### Current Limitations
```mermaid
graph TD
    A[Single VPS Server] -->|Limits| B[Vertical Scaling Only]
    B --> C[Max 8GB RAM]
    B --> D[Max 2 CPU Cores]
    B --> E[Single Point of Failure]
```

### Horizontal Scaling Opportunities
```mermaid
graph LR
    subgraph "Can Scale Horizontally"
        AirflowWorker[Airflow Workers]
        Frontend[Frontend Instances]
    end
    
    subgraph "Cannot Scale Easily"
        Postgres[(PostgreSQL)]
        Redis[(Redis)]
        Prometheus[(Prometheus)]
    end
    
    subgraph "Requires Changes"
        Nginx[Nginx - Needs Load Balancer]
        Keycloak[Keycloak - Needs Clustering]
    end
```

### Future Architecture (Multi-Server)

```mermaid
graph TB
    subgraph "Load Balancer Tier"
        LB[Load Balancer]
    end
    
    subgraph "Application Tier"
        App1[App Server 1]
        App2[App Server 2]
        App3[App Server 3]
    end
    
    subgraph "Data Tier"
        DBPrimary[(PostgreSQL Primary)]
        DBReplica[(PostgreSQL Replica)]
        RedisCluster[(Redis Cluster)]
    end
    
    subgraph "Monitoring Tier"
        PromFed[Prometheus Federation]
        GrafanaHA[Grafana HA]
    end
    
    LB --> App1
    LB --> App2
    LB --> App3
    
    App1 --> DBPrimary
    App2 --> DBPrimary
    App3 --> DBPrimary
    
    DBPrimary --> DBReplica
```

## Security Architecture

### Defense in Depth

```mermaid
graph TD
    Layer1[Layer 1 - Network - Firewall, SSL/TLS]
    Layer2[Layer 2 - Application - OAuth2, Rate Limiting]
    Layer3[Layer 3 - Container - Isolation, Resource Limits]
    Layer4[Layer 4 - Data - Encryption, Access Control]
    Layer5[Layer 5 - Monitoring - Logging, Alerting]
    
    Layer1 --> Layer2
    Layer2 --> Layer3
    Layer3 --> Layer4
    Layer4 --> Layer5
```

### Trust Boundaries

```mermaid
graph TB
    subgraph "Untrusted Zone"
        Internet[Internet]
    end
    
    subgraph "DMZ"
        Nginx[Nginx]
    end
    
    subgraph "Trusted Zone - Auth"
        OAuth2[OAuth2 Proxy]
        Keycloak[Keycloak]
    end
    
    subgraph "Trusted Zone - Apps"
        Frontend[Frontend]
        Airflow[Airflow]
        Grafana[Grafana]
    end
    
    subgraph "Trusted Zone - Data"
        Postgres[(PostgreSQL)]
        Redis[(Redis)]
    end
    
    Internet -->|HTTPS Only| Nginx
    Nginx -->|Auth Check| OAuth2
    OAuth2 -->|OIDC| Keycloak
    Nginx -->|Authenticated| Frontend
    Nginx -->|Authenticated| Airflow
    Nginx -->|Authenticated| Grafana
    
    Airflow --> Postgres
    Airflow --> Redis
    Keycloak --> Postgres
```

## Failure Modes and Recovery

### Single Points of Failure

| Component | Impact | Mitigation | Recovery Time |
|-----------|--------|------------|---------------|
| VPS Server | Total outage | Backups, monitoring | 1-2 hours |
| Nginx | No external access | Auto-restart | 30 seconds |
| PostgreSQL | Airflow/Keycloak down | Backups, replication | 15-30 minutes |
| Keycloak | No new logins | Auto-restart, session cache | 1-2 minutes |
| OAuth2 Proxy | No authentication | Auto-restart | 30 seconds |

### Recovery Procedures

```mermaid
graph TD
    Failure[Service Failure Detected]
    Failure --> Auto{Auto-Restart - Successful?}
    
    Auto -->|Yes| Monitor[Monitor Service]
    Auto -->|No| Manual[Manual Intervention]
    
    Manual --> Logs[Check Logs]
    Logs --> Diagnose[Diagnose Issue]
    Diagnose --> Fix[Apply Fix]
    Fix --> Restart[Restart Service]
    Restart --> Verify[Verify Operation]
    Verify --> Monitor
```

## Performance Optimization

### Current Optimizations
1. **Nginx**: Gzip compression, proxy caching
2. **Frontend**: Static file serving, CDN-ready
3. **Prometheus**: Recording rules for complex queries
4. **Docker**: Resource limits prevent resource exhaustion
5. **PostgreSQL**: Connection pooling

### Monitoring Performance

```mermaid
graph LR
    subgraph "Key Metrics"
        CPU[CPU Usage - Target under 70%]
        Memory[Memory Usage - Target under 80%]
        Disk[Disk I/O - Target under 80%]
        Network[Network - Target under 100 Mbps]
    end
    
    subgraph "Application Metrics"
        ResponseTime[Response Time - Target under 500ms]
        ErrorRate[Error Rate - Target under 1%]
        Throughput[Throughput - Monitor]
    end
```

## Technology Decisions

### Why Docker Compose?
- **Pros**: Simple, single-server deployment, easy to understand
- **Cons**: No orchestration, manual scaling, single point of failure
- **Alternative**: Kubernetes (overkill for current scale)

### Why Keycloak?
- **Pros**: Full-featured IAM, OIDC support, user management
- **Cons**: Resource-heavy, complex configuration
- **Alternative**: Auth0 (SaaS, costs money)

### Why Prometheus + Loki?
- **Pros**: Industry standard, powerful querying, Grafana integration
- **Cons**: Storage overhead, retention limits
- **Alternative**: ELK Stack (more complex), Cloud services (costs)

### Why Single VPS?
- **Pros**: Cost-effective, simple operations, sufficient for current load
- **Cons**: No high availability, limited scalability
- **Alternative**: Multi-server cluster (higher cost, complexity)
