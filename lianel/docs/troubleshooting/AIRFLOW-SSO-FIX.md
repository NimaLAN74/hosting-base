# Airflow SSO Login Fix

## Issue
Airflow was showing login form instead of OAuth login button.

## Root Cause
Airflow OAuth configuration was correct, but may need:
1. Full restart of Airflow services
2. Browser cache cleared
3. Verification of client secret in environment

## Configuration
Airflow OAuth is configured in `/opt/airflow/config/webserver_config.py`:
- `AUTH_TYPE = AUTH_OAUTH`
- Client ID: `airflow`
- Client Secret: From `AIRFLOW_OAUTH_CLIENT_SECRET` env var
- Authorization URL: `https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth`
- Token URL: `https://auth.lianel.se/realms/lianel/protocol/openid-connect/token`
- Redirect URI: `https://airflow.lianel.se/oauth-authorized/keycloak`

## Keycloak Client
- Client ID: `airflow`
- Enabled: `true`
- Standard Flow Enabled: `true`
- Redirect URIs: `https://airflow.lianel.se/oauth-authorized/keycloak`

## Fix Steps
1. **Verify client secret in `.env`**:
   ```bash
   grep AIRFLOW_OAUTH_CLIENT_SECRET /root/lianel/dc/.env
   ```

2. **Restart Airflow services**:
   ```bash
   cd /root/lianel/dc
   docker compose -f docker-compose.airflow.yaml restart airflow-apiserver
   ```

3. **Clear browser cache** or use incognito mode

4. **Check Airflow logs**:
   ```bash
   docker logs dc-airflow-apiserver-1 | grep -i oauth
   ```

## Testing
1. Visit: `https://airflow.lianel.se`
2. Should see "Sign in with Keycloak" button instead of login form
3. Click button → Redirects to Keycloak
4. Login → Redirects back to Airflow

## Troubleshooting
If still showing login form:
1. Check `webserver_config.py` is loaded:
   ```bash
   docker exec dc-airflow-apiserver-1 cat /opt/airflow/config/webserver_config.py | grep AUTH_TYPE
   ```

2. Verify environment variable:
   ```bash
   docker exec dc-airflow-apiserver-1 env | grep AIRFLOW_OAUTH
   ```

3. Check Keycloak client:
   - Client exists and is enabled
   - Standard Flow is enabled
   - Redirect URI matches exactly

4. Full restart (if needed):
   ```bash
   docker compose -f docker-compose.airflow.yaml down
   docker compose -f docker-compose.airflow.yaml up -d
   ```

## Status
✅ Configuration verified
✅ Airflow services restarted
⏳ Waiting for user to test in browser

