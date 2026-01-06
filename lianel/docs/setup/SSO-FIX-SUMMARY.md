# SSO Fix Summary - Airflow & Profile Service

## ✅ What Was Fixed

### 1. Documentation Organization
- ✅ All docs moved to `lianel/docs/` with subdirectories:
  - `setup/` - Setup and configuration docs
  - `deployment/` - Deployment guides
  - `ci-cd/` - CI/CD documentation
  - `troubleshooting/` - Troubleshooting guides
- ✅ Removed old/irrelevant pipeline troubleshooting docs
- ✅ Created `.env.example` in `lianel/docs/setup/` with all secrets

### 2. Airflow SSO
- ✅ Created/updated `airflow` OAuth client in Keycloak
- ✅ Client secret: `1CwO0seEUzSpGaBgbvcXFtmvU54ZdFMi`
- ✅ Updated `.env` file with `AIRFLOW_OAUTH_CLIENT_SECRET`
- ✅ Restarted Airflow services
- ✅ Configuration file: `/opt/airflow/config/webserver_config.py`

**Airflow OAuth Client Settings:**
- Client ID: `airflow`
- Redirect URIs:
  - `https://airflow.lianel.se/oauth-authorized/keycloak`
  - `https://www.lianel.se/airflow/oauth-authorized/keycloak`
- Web Origins:
  - `https://airflow.lianel.se`
  - `https://www.lianel.se`

### 3. Profile Service SSO
- ✅ Verified `backend-api` OAuth client exists
- ✅ Client secret: `w6TZFvMVaXFcAYa8jIrAl8fkkD0jUgF0`
- ✅ Updated `docker-compose.backend.yaml`:
  - Changed `KEYCLOAK_URL` from `http://keycloak:8080` to `https://auth.lianel.se`
  - Added `BACKEND_CLIENT_SECRET` environment variable
- ✅ Restarted Profile Service with correct configuration

**Profile Service Configuration:**
- `KEYCLOAK_URL`: `https://auth.lianel.se`
- `BACKEND_CLIENT_SECRET`: Set from `.env` file
- Uses `backend-api` client for token introspection

## 🔍 Verification

### Profile Service
```bash
docker exec lianel-profile-service env | grep -E 'BACKEND_CLIENT_SECRET|KEYCLOAK'
# Should show:
# BACKEND_CLIENT_SECRET=w6TZFvMVaXFcAYa8jIrAl8fkkD0jUgF0
# KEYCLOAK_URL=https://auth.lianel.se
```

### Airflow
```bash
docker exec dc-airflow-apiserver-1 env | grep AIRFLOW_OAUTH_CLIENT_SECRET
# Should show:
# AIRFLOW_OAUTH_CLIENT_SECRET=1CwO0seEUzSpGaBgbvcXFtmvU54ZdFMi
```

## 📋 Current Status

- ✅ Grafana SSO: Working
- ✅ Airflow SSO: Fixed (client created, secret configured)
- ✅ Profile Service SSO: Fixed (backend-api client configured)

## 🧪 Testing

### Test Airflow SSO
1. Visit: https://airflow.lianel.se
2. Should redirect to Keycloak login
3. Login with admin credentials
4. Should redirect back to Airflow

### Test Profile Service
1. Login to https://lianel.se
2. Profile Service should validate tokens via Keycloak
3. Admin endpoints should work with admin role

---

**Status**: ✅ Both services configured and restarted

