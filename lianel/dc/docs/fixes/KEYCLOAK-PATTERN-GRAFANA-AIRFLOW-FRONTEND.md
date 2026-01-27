# Keycloak Pattern: Grafana, Airflow, and Frontend

**Purpose**: Document how Keycloak is used by working services so the frontend matches the same pattern. The frontend login worked before the "CORS/COMP AI" change that switched it to `www.lianel.se/auth`; it should use **https://auth.lianel.se** like Grafana and Airflow.

---

## Working Pattern: Grafana and Airflow

### Grafana (`monitoring/grafana/grafana.ini`)

- **Browser auth**: `auth_url = https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth`
- **Token/userinfo** (server-side): `token_url = http://keycloak:8080/...`, `api_url = http://keycloak:8080/...`
- **Flow**: User clicks Login → browser goes to **auth.lianel.se** → Keycloak login → redirect back to `https://monitoring.lianel.se/login/generic_oauth` (callback). Grafana backend then uses `keycloak:8080` for token/userinfo.
- **Key point**: The **authorization URL** the browser is sent to is **https://auth.lianel.se** (Keycloak’s public URL). No same-origin proxy.

### Airflow (`config/webserver_config.py`)

- **Auth URLs**: `api_base_url`, `access_token_url`, `authorize_url` = **https://auth.lianel.se/realms/lianel/protocol/openid-connect/...**
- **Callback**: `OAUTH_REDIRECT_URI = 'https://airflow.lianel.se/oauth-authorized/keycloak'`
- **Flow**: User clicks Login → browser goes to **auth.lianel.se** → Keycloak login → redirect to airflow.lianel.se/oauth-authorized/keycloak. Airflow backend uses auth.lianel.se for token/userinfo.
- **Key point**: All OAuth URLs used by Airflow point at **https://auth.lianel.se**. No www.lianel.se/auth proxy.

### Comp-AI service (`docker-compose.comp-ai.yaml`, `comp-ai-service/src/config.rs`)

- **Keycloak URL**: `KEYCLOAK_URL: https://auth.lianel.se`
- Used for token validation / introspect; no browser flow. Same pattern: Keycloak = **auth.lianel.se**.

---

## Documented Working Frontend (AUTHENTICATION-KEYCLOAK-GUIDE.md, Dec 2025)

- **keycloakConfig.url**: `'https://auth.lianel.se'`
- **KC_HOSTNAME**: `auth.lianel.se`
- **Flow**: User on lianel.se → Login → **auth.lianel.se** → Keycloak → redirect back to lianel.se.

So the **originally working** frontend also used **https://auth.lianel.se**, not a proxy.

---

## What Broke: “CORS / COMP AI” Change

To address CORS/NetworkError on keycloak-js init, the frontend was switched to:

- `REACT_APP_KEYCLOAK_URL = https://www.lianel.se/auth` (same-origin proxy)
- keycloak.js `getKeycloakUrl()` returning `origin + '/auth'` or `https://www.lianel.se/auth`

**Effect**: Login redirect started sending users to **auth.lianel.se/admin/master/console/** instead of back to the app. Grafana and Airflow were never changed to use www.lianel.se/auth and kept using auth.lianel.se; they continued to work.

---

## Correct Pattern for Frontend

To match Grafana, Airflow, and the AUTHENTICATION-KEYCLOAK-GUIDE:

1. **Frontend Keycloak URL** = **https://auth.lianel.se** (same as Grafana `auth_url` host, Airflow `authorize_url` host).
2. **keycloak.js**  
   - `getKeycloakUrl()` should return `https://auth.lianel.se` when no env is set (and respect `REACT_APP_KEYCLOAK_URL` if set to auth.lianel.se).
3. **Build/deploy**  
   - Default `REACT_APP_KEYCLOAK_URL` to **https://auth.lianel.se** in docker-compose.frontend, .env.example, and the deploy workflow (same as other services).
4. **Keycloak**  
   - KC_HOSTNAME remains **auth.lianel.se**  
   - frontend-client **redirect URIs** and **webOrigins** must include https://www.lianel.se, https://lianel.se (and variants) so redirects and CORS work. Scripts like `update-keycloak-frontend-client.sh` already maintain these.
5. **CORS**  
   - If keycloak-js still hits CORS on auth.lianel.se, fix it via Keycloak/frontend-client **webOrigins** and, if needed, proxy/CORS on auth.lianel.se, **not** by moving the frontend to www.lianel.se/auth (which caused the redirect-to-admin regression).

---

## Summary

| Service    | Keycloak URL (auth / OAuth) | Source |
|-----------|-----------------------------|--------|
| Grafana   | https://auth.lianel.se      | grafana.ini `auth_url` |
| Airflow   | https://auth.lianel.se      | webserver_config.py `authorize_url` etc. |
| Comp-AI   | https://auth.lianel.se      | docker-compose.comp-ai, config |
| Frontend  | **https://auth.lianel.se**  | AUTHENTICATION-KEYCLOAK-GUIDE, revert from www.lianel.se/auth |

Frontend should use **https://auth.lianel.se** like the other services. The www.lianel.se/auth proxy was introduced for CORS and caused the login redirect bug; the correct fix is to keep auth.lianel.se and resolve CORS via Keycloak client config (webOrigins) and/or server CORS, not by changing the frontend’s Keycloak URL.
