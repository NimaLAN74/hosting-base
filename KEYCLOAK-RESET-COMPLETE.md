# ✅ Keycloak & SSO Reset Complete

## 🔧 What Was Reset

1. ✅ **Deleted existing 'lianel' realm** (removed all old configuration)
2. ✅ **Created new 'lianel' realm** with proper settings
3. ✅ **Created 'admin' realm role**
4. ✅ **Created OAuth2 Proxy client** (for SSO)
5. ✅ **Created Frontend client** (public client for web app)
6. ✅ **Created Backend API client** (for profile service)
7. ✅ **Created Grafana client** (for monitoring SSO)
8. ✅ **Created admin user** with admin role
9. ✅ **Created test user**

## 👤 Admin Credentials

- **Username**: `admin`
- **Password**: `D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA`
- **Realm**: `lianel`
- **Login URL**: https://lianel.se

## 🧪 Test User

- **Username**: `testuser`
- **Password**: `Test123!`
- **Realm**: `lianel`

## 📋 Next Steps

### 1. Get Client Secrets

Run on remote host:
```bash
cd /root/lianel/dc
./get-client-secrets.sh
```

This will output the client secrets needed for `.env` file.

### 2. Update .env File

Update `/root/lianel/dc/.env` with the new client secrets:
- `OAUTH2_CLIENT_SECRET=...`
- `BACKEND_CLIENT_SECRET=...`
- `GRAFANA_OAUTH_CLIENT_SECRET=...`

### 3. Restart Services

```bash
cd /root/lianel/dc
docker compose -f docker-compose.oauth2-proxy.yaml restart oauth2-proxy
docker compose -f docker-compose.backend.yaml restart profile-service
```

### 4. Test Login

1. Visit: https://lianel.se
2. Click "Login"
3. Use admin credentials above
4. Should redirect back with admin access

## 🔍 Verify Setup

Check that everything is working:
```bash
# Check Keycloak realm
curl -sk https://auth.lianel.se/realms/lianel/.well-known/openid-configuration | jq '.issuer'

# Test admin login
curl -sk -X POST "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token" \
  -d "username=admin" \
  -d "password=D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA" \
  -d "grant_type=password" \
  -d "client_id=frontend-client" | jq '.access_token'
```

## 📝 Scripts Available

- `/root/lianel/dc/reset-keycloak-sso.sh` - Complete reset script
- `/root/lianel/dc/get-client-secrets.sh` - Get current client secrets

---

**Status**: ✅ Reset complete! Update .env and restart services.

