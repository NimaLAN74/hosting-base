# Swagger UI 502 & Admin Login Fix

## 🔴 Issues Found

1. **Swagger UI 502 Error**
   - Nginx was still configured to use port 9000
   - Profile service is running on port 3000
   - Caused "Connection refused" errors

2. **Admin Login Invalid**
   - Need to clarify which admin credentials to use
   - Different credentials for different services

## ✅ Fixes Applied

### 1. Nginx Configuration
- Updated all `profile_service_upstream` references from port 9000 to 3000
- Updated in 4 locations:
  - Swagger UI static assets
  - Swagger UI main page
  - OpenAPI JSON endpoint
  - API endpoints (`/api/`)

### 2. Admin Credentials Clarification

**Keycloak Admin Console** (https://auth.lianel.se):
- Username: `admin`
- Password: `D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA`

**Frontend Login** (https://lianel.se):
- Username: `admin`
- Password: `Admin123!Secure`

**Airflow** (https://airflow.lianel.se):
- Uses Keycloak SSO
- Username: `admin`
- Password: `Admin123!Secure`

## 🧪 Testing

### Swagger UI
1. Visit: https://lianel.se/swagger-ui
2. Should load Swagger UI (no 502 error)
3. Can browse API endpoints

### Admin Login
1. Visit: https://lianel.se
2. Click "Login"
3. Use credentials:
   - Username: `admin`
   - Password: `Admin123!Secure`
4. Should successfully authenticate

## 📋 Configuration Details

### Profile Service
- **Port**: 3000 (internal and exposed)
- **Health Check**: http://lianel-profile-service:3000/health
- **Swagger UI**: http://lianel-profile-service:3000/swagger-ui

### Nginx
- **Upstream**: `http://lianel-profile-service:3000`
- **Routes**:
  - `/swagger-ui` → Profile service Swagger UI
  - `/api/` → Profile service API
  - `/api-doc/openapi.json` → OpenAPI spec

## 🔧 Verification

```bash
# Check nginx config
docker exec nginx-proxy nginx -t

# Test profile service directly
docker exec nginx-proxy curl http://lianel-profile-service:3000/health

# Test swagger-ui via nginx
curl -k https://lianel.se/swagger-ui
```

---

**Status**: ✅ Fixed - Nginx updated to use port 3000, admin credentials documented

