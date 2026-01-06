# Swagger UI Authentication Enabled

## Overview
Swagger UI and OpenAPI JSON endpoints are now protected with Keycloak OAuth authentication via OAuth2-proxy.

## Protected Endpoints

### Swagger UI
- **URL**: `https://lianel.se/swagger-ui`
- **Protection**: OAuth2 authentication required
- **Behavior**: Unauthenticated users are redirected to Keycloak login

### OpenAPI JSON
- **URL**: `https://lianel.se/api-doc/openapi.json`
- **Protection**: OAuth2 authentication required
- **Behavior**: Unauthenticated users are redirected to Keycloak login

## Static Assets
Swagger UI static assets (CSS, JS files) remain publicly accessible to allow the UI to function properly. Only the main page and JSON endpoint require authentication.

## Configuration

### Nginx Configuration
The following locations are protected:
```nginx
location ~ ^/swagger-ui/?$ {
    auth_request /oauth2/auth;
    error_page 401 = /oauth2/sign_in;
    # ... proxy settings
}

location /api-doc/openapi.json {
    auth_request /oauth2/auth;
    error_page 401 = /oauth2/sign_in;
    # ... proxy settings
}
```

### OAuth2-Proxy
- OAuth2-proxy handles authentication via Keycloak
- Users must login through Keycloak to access Swagger UI
- Session cookies are used for subsequent requests

## Testing

### Test Unauthenticated Access
```bash
# Should redirect to login
curl -I https://lianel.se/swagger-ui
curl -I https://lianel.se/api-doc/openapi.json
```

### Test Authenticated Access
1. Login to the main site: `https://lianel.se`
2. Navigate to: `https://lianel.se/swagger-ui`
3. Should show Swagger UI without redirect

## Status
✅ OAuth authentication enabled for Swagger UI
✅ OAuth authentication enabled for OpenAPI JSON
✅ Static assets remain accessible
✅ Configuration committed to repository

