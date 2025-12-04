# Keycloak SSO Integration Guide

## Overview
This guide explains how to implement Single Sign-On (SSO) using Keycloak for all services in your infrastructure.

## Architecture

```
User → Main Site (lianel.se) → Keycloak (SSO Provider)
                                    ↓
                    ┌───────────────┼───────────────┐
                    ↓               ↓               ↓
                Airflow         Grafana         Future Services
```

## Components Needed

### 1. **Keycloak** (Identity Provider)
- Manages users, authentication, and authorization
- Provides OAuth2/OIDC endpoints
- Centralized user management

### 2. **OAuth2 Proxy** (Already configured!)
- Acts as authentication gateway
- Sits between nginx and backend services
- Validates tokens from Keycloak

### 3. **Service Integration**
Each service needs to be configured to trust Keycloak tokens

---

## Implementation Steps

### Phase 1: Fix Keycloak Database Connection

**Problem:** Keycloak is crashing due to PostgreSQL authentication failure.

**Solution:**
```yaml
# In docker-compose.oauth2-proxy.yaml, add PostgreSQL for Keycloak:

services:
  keycloak-db:
    image: postgres:15
    container_name: keycloak-db
    environment:
      POSTGRES_DB: keycloak
      POSTGRES_USER: ${KEYCLOAK_DB_USER}
      POSTGRES_PASSWORD: ${KEYCLOAK_DB_PASSWORD}
    volumes:
      - keycloak-db-data:/var/lib/postgresql/data
    networks:
      - lianel-network
    restart: unless-stopped

  keycloak:
    image: quay.io/keycloak/keycloak:latest
    container_name: keycloak
    environment:
      KC_DB: postgres
      KC_DB_URL: jdbc:postgresql://keycloak-db:5432/keycloak
      KC_DB_USERNAME: ${KEYCLOAK_DB_USER}
      KC_DB_PASSWORD: ${KEYCLOAK_DB_PASSWORD}
      KEYCLOAK_ADMIN: ${KEYCLOAK_ADMIN_USER}
      KEYCLOAK_ADMIN_PASSWORD: ${KEYCLOAK_ADMIN_PASSWORD}
      KC_HOSTNAME: auth.lianel.se
      KC_PROXY: edge
    command: start
    depends_on:
      - keycloak-db
    expose:
      - "8080"
    networks:
      - lianel-network
    restart: unless-stopped

volumes:
  keycloak-db-data:
```

### Phase 2: Configure Keycloak Realm and Clients

1. **Access Keycloak Admin Console:**
   - URL: https://auth.lianel.se
   - Login with admin credentials from .env

2. **Create Realm:**
   - Name: `lianel`
   - Display name: `Lianel Services`

3. **Create Clients for Each Service:**

   **Airflow Client:**
   ```
   Client ID: airflow
   Client Protocol: openid-connect
   Access Type: confidential
   Valid Redirect URIs: https://airflow.lianel.se/*
   Web Origins: https://airflow.lianel.se
   ```

   **Grafana Client:**
   ```
   Client ID: grafana
   Client Protocol: openid-connect
   Access Type: confidential
   Valid Redirect URIs: https://www.lianel.se/monitoring/*
   Web Origins: https://www.lianel.se
   ```

4. **Create Users:**
   - Add users in Keycloak
   - Set passwords
   - Assign roles

### Phase 3: Configure OAuth2 Proxy

Update `docker-compose.oauth2-proxy.yaml`:

```yaml
services:
  oauth2-proxy:
    image: quay.io/oauth2-proxy/oauth2-proxy:latest
    container_name: oauth2-proxy
    command:
      - --provider=keycloak-oidc
      - --client-id=oauth2-proxy
      - --client-secret=${OAUTH2_CLIENT_SECRET}
      - --oidc-issuer-url=https://auth.lianel.se/realms/lianel
      - --cookie-secret=${OAUTH2_COOKIE_SECRET}
      - --cookie-secure=true
      - --cookie-domain=.lianel.se
      - --email-domain=*
      - --upstream=static://200
      - --http-address=0.0.0.0:4180
      - --redirect-url=https://auth.lianel.se/oauth2/callback
      - --whitelist-domain=.lianel.se
      - --cookie-refresh=1h
      - --cookie-expire=4h
    expose:
      - "4180"
    networks:
      - lianel-network
    restart: unless-stopped
```

### Phase 4: Update Nginx Configuration

Add authentication layer for each service:

```nginx
# Airflow with SSO
server {
    listen 443 ssl;
    server_name airflow.lianel.se;
    
    # SSL config...
    
    # OAuth2 Proxy authentication
    location /oauth2/ {
        proxy_pass http://oauth2-proxy:4180;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    location = /oauth2/auth {
        proxy_pass http://oauth2-proxy:4180;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header Content-Length "";
        proxy_pass_request_body off;
    }
    
    location / {
        # Check authentication
        auth_request /oauth2/auth;
        error_page 401 = /oauth2/sign_in;
        
        # Pass user info to backend
        auth_request_set $user $upstream_http_x_auth_request_user;
        auth_request_set $email $upstream_http_x_auth_request_email;
        proxy_set_header X-User $user;
        proxy_set_header X-Email $email;
        
        # Proxy to Airflow
        proxy_pass http://airflow-apiserver:8080;
        # ... other proxy settings
    }
}

# Keycloak
server {
    listen 443 ssl;
    server_name auth.lianel.se;
    
    # SSL config...
    
    location / {
        proxy_pass http://keycloak:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Phase 5: Configure Grafana for Keycloak SSO

Update Grafana environment in `docker-compose.monitoring.yaml`:

```yaml
grafana:
  environment:
    # ... existing config
    - GF_AUTH_GENERIC_OAUTH_ENABLED=true
    - GF_AUTH_GENERIC_OAUTH_NAME=Keycloak
    - GF_AUTH_GENERIC_OAUTH_ALLOW_SIGN_UP=true
    - GF_AUTH_GENERIC_OAUTH_CLIENT_ID=grafana
    - GF_AUTH_GENERIC_OAUTH_CLIENT_SECRET=${GRAFANA_OAUTH_CLIENT_SECRET}
    - GF_AUTH_GENERIC_OAUTH_SCOPES=openid profile email
    - GF_AUTH_GENERIC_OAUTH_AUTH_URL=https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth
    - GF_AUTH_GENERIC_OAUTH_TOKEN_URL=https://auth.lianel.se/realms/lianel/protocol/openid-connect/token
    - GF_AUTH_GENERIC_OAUTH_API_URL=https://auth.lianel.se/realms/lianel/protocol/openid-connect/userinfo
    - GF_AUTH_GENERIC_OAUTH_ROLE_ATTRIBUTE_PATH=contains(roles[*], 'admin') && 'Admin' || contains(roles[*], 'editor') && 'Editor' || 'Viewer'
```

### Phase 6: Configure Airflow for Keycloak SSO

Add to Airflow's `webserver_config.py`:

```python
from flask_appbuilder.security.manager import AUTH_OAUTH

AUTH_TYPE = AUTH_OAUTH
OAUTH_PROVIDERS = [{
    'name': 'keycloak',
    'icon': 'fa-key',
    'token_key': 'access_token',
    'remote_app': {
        'client_id': 'airflow',
        'client_secret': os.getenv('AIRFLOW_OAUTH_CLIENT_SECRET'),
        'api_base_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect',
        'client_kwargs': {
            'scope': 'openid profile email'
        },
        'access_token_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/token',
        'authorize_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth',
        'request_token_url': None,
    }
}]
```

---

## User Flow

1. **User visits Airflow** (https://airflow.lianel.se)
2. **Nginx checks authentication** via OAuth2 Proxy
3. **Not authenticated?** → Redirect to Keycloak login
4. **User logs in** to Keycloak
5. **Keycloak issues token** and redirects back
6. **OAuth2 Proxy validates token** and sets cookie
7. **User accesses Airflow** with valid session
8. **User visits Grafana** → Already authenticated (same cookie domain)!

---

## Benefits

✅ **Single Login** - Users log in once, access all services
✅ **Centralized User Management** - Add/remove users in one place
✅ **Better Security** - Token-based authentication, no password sharing
✅ **Audit Trail** - Track who accessed what and when
✅ **Role-Based Access** - Assign different permissions per service
✅ **Session Management** - Control session timeouts centrally

---

## Required DNS Records

Add these CNAME records:
- `auth.lianel.se` → `lianel.se` (for Keycloak)

---

## Environment Variables to Add

Add to `.env`:
```bash
# OAuth2 Proxy
OAUTH2_CLIENT_SECRET=<generate-random-secret>
OAUTH2_COOKIE_SECRET=<generate-random-32-char-string>

# Grafana OAuth
GRAFANA_OAUTH_CLIENT_SECRET=<from-keycloak-grafana-client>

# Airflow OAuth
AIRFLOW_OAUTH_CLIENT_SECRET=<from-keycloak-airflow-client>
```

Generate secrets:
```bash
# For OAUTH2_CLIENT_SECRET
openssl rand -base64 32

# For OAUTH2_COOKIE_SECRET (must be 32 bytes)
python -c 'import os,base64; print(base64.urlsafe_b64encode(os.urandom(32)).decode())'
```

---

## Testing Plan

1. **Test Keycloak Access:**
   - Visit https://auth.lianel.se
   - Login with admin credentials
   - Create test user

2. **Test Airflow SSO:**
   - Visit https://airflow.lianel.se
   - Should redirect to Keycloak
   - Login with test user
   - Should redirect back to Airflow

3. **Test Grafana SSO:**
   - Visit https://www.lianel.se/monitoring/
   - Should use same session (no re-login needed)
   - Or redirect to Keycloak if not logged in

4. **Test Logout:**
   - Logout from one service
   - Should logout from all services

---

## Troubleshooting

### Keycloak won't start
- Check database connection
- Verify environment variables
- Check logs: `docker logs keycloak`

### OAuth2 Proxy errors
- Verify client secrets match Keycloak
- Check cookie domain settings
- Ensure redirect URLs are correct

### Service not redirecting
- Check nginx auth_request configuration
- Verify OAuth2 Proxy is running
- Check network connectivity

---

## Next Steps

1. **Fix Keycloak database** (create dedicated PostgreSQL)
2. **Add DNS record** for auth.lianel.se
3. **Configure Keycloak realm and clients**
4. **Update nginx configuration**
5. **Configure each service** (Grafana, Airflow)
6. **Test end-to-end flow**
7. **Create user accounts**
8. **Document for team**

---

## Alternative: Simpler Approach

If full SSO is too complex initially, you can use **OAuth2 Proxy alone** as a simpler authentication layer:

1. Users authenticate with OAuth2 Proxy (using Google, GitHub, or Keycloak)
2. OAuth2 Proxy sets a cookie valid for `.lianel.se`
3. All services check this cookie via nginx `auth_request`
4. Services themselves don't need OAuth configuration

This gives you SSO without modifying each service's configuration!
