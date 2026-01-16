# Keycloak Admin Role Authentication - Deep Analysis & Fix

## Problem Statement

Admin users cannot see admin services (Airflow, Grafana) on the frontend portal even though:
- Admin role is assigned in Keycloak
- Backend API recognizes admin status
- Frontend cannot detect admin role from token

## Root Cause Analysis

### 1. Keycloak Token Structure

Keycloak tokens contain roles in different places:
- **Realm Roles**: Stored in `token.realm_access.roles[]`
- **Client Roles**: Stored in `token.resource_access.{client-id}.roles[]`

### 2. Client Scope Configuration Issue

**The Problem:**
- `frontend-client` is a **public client** (no client secret)
- Public clients need **explicit client scope assignment** to include realm roles
- The `realm-roles` client scope must be assigned as a **default scope** to the client
- The protocol mapper must be configured to include roles in the access token

**Why This Happens:**
- Keycloak has default client scopes, but they may not be assigned to all clients
- The `realm-roles` scope exists by default but must be explicitly assigned
- Without this assignment, realm roles are NOT included in the token

### 3. Frontend vs Backend Role Checking

**Backend (Profile Service):**
- Uses Keycloak Admin API: `/admin/realms/{realm}/users/{user-id}/role-mappings/realm`
- Directly queries Keycloak for user's realm roles
- **This works correctly** - backend can detect admin status

**Frontend (React App):**
- Relies on JWT token: `tokenParsed.realm_access.roles`
- If roles are not in token, frontend cannot detect admin status
- **This was failing** - token didn't contain roles

## Solution

### Solution 1: Fix Keycloak Client Scope (Recommended)

**What to do:**
1. Ensure `realm-roles` client scope is assigned to `frontend-client`
2. Verify protocol mapper is configured correctly
3. User must **log out and log back in** to get new token

**Script:** `fix-keycloak-client-scope-realm-roles.sh`

### Solution 2: Use Backend API Check (Implemented)

**What we did:**
- Frontend now calls `/api/admin/check` endpoint
- Backend uses Keycloak Admin API (more reliable)
- Frontend uses API result as primary, token check as fallback

**Benefits:**
- Works even if token doesn't contain roles
- Single source of truth (backend)
- More secure (backend validates with Keycloak)

## Keycloak Admin vs Application Admin

### Keycloak Admin Roles

**Realm Admin (`realm-admin`):**
- Keycloak administrative role
- Manages realm configuration
- **NOT** what we need for application admin

**Application Admin (`admin`):**
- Custom realm role for application
- Used for application-level permissions
- **This is what we need**

### How to Assign Application Admin Role

1. **Create the role** (if it doesn't exist):
   - Keycloak Admin Console → Realm → Roles → Add Role
   - Name: `admin`
   - Type: Realm Role

2. **Assign to user**:
   - Users → Select user → Role Mappings → Assign Role
   - Select `admin` realm role

3. **Verify in token**:
   - User must log out and log back in
   - Decode JWT token at https://jwt.io
   - Check for: `"realm_access": { "roles": ["admin", ...] }`

## Client Scope Configuration

### Required Configuration

**frontend-client must have:**
1. `realm-roles` scope assigned as **default scope**
2. Protocol mapper configured to include roles in access token
3. Mapper must have `access.token.claim: true`

### How to Verify

```bash
# Check if realm-roles scope is assigned
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${CLIENT_UUID}/default-client-scopes"

# Check protocol mapper
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "${KEYCLOAK_URL}/admin/realms/${REALM}/client-scopes/realm-roles/protocol-mappers/models"
```

## Frontend Implementation

### Current Implementation (Fixed)

```javascript
// Primary: Backend API check
authenticatedFetch('/api/admin/check')
  .then(res => res.json())
  .then(data => setIsAdmin(!!data?.isAdmin))

// Fallback: Token check
const roles = keycloak?.tokenParsed?.realm_access?.roles || [];
const hasAdminInToken = roles.some(r => r.toLowerCase() === 'admin');
```

### Why This Works

1. **Backend API is authoritative** - queries Keycloak directly
2. **Token check as fallback** - works if roles are in token
3. **Handles both cases** - works even if client scope is misconfigured

## Testing & Verification

### Step 1: Verify Role Assignment

```bash
# Check user's realm roles
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "${KEYCLOAK_URL}/admin/realms/${REALM}/users/${USER_ID}/role-mappings/realm"
```

Should show: `{"name": "admin", ...}`

### Step 2: Verify Client Scope

```bash
# Run fix script
./fix-keycloak-client-scope-realm-roles.sh
```

### Step 3: Test Token

1. User logs out completely
2. User logs back in
3. Decode JWT token at https://jwt.io
4. Verify `realm_access.roles` contains `"admin"`

### Step 4: Test Frontend

1. Open browser console
2. Check debug output: `=== DASHBOARD ADMIN ROLE DEBUG ===`
3. Verify `Backend API isAdmin: true`
4. Verify admin services are visible

## Common Issues & Solutions

### Issue 1: Role assigned but not in token

**Cause:** Client scope not configured
**Fix:** Run `fix-keycloak-client-scope-realm-roles.sh`

### Issue 2: Backend says admin, frontend doesn't

**Cause:** Frontend not using backend API check
**Fix:** Already fixed - frontend now uses `/api/admin/check`

### Issue 3: Token refresh doesn't update roles

**Cause:** Token refresh doesn't re-query Keycloak for roles
**Fix:** User must log out and log back in (not just refresh)

### Issue 4: Role in token but frontend doesn't detect

**Cause:** Frontend checking wrong role name or location
**Fix:** Already fixed - frontend checks both API and token

## Best Practices

1. **Always use backend API for role checks** - more reliable
2. **Token check as fallback only** - for offline/performance
3. **Log out/in after role changes** - token refresh won't work
4. **Verify client scope configuration** - especially for public clients
5. **Use consistent role names** - `admin` for application, `realm-admin` for Keycloak

## Next Steps

1. Run `fix-keycloak-client-scope-realm-roles.sh` on remote host
2. Verify user has `admin` realm role assigned
3. User logs out and logs back in
4. Frontend will use backend API check (already deployed)
5. Admin services should now be visible
