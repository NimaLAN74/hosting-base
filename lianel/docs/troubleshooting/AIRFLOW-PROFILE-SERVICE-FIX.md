# Airflow & Profile Service Fix

## 🔴 Issues Found

1. **Profile Service Port Mismatch**
   - Docker compose configured port 9000
   - Service actually running on port 3000
   - Caused connection/routing issues

2. **Airflow OAuth Not Activating**
   - OAuth config file exists and is correct
   - But Airflow showing login form instead of OAuth redirect
   - Needed full restart to pick up OAuth configuration

## ✅ Fixes Applied

### 1. Profile Service Port Fix
- Updated `docker-compose.yaml` to use port 3000 (matching actual service)
- Added `BACKEND_CLIENT_SECRET` environment variable
- Restarted service with correct configuration

### 2. Airflow OAuth Fix
- Verified `webserver_config.py` is correctly configured
- Verified `AIRFLOW_OAUTH_CLIENT_SECRET` is set correctly
- Performed full restart of Airflow services to activate OAuth

## 🔍 Verification

### Profile Service
```bash
docker exec lianel-profile-service env | grep PORT
# Should show: PORT=3000

docker logs lianel-profile-service | tail -5
# Should show: "Profile service listening on 0.0.0.0:3000"
```

### Airflow
```bash
docker exec dc-airflow-apiserver-1 env | grep AIRFLOW_OAUTH_CLIENT_SECRET
# Should show the secret

docker exec dc-airflow-apiserver-1 cat /opt/airflow/config/webserver_config.py | grep AUTH_TYPE
# Should show: AUTH_TYPE = AUTH_OAUTH
```

## 🧪 Testing

### Profile Service
1. Visit: https://lianel.se/swagger-ui
2. Should load Swagger UI
3. Try `/api/profile` endpoint (requires authentication)

### Airflow
1. Visit: https://airflow.lianel.se
2. Should redirect to Keycloak login (NOT show login form)
3. After login, should redirect back to Airflow authenticated

## 📋 Configuration Details

### Profile Service
- **Port**: 3000 (internal and exposed)
- **Keycloak URL**: https://auth.lianel.se
- **Backend Client Secret**: Set from `.env` file
- **Endpoints**: `/api/profile`, `/health`, `/swagger-ui`

### Airflow
- **OAuth Provider**: Keycloak
- **Client ID**: `airflow`
- **Client Secret**: From `AIRFLOW_OAUTH_CLIENT_SECRET` env var
- **Redirect URI**: `https://airflow.lianel.se/oauth-authorized/keycloak`
- **Config File**: `/opt/airflow/config/webserver_config.py`

## 🔧 If Issues Persist

### Profile Service
- Check logs: `docker logs lianel-profile-service`
- Verify port: `docker exec lianel-profile-service env | grep PORT`
- Test health: `curl https://lianel.se/api/profile` (requires auth)

### Airflow
- Clear browser cache and cookies
- Check Airflow logs: `docker logs dc-airflow-apiserver-1 | grep -i oauth`
- Verify Keycloak client: Check redirect URIs match exactly
- Try incognito/private mode

---

**Status**: ✅ Fixed - Port corrected and Airflow OAuth activated

