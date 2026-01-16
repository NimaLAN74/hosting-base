# Admin Access Issue - Debug Guide

## Current Status

- ✅ Backend API works: Returns `{"isAdmin": true}` for admin user
- ✅ Keycloak configured: Roles scope assigned, admin role assigned
- ✅ Token contains admin role: Verified in decoded token
- ❌ Frontend not showing admin services: Still an issue

## Debugging Steps

### 1. Check Browser Console

Open browser console (F12) and look for:
```
=== DASHBOARD ADMIN ROLE DEBUG ===
Dashboard - Backend API isAdmin: <true/false>
Dashboard - API check loading: <true/false>
Dashboard - Token roles: [...]
Dashboard - Final isAdmin: <true/false>
```

**If `Backend API isAdmin: false`**:
- Check Network tab for `/api/admin/check` request
- Verify request has `Authorization: Bearer <token>` header
- Check response status code and body

**If `Backend API isAdmin: true` but services not visible**:
- Check `finalIsAdmin` value
- Verify `adminServices` array is populated
- Check for JavaScript errors

### 2. Check Network Requests

1. Open Network tab in browser DevTools
2. Filter by "admin"
3. Find `/api/admin/check` request
4. Check:
   - Request headers (Authorization token present?)
   - Response status (200?)
   - Response body (`{"isAdmin": true}`?)

### 3. Verify Token

Decode the JWT token:
1. Copy token from browser console: `keycloak.token`
2. Go to https://jwt.io
3. Paste token
4. Verify `realm_access.roles` contains `"admin"`

### 4. Check Frontend Build

If frontend was recently updated, verify:
1. Frontend container is running latest build
2. Browser cache is cleared (hard refresh: Ctrl+Shift+R)
3. No JavaScript errors in console

### 5. Force Token Refresh

If token doesn't have roles:
1. User must completely log out
2. Clear browser cache/cookies
3. Log back in
4. Check new token has roles

## Common Issues

### Issue 1: API Returns 401/403
**Cause**: Token not sent or invalid
**Fix**: Check `authenticatedFetch` is including token in headers

### Issue 2: API Returns isAdmin: false
**Cause**: Backend can't verify admin role
**Fix**: Check user has admin role in Keycloak, verify backend can access Keycloak

### Issue 3: API Works but Frontend Doesn't Show Services
**Cause**: Frontend state not updating
**Fix**: Check React state updates, verify `finalIsAdmin` calculation

### Issue 4: Token Doesn't Have Roles
**Cause**: User didn't log out/in after role assignment
**Fix**: User must log out completely and log back in

## Quick Fix Commands

```bash
# Check admin API directly
curl "https://www.lianel.se/api/admin/check" \
  -H "Authorization: Bearer <user-token>"

# Verify user has admin role in Keycloak
# (Run on remote host)
cd /root/lianel/dc
source .env
TOKEN=$(curl -s -X POST "https://auth.lianel.se/realms/master/protocol/openid-connect/token" \
  -d "username=${KEYCLOAK_ADMIN_USER}" \
  -d "password=${KEYCLOAK_ADMIN_PASSWORD}" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | python3 -c "import sys, json; print(json.load(sys.stdin)['access_token'])")

USER_ID=$(curl -s "https://auth.lianel.se/admin/realms/lianel/users?username=admin" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; users = json.load(sys.stdin); print(users[0]['id']) if users else exit(1)")

curl -s "https://auth.lianel.se/admin/realms/lianel/users/${USER_ID}/role-mappings/realm" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; roles = json.load(sys.stdin); print([r['name'] for r in roles] if roles else [])"
```

## Next Steps if Still Not Working

1. **Check frontend logs**: `docker logs lianel-frontend`
2. **Rebuild frontend**: May need to rebuild with latest changes
3. **Check Nginx**: Verify Nginx is proxying `/api/admin/check` correctly
4. **Test with curl**: Verify API works from command line
5. **Check CORS**: Verify CORS headers allow frontend origin
