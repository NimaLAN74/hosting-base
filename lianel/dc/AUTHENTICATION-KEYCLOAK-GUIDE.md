# Keycloak 26.4.6 Authentication & Logout Guide

**Status**: ✅ WORKING (December 11, 2025)
**Last Updated**: 2025-12-11

## Root Cause Analysis & Solution

### The Problem
The Keycloak logout endpoint was returning **400 Bad Request** with `invalid_redirect_uri` error, preventing users from logging out.

### Root Cause (Found via Deep Debugging)
The **keycloak-js** JavaScript library constructs logout requests with the parameter `post_logout_redirect_uri`, but **Keycloak 26.4.6's logout endpoint expects `redirect_uri`**. This is a version incompatibility between keycloak-js and Keycloak 26.4.6.

**Proof:**
- Test with `post_logout_redirect_uri` → **400 (Bad Request)**
- Test with `redirect_uri` → **200 (OK)**

### Solution
Instead of relying on `keycloak.logout({redirectUri})` from the keycloak-js library, we manually construct the logout URL with the correct parameter name:

```javascript
const logoutUrl = `${keycloakConfig.url}/realms/${keycloakConfig.realm}/protocol/openid-connect/logout?redirect_uri=${encodeURIComponent(redirectUri)}`;
window.location.href = logoutUrl;
```

## Keycloak 26.4.6 Setup (From Scratch)

### Prerequisites
- PostgreSQL running on `172.17.0.1:5432`
- Docker network `lianel-network` (bridge)

### Step 1: Create Keycloak Realm

```bash
# Get admin token
TOKEN=$(curl -sk -X POST "http://172.18.0.4:8080/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=admin-cli&username=admin&password=<KEYCLOAK_ADMIN_PASSWORD>&grant_type=password" \
  2>/dev/null | python3 -c "import sys, json; print(json.load(sys.stdin)['access_token'])")

# Create realm
curl -sk -X POST "http://172.18.0.4:8080/admin/realms" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "realm": "lianel",
    "enabled": true,
    "displayName": "Lianel EU Energy Platform"
  }'
```

### Step 2: Create Frontend Client

```bash
curl -sk -X POST "http://172.18.0.4:8080/admin/realms/lianel/clients" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "frontend-client",
    "name": "Frontend Client",
    "enabled": true,
    "publicClient": true,
    "standardFlowEnabled": true,
    "directAccessGrantsEnabled": true,
    "redirectUris": [
      "https://lianel.se",
      "https://lianel.se/"
    ],
    "webOrigins": ["*"],
    "frontchannelLogout": true,
    "attributes": {
      "backchannel.logout.session.required": "true",
      "backchannel.logout.revoke.offline.tokens": "false",
      "pkce.code.challenge.method": "S256"
    }
  }'
```

**Key Configuration Points:**
- `redirectUris`: Must include BOTH versions (with and without trailing slash) to handle all cases
- `publicClient: true`: Required for public web applications
- `standardFlowEnabled: true`: Required for OAuth 2.0 authorization code flow
- `webOrigins: ["*"]`: Allow all origins (adjust for production)
- `frontchannelLogout: true`: Enable front-channel logout
- `pkce.code.challenge.method: S256`: Required for security

### Step 3: Create Test User

```bash
# Create test user
curl -sk -X POST "http://172.18.0.4:8080/admin/realms/lianel/users" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "email": "testuser@lianel.se",
    "firstName": "Test",
    "lastName": "User",
    "enabled": true,
    "emailVerified": true
  }'

# Set password (replace USER_ID with the returned ID)
curl -sk -X PUT "http://172.18.0.4:8080/admin/realms/lianel/users/{USER_ID}/reset-password" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "type": "password",
    "value": "TestPass123",
    "temporary": false
  }'
```

## Frontend Integration

### Keycloak Configuration (keycloak.js)

```javascript
const keycloakConfig = {
  url: 'https://auth.lianel.se',
  realm: 'lianel',
  clientId: 'frontend-client',
  redirectUri: window.location.origin + window.location.pathname
};
```

### Login Function
```javascript
export const login = () => {
  keycloak.login({
    redirectUri: window.location.origin + '/',
    prompt: 'login'
  });
};
```

### Logout Function (FIXED)
```javascript
export const logout = () => {
  const redirectUri = window.location.origin === 'https://www.lianel.se' 
    ? 'https://www.lianel.se/' 
    : 'https://lianel.se/';
  
  try {
    keycloak.clearToken();
    sessionStorage.clear();
    localStorage.clear();
  } catch (e) {
    console.error('Error clearing storage:', e);
  }
  
  // Use correct parameter name: redirect_uri (not post_logout_redirect_uri)
  const logoutUrl = `${keycloakConfig.url}/realms/${keycloakConfig.realm}/protocol/openid-connect/logout?redirect_uri=${encodeURIComponent(redirectUri)}`;
  window.location.href = logoutUrl;
};
```

## Docker Deployment

### Environment Variables (.env)
```bash
KEYCLOAK_ADMIN_USER=admin
KEYCLOAK_ADMIN_PASSWORD=<SECURE_PASSWORD>
KEYCLOAK_DB_USER=keycloak
KEYCLOAK_DB_PASSWORD=<SECURE_PASSWORD>
```

### Docker Compose (docker-compose.infra.yaml)
```yaml
keycloak:
  image: quay.io/keycloak/keycloak:26.4.6
  env_file:
    - .env
  environment:
    KC_DB: postgres
    KC_DB_URL: jdbc:postgresql://172.17.0.1:5432/keycloak
    KC_DB_USERNAME: ${KEYCLOAK_DB_USER}
    KC_DB_PASSWORD: ${KEYCLOAK_DB_PASSWORD}
    KC_HOSTNAME: auth.lianel.se
    KC_HOSTNAME_STRICT: "false"
    KC_PROXY_HEADERS: xforwarded
    KC_HTTP_ENABLED: "true"
    KEYCLOAK_ADMIN: ${KEYCLOAK_ADMIN_USER}
    KEYCLOAK_ADMIN_PASSWORD: ${KEYCLOAK_ADMIN_PASSWORD}
  command: start
  networks:
    - lianel-network
```

## Testing the Authentication Flow

### 1. Login Test
```bash
# Open browser and navigate to:
https://lianel.se/

# Click "Login" button
# Enter: testuser / TestPass123
# Should redirect to dashboard after successful authentication
```

### 2. Logout Test
```bash
# Click logout button
# Should redirect to landing page WITHOUT 400 error
# Check browser console - should see:
# "Logging out with correct parameter: redirect_uri"
```

### 3. Direct Keycloak Tests
```bash
# Test with correct parameter (should return 200)
curl -sk -w "\nStatus: %{http_code}\n" \
  "https://auth.lianel.se/realms/lianel/protocol/openid-connect/logout?redirect_uri=https://lianel.se/"

# Test with wrong parameter (keycloak-js default, should return 400)
curl -sk -w "\nStatus: %{http_code}\n" \
  "https://auth.lianel.se/realms/lianel/protocol/openid-connect/logout?post_logout_redirect_uri=https://lianel.se/"
```

## Common Issues & Solutions

### Issue: Login returns 400 with invalid_redirect_uri
**Solution**: Ensure both redirect URI variants are in client config:
- `https://lianel.se` (without slash)
- `https://lianel.se/` (with slash)

### Issue: Logout still returns 400
**Solution**: Verify the logout URL uses `redirect_uri` parameter, not `post_logout_redirect_uri`

### Issue: HTTPS not working
**Solution**: Ensure nginx is properly configured with:
```
KC_HOSTNAME: auth.lianel.se
KC_PROXY_HEADERS: xforwarded
```

### Issue: Database connection fails
**Solution**: Verify PostgreSQL is running and accessible:
```bash
psql -h 172.17.0.1 -U keycloak -d keycloak
```

## Security Notes

⚠️ **Important for Production:**
- Change `webOrigins` from `["*"]` to specific domain
- Use strong passwords for admin and database users
- Enable HTTPS for all endpoints
- Consider implementing OTP/MFA
- Set up proper logging and monitoring
- Regularly backup Keycloak database

## Files Modified

- `lianel/dc/frontend/src/keycloak.js` - Fixed logout function
- `lianel/dc/docker-compose.infra.yaml` - Keycloak configuration
- `.env` - Keycloak credentials

## References

- [Keycloak 26.4.6 Documentation](https://www.keycloak.org/documentation)
- [OpenID Connect Logout Endpoint Spec](https://openid.net/specs/openid-connect-rpinitiated-1_0.html)
- [keycloak-js GitHub](https://github.com/keycloak/keycloak-js)
