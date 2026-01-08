# Project Goals and Implementation Plan - Comprehensive Analysis

**Date**: January 7, 2026  
**Based on**: P0-01 through P0-08 documentation review

---

## Executive Summary

The Lianel project is a **containerized web application platform** deployed on a single VPS server (72.60.80.84) that provides:

1. **Current Infrastructure** (Phase 0 - Complete):
   - React frontend with SSO authentication
   - Apache Airflow for data orchestration
   - Comprehensive monitoring (Prometheus, Grafana, Loki)
   - Keycloak SSO across all services
   - Profile Service (Rust-based API)

2. **Future Vision** (Phase 0+ - Documentation Phase):
   - **EU Energy & Geospatial Intelligence Platform**
   - Data ingestion from Eurostat, ENTSO-E, OSM, NUTS
   - ML-ready datasets for forecasting, clustering, geo-enrichment
   - Analytical dashboards and self-service data access

---

## Part 1: Current Infrastructure Goals (P0-01 to P0-08)

### P0-01: System Overview

**Goal**: Provide a comprehensive containerized platform with SSO

**Key Components**:
- **Frontend**: React 18 (static files served by Nginx)
- **Backend**: Rust Profile Service (port 3000)
- **Orchestration**: Apache Airflow 3.1.3
- **Authentication**: Keycloak + OAuth2 Proxy
- **Monitoring**: Prometheus + Grafana + Loki stack
- **Database**: PostgreSQL (host-level, not containerized)
- **Cache/Queue**: Redis

**Architecture Pattern**:
- Single VPS deployment (72.60.80.84)
- Docker Compose orchestration
- Nginx as reverse proxy with SSL termination
- OAuth2 Proxy as authentication gateway
- All services on `lianel-network` Docker network

**Domain Structure**:
- `lianel.se` / `www.lianel.se` → Frontend
- `auth.lianel.se` → Keycloak
- `airflow.lianel.se` → Airflow UI
- `monitoring.lianel.se` → Grafana

### P0-02: Technical Specifications

**Goal**: Define exact service configurations and resource requirements

**Key Specifications**:
- **VPS**: 2 cores, 8GB RAM, 100GB storage
- **Docker**: Latest stable, Docker Compose 2.0+
- **SSL**: Let's Encrypt certificates (auto-renewal)
- **Firewall**: UFW (ports 22, 80, 443 only)
- **Resource Usage**: ~4.5GB RAM / 8GB (55%), ~25-30% CPU average

**Service Ports**:
- All services internal-only except Nginx (80/443)
- PostgreSQL: 5432 (host-level, accessible via `172.18.0.1` from containers)

### P0-03: Architecture

**Goal**: Document system architecture layers and design patterns

**Architecture Layers**:
1. **External Access**: DNS, SSL/TLS
2. **Ingress**: Nginx reverse proxy
3. **Authentication**: OAuth2 Proxy + Keycloak
4. **Application**: Frontend, Airflow, Profile Service
5. **Data**: PostgreSQL, Redis
6. **Monitoring**: Prometheus, Loki, Grafana

**Design Patterns**:
- Reverse Proxy Pattern (Nginx)
- Sidecar Pattern (Promtail)
- Gateway Pattern (OAuth2 Proxy)
- Service Discovery (Docker DNS)
- Configuration Management (Environment variables)

**Security Architecture**:
- Defense in depth (7 layers)
- Trust boundaries (DMZ → Trusted zones)
- HTTPS enforced for all external traffic
- Rate limiting on Nginx
- Security headers (X-Frame-Options, CSP, HSTS)

### P0-04: Deployment Guide

**Goal**: Provide step-by-step deployment instructions

**Deployment Model**:
- **Environment**: Single VPS server
- **OS**: Ubuntu/Debian Linux
- **Container Orchestration**: Docker Compose
- **Configuration**: Environment files (.env)
- **Service Discovery**: Docker DNS
- **Storage**: Docker volumes + host filesystem

**Deployment Workflow**:
1. Initial server setup (Docker, firewall, DNS)
2. PostgreSQL setup (host-level)
3. Infrastructure services (Nginx)
4. SSL certificates (Let's Encrypt)
5. Authentication services (Keycloak, OAuth2 Proxy)
6. Application services (Frontend, Airflow)
7. Monitoring stack (Prometheus, Grafana, Loki)

**CI/CD**: GitHub Actions for automated deployment
- Build Docker images for `linux/amd64`
- Push to GitHub Container Registry (GHCR)
- Deploy to remote host via SSH
- Health checks after deployment

### P0-05: Monitoring Setup

**Goal**: Comprehensive observability for infrastructure

**Monitoring Stack**:
- **Metrics**: Prometheus (15s scrape interval, 15-day retention)
- **Logs**: Loki + Promtail (Docker container logs)
- **Visualization**: Grafana (dashboards for containers, system, Docker)
- **Collection**: cAdvisor (container metrics), Node Exporter (system metrics)

**Key Metrics**:
- Container CPU, memory, disk I/O
- System CPU, memory, disk, network
- Docker daemon statistics
- Application logs

**Access**: Grafana at `https://www.lianel.se/monitoring/` (SSO required)

### P0-06: Security Configuration

**Goal**: Multi-layer security following defense-in-depth principles

**Security Layers**:
1. **Network**: Firewall (UFW), SSL/TLS
2. **Authentication**: SSO, OAuth2
3. **Authorization**: Role-based access
4. **Application**: Rate limiting, security headers
5. **Container**: Isolation, resource limits
6. **Data**: Encryption, backups
7. **Monitoring**: Logging, alerting

**Key Security Features**:
- **SSO**: Keycloak with OIDC
- **MFA**: Conditional 2FA with OTP (configured for all users)
- **Session Management**: 4-hour expiration, 1-hour refresh
- **Back-Channel Logout**: Proper SSO logout across all services
- **Rate Limiting**: 10 req/s general, 5 req/m login
- **Security Headers**: X-Frame-Options, CSP, HSTS, X-Content-Type-Options

**Secrets Management**:
- Environment variables in `.env` file (600 permissions)
- Strong password generation
- Secret rotation procedures

### P0-07: Remote Host Analysis

**Goal**: Document current remote host state and deployment workflow

**Current State** (as of December 2024):
- **Host**: 72.60.80.84 (lianel.se)
- **Docker**: 29.0.4
- **Disk**: 24GB / 96GB (26% used)
- **Memory**: 4.8GB / 7.8GB (62% used)
- **All Services**: Running and healthy

**Deployment Workflow**:
1. Local build (with `--platform linux/amd64` for cross-platform)
2. Save image as `.tar.gz`
3. SCP to remote host
4. Load image on remote
5. Deploy with Docker Compose

**Issues Identified** (now resolved):
- ✅ Keycloak health check (non-critical)
- ✅ Frontend Docker Compose file (build section)
- ✅ Network connectivity (all services on `lianel-network`)

### P0-08: Profile Service

**Goal**: Rust-based backend API for user profile management

**Features**:
- User profile retrieval and updates
- Password change with verification
- OpenAPI 3.0 specification
- Swagger UI at `/swagger-ui` (OAuth2 protected)
- Health check endpoint

**Technology**:
- Rust + Axum 0.7
- Small binary (~10-15MB)
- Low memory footprint (~15-20MB)
- Fast response times (<100ms)

**Security**:
- OAuth2 authentication required
- User identification via headers (OAuth2-proxy)
- Keycloak admin credentials never exposed to frontend

---

## Part 2: Future Vision - EU Energy & Geospatial Intelligence Platform

### Goals (from Requirements & Roadmap)

**Primary Goal**: Build a data platform that:
1. Ingests energy data from Eurostat, ENTSO-E
2. Integrates geospatial data from NUTS, OSM
3. Harmonizes and validates data
4. Creates ML-ready datasets
5. Provides analytical dashboards
6. Enables self-service data access

**Data Sources**:
- **Eurostat**: Energy balances, indicators (annual data)
- **ENTSO-E**: High-frequency electricity data (hourly/15-min)
- **OSM**: Geospatial features (power plants, infrastructure)
- **NUTS**: Administrative boundaries (NUTS0, NUTS1, NUTS2)

**ML Datasets**:
- **Forecasting**: Time-series features for energy demand prediction
- **Clustering**: Regional energy mix patterns
- **Geo-Enrichment**: Energy data + spatial features

### Implementation Roadmap (Phase 0-7)

#### Phase 0: Documentation Enhancement & Requirements (Current)
**Duration**: 2-3 weeks  
**Status**: IN PROGRESS

**Objectives**:
- Complete technical specifications
- Assess Eurostat + NUTS data coverage
- Finalize storage architecture decisions
- Create detailed Airflow DAG specifications

**Key Tasks**:
1. **Eurostat API Research**
   - Document API endpoints and table codes
   - Test rate limits and authentication
   - Assess data completeness for EU27
   - Estimate data volumes

2. **NUTS Data Assessment**
   - Identify GISCO download URLs
   - Test spatial data loading
   - Verify coordinate system handling

3. **Storage Architecture Decision**
   - Compare PostgreSQL vs TimescaleDB
   - Estimate storage requirements
   - Design partitioning strategy

4. **Airflow DAG Design**
   - Define DAG structure and dependencies
   - Specify error handling and retry logic
   - Design data validation rules

**Deliverables**:
- Enhanced data inventory documents
- Complete database schema (DDL)
- Airflow DAG specifications
- Data quality framework

#### Phase 1: Foundation - Infrastructure Setup
**Duration**: 2-3 weeks  
**Dependencies**: Phase 0 complete

**Objectives**:
- Set up database infrastructure
- Configure Airflow for data pipelines
- Establish monitoring and alerting
- Create development environment

**Key Tasks**:
- Database setup (PostgreSQL + PostGIS)
- Airflow configuration (connections, secrets)
- Monitoring dashboards (Grafana)
- Development environment setup

#### Phase 2: Eurostat + NUTS Implementation
**Duration**: 4-6 weeks  
**Dependencies**: Phase 1 complete

**Objectives**:
- Implement Eurostat data ingestion pipeline
- Load and process NUTS geospatial data
- Harmonize and validate data
- Create initial ML datasets

**Key DAGs**:
1. Eurostat Raw Ingestion
2. Eurostat Standardization
3. Eurostat Enrichment
4. NUTS Boundary Loading
5. Spatial Validation
6. Country-Region Mapping

#### Phase 3: Data Coverage Assessment & Decision Point
**Duration**: 1-2 weeks

**Objectives**:
- Evaluate Eurostat + NUTS data sufficiency
- Assess ML dataset readiness
- Decide on next data sources (ENTSO-E, OSM)

**Decision Point**:
- **IF sufficient**: Proceed to Phase 4 (ML & Analytics)
- **IF gaps exist**: Proceed to Phase 5 (Additional Sources)

#### Phase 4: ML & Analytics (if Eurostat + NUTS sufficient)
**Duration**: 4-6 weeks

**Objectives**:
- Create all ML datasets (forecasting, clustering, geo-enrichment)
- Develop exploratory analysis notebooks
- Build analytical dashboards
- Enable self-service data access

#### Phase 5: Additional Sources Integration (if needed)
**Duration**: 6-8 weeks

**Objectives**:
- Integrate ENTSO-E for high-frequency electricity data
- Add OSM for geospatial enrichment
- Fill identified data gaps

#### Phase 6: Operationalization & Optimization
**Duration**: 3-4 weeks

**Objectives**:
- Optimize pipeline performance
- Implement advanced monitoring
- Establish operational procedures
- Enable production use

#### Phase 7: Continuous Improvement
**Duration**: Ongoing

**Objectives**:
- Iterate based on user feedback
- Expand data sources as needed
- Enhance ML capabilities
- Maintain and evolve platform

---

## Part 3: Key Decisions and Architecture

### Database Architecture

**Decision**: PostgreSQL + PostGIS (host-level, not containerized)

**Rationale**:
- Already running in Lianel infrastructure
- Team familiarity
- PostGIS handles geospatial needs
- Can add TimescaleDB extension later if needed
- Partitioning by year handles time-series adequately

**Configuration**:
- PostgreSQL accessible from containers via `172.18.0.1:5432`
- Separate databases: `airflow`, `keycloak`
- Future: Additional database for energy platform data

### Network Architecture

**Docker Network**: `lianel-network` (172.18.0.0/16)

**Key Points**:
- All services on `lianel-network` for inter-service communication
- PostgreSQL on host, accessible via Docker network gateway (172.18.0.1)
- Nginx as single entry point (ports 80/443)
- All other services internal-only

### Authentication Architecture

**SSO Flow**:
1. User accesses protected resource
2. Nginx checks authentication via OAuth2 Proxy
3. If not authenticated → redirect to Keycloak
4. User logs in → Keycloak issues OIDC token
5. OAuth2 Proxy validates token → sets session cookie
6. Subsequent requests use session cookie

**MFA Configuration**:
- Conditional 2FA (Browser flow)
- OTP Form (REQUIRED within Conditional 2FA)
- Configure OTP as default required action
- All users must configure OTP before login

### Deployment Architecture

**CI/CD Pipeline**:
- GitHub Actions for automated deployment
- Build images for `linux/amd64`
- Push to GitHub Container Registry (GHCR)
- Deploy to remote host via SSH
- Health checks after deployment

**Current Workflow**:
1. Build locally (with `--platform linux/amd64`)
2. Push to GHCR
3. Remote host pulls from GHCR
4. Deploy with Docker Compose
5. Verify health endpoints

---

## Part 4: Current State Assessment

### Infrastructure Status

**✅ Complete and Operational**:
- React frontend with SSO
- Keycloak authentication
- OAuth2 Proxy
- Apache Airflow (all services healthy, DAGs working)
- Profile Service (Rust API)
- Monitoring stack (Prometheus, Grafana, Loki)
- Nginx reverse proxy with SSL
- PostgreSQL (host-level)
- Redis

**✅ Recent Fixes**:
- Airflow DAG execution (network connectivity)
- Profile Service token validation
- Swagger UI authentication
- PostgreSQL migration (container → host-level)
- Security hardening (firewall, PostgreSQL access)

### Documentation Status

**✅ Complete**:
- P0-01: System Overview
- P0-02: Technical Specifications
- P0-03: Architecture
- P0-04: Deployment Guide
- P0-05: Monitoring Setup
- P0-06: Security Configuration
- P0-07: Remote Host Analysis
- P0-08: Profile Service

**📋 In Progress** (Phase 0):
- Eurostat API research
- NUTS data assessment
- Storage architecture decision
- Airflow DAG design
- Data quality framework

---

## Part 5: Next Steps and Priorities

### Immediate Actions (This Week)

1. **Complete Phase 0 Documentation**:
   - [ ] Eurostat API exploration and documentation
   - [ ] NUTS geospatial data download and testing
   - [ ] Storage technology decision (PostgreSQL confirmed)
   - [ ] Complete database schema with DDL
   - [ ] Airflow DAG designs
   - [ ] Data quality rules definition

2. **Infrastructure Verification**:
   - [ ] Verify all services are healthy
   - [ ] Test DAG execution in Airflow
   - [ ] Verify monitoring dashboards
   - [ ] Test SSO across all services

### Short-Term (Next 2-3 Weeks)

1. **Phase 0 Completion**:
   - All documentation gaps filled
   - Clear decision on database technology
   - Eurostat + NUTS coverage confirmed
   - Detailed Airflow DAG designs reviewed

2. **Phase 1 Preparation**:
   - Database schema finalized
   - Airflow connections configured
   - Development environment setup
   - Monitoring dashboards designed

### Medium-Term (Next 1-2 Months)

1. **Phase 1: Foundation Setup**
   - Database infrastructure provisioned
   - Airflow configured for data pipelines
   - Monitoring and alerting established
   - Development environment ready

2. **Phase 2: Eurostat + NUTS Implementation**
   - Data ingestion pipelines operational
   - Geospatial data loaded
   - Initial ML datasets created

---

## Part 6: Success Metrics

### Infrastructure Metrics

- **Uptime**: >99.9% target
- **Response Time**: <500ms for API calls
- **Resource Usage**: <80% memory, <70% CPU
- **Pipeline Success Rate**: >95%

### Data Platform Metrics (Future)

- **Data Completeness**: >95% for critical fields
- **Data Accuracy**: >98% manual spot-checks pass
- **Timeliness**: Data refreshed within SLA (weekly)
- **Pipeline Runtime**: <2 hours for full refresh
- **Query Performance**: <5 seconds for analytical queries

---

## Part 7: Risk Management

### Current Infrastructure Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Single VPS failure | High | Low | Backups, monitoring, quick recovery |
| PostgreSQL corruption | High | Low | Regular backups, replication (future) |
| SSL certificate expiration | Medium | Low | Auto-renewal configured |
| Resource exhaustion | Medium | Medium | Monitoring, resource limits |

### Data Platform Risks (Future)

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Eurostat API changes | High | Medium | Version API calls, monitor deprecations |
| Data quality issues | High | Medium | Comprehensive validation, alerting |
| Storage scaling needs | Medium | High | Partitioning, capacity planning |
| Performance degradation | Medium | Medium | Continuous monitoring, optimization |

---

## Conclusion

The Lianel project has a **solid foundation** with:
- ✅ Complete infrastructure (frontend, backend, orchestration, monitoring)
- ✅ SSO authentication across all services
- ✅ Comprehensive documentation (P0-01 to P0-08)
- ✅ Operational deployment on remote host

**Next Phase**: Complete Phase 0 documentation for the EU Energy & Geospatial Intelligence Platform, then proceed with Phase 1 infrastructure setup.

**Timeline**: 5-8 months to full production (depending on data source decisions)

---

**Document Created**: January 7, 2026  
**Based on**: Comprehensive review of P0-01 through P0-08, requirements, and roadmap documents

