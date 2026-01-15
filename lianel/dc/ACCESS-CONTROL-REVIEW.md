# Access Control Review
**Last Updated**: January 15, 2026

---

## Overview

This document reviews access control configurations across the Lianel platform to ensure proper authentication and authorization.

---

## 1. Keycloak Access Control

### Realm Configuration
- **Realm Name**: `lianel`
- **SSL Required**: External
- **Login with Email**: Enabled
- **Duplicate Emails**: Not Allowed
- **Brute Force Protection**: Enabled

### Client Configuration

#### Frontend Client
- **Client ID**: `frontend-client`
- **Type**: Public Client
- **Standard Flow**: Enabled
- **Direct Access Grants**: Enabled
- **Valid Redirect URIs**:
  - `https://www.lianel.se`
  - `https://www.lianel.se/`
- **Web Origins**: `*` (should be restricted in production)

**Review Status**: ⚠️ **Needs Review**
- **Issue**: Web Origins set to `*` allows any origin
- **Recommendation**: Restrict to specific domains

#### OAuth2 Proxy Client
- **Client ID**: `oauth2-proxy`
- **Type**: Confidential
- **Standard Flow**: Enabled
- **Valid Redirect URIs**:
  - `https://www.lianel.se/oauth2/callback`
  - `https://monitoring.lianel.se/oauth2/callback`
  - `https://airflow.lianel.se/oauth2/callback`

**Review Status**: ✅ **OK**

#### Grafana Client
- **Client ID**: `grafana`
- **Type**: Confidential
- **Standard Flow**: Enabled
- **Valid Redirect URIs**:
  - `https://www.lianel.se/monitoring/*`
  - `https://monitoring.lianel.se/*`

**Review Status**: ✅ **OK**

#### Airflow Client
- **Client ID**: `airflow`
- **Type**: Confidential
- **Standard Flow**: Enabled
- **Valid Redirect URIs**:
  - `https://airflow.lianel.se/oauth-authorized/keycloak`

**Review Status**: ✅ **OK**

#### Backend Client (Service-to-Service)
- **Client ID**: `backend-client`
- **Type**: Confidential
- **Service Accounts**: Enabled
- **Client Credentials Flow**: Enabled

**Review Status**: ✅ **OK**

### User Roles

#### Standard Roles
- **Viewer**: Read-only access
- **Editor**: Read and write access
- **Admin**: Full access

#### Role Assignments
- [ ] Review user role assignments
- [ ] Verify least privilege principle
- [ ] Document role permissions

---

## 2. API Access Control

### Public Endpoints (No Authentication)
- `/api/v1/health`
- `/api/energy/health`

**Review Status**: ✅ **OK** - Health checks should be public

### Protected Endpoints (Authentication Required)

#### Energy Service Endpoints
- `/api/v1/energy/annual` - Requires Bearer token
- `/api/v1/energy/timeseries` - Requires Bearer token
- `/api/v1/datasets/*` - Requires Bearer token

**Review Status**: ✅ **OK** - All protected endpoints require authentication

#### Geo Service Endpoints
- `/api/v1/geo/features` - Requires Bearer token (optional, but recommended)

**Review Status**: ⚠️ **Needs Review**
- **Issue**: Geo endpoints may not require authentication
- **Recommendation**: Require authentication for all data endpoints

### Token Validation
- [ ] Tokens are validated on every request
- [ ] Expired tokens are rejected
- [ ] Invalid tokens are rejected
- [ ] Token claims are checked

**Review Status**: ✅ **OK** - Backend validates Keycloak tokens

---

## 3. Database Access Control

### PostgreSQL Users

#### Airflow Database User
- **Username**: `airflow`
- **Database**: `airflow`
- **Permissions**: Full access to `airflow` database

**Review Status**: ✅ **OK** - Appropriate for Airflow

#### Keycloak Database User
- **Username**: `keycloak`
- **Database**: `keycloak`
- **Permissions**: Full access to `keycloak` database

**Review Status**: ✅ **OK** - Appropriate for Keycloak

#### Energy Service Database User
- **Username**: `energy_user` (if exists)
- **Database**: `energy_db`
- **Permissions**: Read/write access to energy tables

**Review Status**: ⚠️ **Needs Review**
- **Issue**: Verify user has minimal required permissions
- **Recommendation**: Use read-only user for queries, separate user for writes

### Connection Security
- [ ] Database is not exposed externally
- [ ] Connections use strong passwords
- [ ] Connection encryption is enabled (if available)
- [ ] Connection pooling is configured

**Review Status**: ✅ **OK** - Database is internal only

---

## 4. Service-to-Service Authentication

### Backend Services
- **Profile Service**: Uses backend client credentials
- **Energy Service**: Uses backend client credentials

**Review Status**: ✅ **OK** - Services authenticate with Keycloak

### Internal Communication
- [ ] Services communicate over private network
- [ ] No authentication required for internal calls (if appropriate)
- [ ] Sensitive data is encrypted in transit

**Review Status**: ✅ **OK** - Services use Docker network

---

## 5. Container Access Control

### Container Users
- [ ] Containers run as non-root user (where possible)
- [ ] User permissions are minimal
- [ ] No unnecessary capabilities

**Review Status**: ⚠️ **Needs Review**
- **Issue**: Some containers may run as root
- **Recommendation**: Review and update container configurations

### File System Permissions
- [ ] Read-only filesystems where possible
- [ ] Write access is restricted
- [ ] Sensitive files have restricted permissions

**Review Status**: ⚠️ **Needs Review**
- **Issue**: Containers may have full filesystem access
- **Recommendation**: Implement read-only filesystems

---

## 6. Monitoring and Logging Access

### Grafana Access
- **Authentication**: SSO via Keycloak
- **Authorization**: Role-based (Viewer, Editor, Admin)

**Review Status**: ✅ **OK**

### Log Access
- [ ] Logs are accessible only to authorized users
- [ ] Sensitive data is not logged
- [ ] Log retention is appropriate

**Review Status**: ✅ **OK** - Logs accessible via Grafana with SSO

---

## 7. Recommendations

### High Priority
1. **Restrict Frontend Client Web Origins**
   - Change from `*` to specific domains
   - Prevents unauthorized origins from using the client

2. **Require Authentication for Geo Endpoints**
   - All data endpoints should require authentication
   - Prevents unauthorized data access

3. **Review Container User Permissions**
   - Run containers as non-root users
   - Implement read-only filesystems

### Medium Priority
1. **Implement Database Read-Only Users**
   - Separate read and write database users
   - Reduces risk of accidental data modification

2. **Review Role Permissions**
   - Document role permissions
   - Verify least privilege principle

3. **Implement API Rate Limiting per User**
   - Current rate limiting is per IP
   - Add per-user rate limiting for authenticated requests

### Low Priority
1. **Implement API Key Rotation**
   - Regular rotation of service account credentials
   - Automated rotation process

2. **Audit Logging**
   - Log all authentication events
   - Log authorization failures
   - Review audit logs regularly

---

## 8. Access Control Matrix

| Resource | Public | Viewer | Editor | Admin |
|----------|--------|--------|--------|-------|
| Frontend | ✅ | ✅ | ✅ | ✅ |
| API Health | ✅ | ✅ | ✅ | ✅ |
| API Data | ❌ | ✅ (Read) | ✅ (Read/Write) | ✅ (Full) |
| Grafana | ❌ | ✅ (View) | ✅ (View/Edit) | ✅ (Full) |
| Airflow | ❌ | ✅ (View) | ✅ (View/Edit) | ✅ (Full) |
| Keycloak Admin | ❌ | ❌ | ❌ | ✅ |
| Database | ❌ | ❌ | ❌ | ✅ |

---

## 9. Review Schedule

- **Quarterly**: Full access control review
- **Monthly**: User role assignments review
- **Weekly**: Security event review
- **After Changes**: Immediate review of affected components

---

**Status**: Active  
**Last Review**: January 15, 2026  
**Next Review**: April 15, 2026
