# Authentication Guide
**Last Updated**: January 15, 2026  
**Authentication Provider**: Keycloak (OAuth2/OIDC)

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication Methods](#authentication-methods)
3. [Getting Access Tokens](#getting-access-tokens)
4. [Using Tokens](#using-tokens)
5. [Token Refresh](#token-refresh)
6. [Troubleshooting](#troubleshooting)

---

## Overview

The Lianel platform uses Keycloak for authentication and authorization. All API endpoints (except health checks) require a valid OAuth2 access token.

### Authentication Flow

```
User/Application → Keycloak → Access Token → API Request → Protected Resource
```

### Keycloak Configuration

- **Realm**: `lianel`
- **Base URL**: `https://auth.lianel.se`
- **Protocol**: OpenID Connect (OIDC)
- **Supported Flows**: Authorization Code, Client Credentials

---

## Authentication Methods

### 1. Client Credentials Flow (Server-to-Server)

**Use Case**: Server applications, scripts, automated processes

**Steps**:
1. Request token from Keycloak
2. Use token in API requests
3. Refresh token before expiration

**Example**:
```bash
curl -X POST "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=your-client-id" \
  -d "client_secret=your-client-secret" \
  -d "grant_type=client_credentials" \
  -d "scope=openid profile email"
```

**Response**:
```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJ...",
  "expires_in": 300,
  "refresh_expires_in": 1800,
  "token_type": "Bearer",
  "scope": "openid profile email"
}
```

---

### 2. Authorization Code Flow (Web Applications)

**Use Case**: Web applications, user-facing services

**Steps**:
1. Redirect user to Keycloak login
2. User authenticates
3. Keycloak redirects back with authorization code
4. Exchange code for access token
5. Use token in API requests

**Example**:
```javascript
// Step 1: Redirect to Keycloak
const authUrl = `https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth?` +
  `client_id=your-client-id&` +
  `redirect_uri=${encodeURIComponent('https://your-app.com/callback')}&` +
  `response_type=code&` +
  `scope=openid profile email&` +
  `state=${randomState}`;

window.location.href = authUrl;

// Step 2: After callback, exchange code for token
const tokenResponse = await fetch('https://auth.lianel.se/realms/lianel/protocol/openid-connect/token', {
  method: 'POST',
  headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
  body: new URLSearchParams({
    grant_type: 'authorization_code',
    client_id: 'your-client-id',
    client_secret: 'your-client-secret',
    code: codeFromCallback,
    redirect_uri: 'https://your-app.com/callback'
  })
});
```

---

### 3. Resource Owner Password Credentials (Not Recommended)

**Note**: This flow is disabled for security reasons. Use Authorization Code or Client Credentials instead.

---

## Getting Access Tokens

### Python Example

```python
import requests

def get_access_token(client_id, client_secret):
    """Get access token using client credentials."""
    token_url = "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token"
    
    data = {
        "client_id": client_id,
        "client_secret": client_secret,
        "grant_type": "client_credentials",
        "scope": "openid profile email"
    }
    
    response = requests.post(token_url, data=data)
    response.raise_for_status()
    
    token_data = response.json()
    return token_data["access_token"], token_data["expires_in"]

# Usage
access_token, expires_in = get_access_token(
    client_id="your-client-id",
    client_secret="your-client-secret"
)
print(f"Token expires in {expires_in} seconds")
```

### JavaScript Example

```javascript
async function getAccessToken(clientId, clientSecret) {
  const response = await fetch('https://auth.lianel.se/realms/lianel/protocol/openid-connect/token', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded'
    },
    body: new URLSearchParams({
      client_id: clientId,
      client_secret: clientSecret,
      grant_type: 'client_credentials',
      scope: 'openid profile email'
    })
  });

  if (!response.ok) {
    throw new Error(`Token request failed: ${response.statusText}`);
  }

  const data = await response.json();
  return {
    accessToken: data.access_token,
    expiresIn: data.expires_in,
    refreshToken: data.refresh_token
  };
}
```

---

## Using Tokens

### Making Authenticated Requests

**Python**:
```python
import requests

headers = {
    "Authorization": f"Bearer {access_token}",
    "Content-Type": "application/json"
}

response = requests.get(
    "https://www.lianel.se/api/v1/datasets/forecasting",
    headers=headers
)
```

**JavaScript**:
```javascript
const response = await fetch('https://www.lianel.se/api/v1/datasets/forecasting', {
  headers: {
    'Authorization': `Bearer ${accessToken}`,
    'Content-Type': 'application/json'
  }
});
```

**cURL**:
```bash
curl -X GET "https://www.lianel.se/api/v1/datasets/forecasting" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json"
```

---

## Token Refresh

### Automatic Token Refresh

```python
import time
from datetime import datetime, timedelta

class TokenManager:
    def __init__(self, client_id, client_secret):
        self.client_id = client_id
        self.client_secret = client_secret
        self.token = None
        self.expires_at = None
    
    def get_token(self):
        """Get valid token, refreshing if needed."""
        if self.token and self.expires_at and datetime.now() < self.expires_at:
            return self.token
        
        # Refresh token
        token_url = "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token"
        data = {
            "client_id": self.client_id,
            "client_secret": self.client_secret,
            "grant_type": "client_credentials"
        }
        
        response = requests.post(token_url, data=data)
        token_data = response.json()
        
        self.token = token_data["access_token"]
        # Refresh 30 seconds before expiration
        self.expires_at = datetime.now() + timedelta(seconds=token_data["expires_in"] - 30)
        
        return self.token

# Usage
token_manager = TokenManager("your-client-id", "your-client-secret")
token = token_manager.get_token()  # Automatically refreshes if needed
```

---

## Troubleshooting

### Common Issues

#### 1. "Invalid client credentials"
- **Cause**: Incorrect client_id or client_secret
- **Solution**: Verify credentials in Keycloak admin console

#### 2. "Token expired"
- **Cause**: Token has expired (default: 5 minutes)
- **Solution**: Implement token refresh logic

#### 3. "Invalid token"
- **Cause**: Token format incorrect or revoked
- **Solution**: Request a new token

#### 4. "Insufficient permissions"
- **Cause**: Token doesn't have required scopes
- **Solution**: Request appropriate scopes during token request

### Debugging

#### Check Token Validity

```bash
# Decode JWT token (without verification)
echo $TOKEN | cut -d. -f2 | base64 -d 2>/dev/null | jq

# Check token expiration
echo $TOKEN | cut -d. -f2 | base64 -d 2>/dev/null | jq '.exp'
```

#### Test Token

```bash
# Test token with API call
curl -X GET "https://www.lianel.se/api/v1/health" \
  -H "Authorization: Bearer $TOKEN"

# If 401, token is invalid
# If 200, token is valid
```

---

## Security Best Practices

### 1. Token Storage
- ✅ Store tokens in memory (not in files)
- ✅ Use environment variables for client secrets
- ✅ Never commit tokens to version control
- ✅ Use secure storage (keychain, vault) for production

### 2. Token Transmission
- ✅ Always use HTTPS
- ✅ Include token in Authorization header (not URL)
- ✅ Use secure cookie storage for web apps

### 3. Token Lifecycle
- ✅ Refresh tokens before expiration
- ✅ Revoke tokens when no longer needed
- ✅ Monitor token usage for anomalies

### 4. Error Handling
- ✅ Handle 401 errors gracefully
- ✅ Implement automatic token refresh
- ✅ Log authentication failures (without tokens)

---

## Client Registration

To get client credentials:

1. **Access Keycloak Admin Console**: `https://auth.lianel.se`
2. **Navigate to**: Realm → Clients → Create Client
3. **Configure**:
   - Client ID: Your application name
   - Client Protocol: `openid-connect`
   - Access Type: `confidential` (for server apps) or `public` (for web apps)
   - Valid Redirect URIs: Your application callback URLs
4. **Save and note**:
   - Client ID
   - Client Secret (shown only once)

---

## API Endpoints

### Keycloak Endpoints

- **Token Endpoint**: `https://auth.lianel.se/realms/lianel/protocol/openid-connect/token`
- **Authorization Endpoint**: `https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth`
- **User Info Endpoint**: `https://auth.lianel.se/realms/lianel/protocol/openid-connect/userinfo`
- **Logout Endpoint**: `https://auth.lianel.se/realms/lianel/protocol/openid-connect/logout`

### API Endpoints

- **Base URL**: `https://www.lianel.se/api/v1`
- **Health Check**: `GET /health` (no auth required)
- **All other endpoints**: Require Bearer token

---

**Status**: Active  
**Last Review**: January 15, 2026
