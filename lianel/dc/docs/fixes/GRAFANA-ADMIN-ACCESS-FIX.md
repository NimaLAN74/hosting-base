# Grafana Admin Access and Connections Page Fix

## Problems
1. **Admin user cannot see Configuration menu** - Even when logged in as admin via Keycloak OAuth
2. **Connections page returns 404** - The `/connections` endpoint is not accessible

## Root Causes

### 1. Role Mapping Issue
- The `role_attribute_path` was mapping to `Admin` (org-level) but not `GrafanaAdmin` (server-level)
- The `roles` scope was missing from OAuth scopes, so Keycloak wasn't sending roles in the token
- Keycloak client might not have a mapper to include roles in the ID token

### 2. Connections Page
- The connections feature might need to be explicitly enabled in Grafana 11.x
- Requires proper admin permissions to access

## Fixes Applied

### 1. Updated `grafana.ini`
- **Added `roles` to scopes**: Changed `scopes = openid email profile` to `scopes = openid email profile roles`
- **Updated role mapping**: Changed `role_attribute_path` to prioritize `GrafanaAdmin` for admin users:
  ```ini
  role_attribute_path = contains(realm_access.roles[*], 'admin') && 'GrafanaAdmin' || contains(realm_access.roles[*], 'admin') && 'Admin' || contains(realm_access.roles[*], 'editor') && 'Editor' || 'Admin'
  ```
- **Enabled connections feature**: Added `[feature_toggles]` section with `enable = connections`

### 2. Created Keycloak Configuration Script
- Created `scripts/configure-grafana-keycloak-roles.sh` to:
  - Add Realm Roles mapper to grafana-client (to include roles in ID token)
  - Ensure `roles` scope is included in default client scopes

## Files Modified
- `lianel/dc/monitoring/grafana/grafana.ini` - Updated OAuth config and enabled connections
- `lianel/dc/scripts/configure-grafana-keycloak-roles.sh` - New script to configure Keycloak

## Next Steps

### On Remote Host:

1. **Restart Grafana** to pick up the new configuration:
   ```bash
   cd /root/hosting-base/lianel/dc
   docker-compose -f docker-compose.monitoring.yaml restart grafana
   ```

2. **Configure Keycloak** to send roles in token:
   ```bash
   cd /root/hosting-base/lianel/dc
   source .env
   KEYCLOAK_ADMIN_PASSWORD="$KEYCLOAK_ADMIN_PASSWORD" bash scripts/configure-grafana-keycloak-roles.sh
   ```

3. **Log out and log back in** to Grafana to get a fresh token with roles

4. **Verify access**:
   - Check that Configuration menu is visible
   - Navigate to `/connections` - should not return 404
   - Verify you can see Data Sources, Connections, etc.

## Verification

After applying fixes, you should be able to:
- ✅ See "Configuration" menu in Grafana sidebar
- ✅ Access `/connections` page without 404
- ✅ See "Server Admin" section (if mapped to GrafanaAdmin)
- ✅ Manage datasources and connections

## Troubleshooting

If issues persist:

1. **Check token claims**: Decode your ID token to verify `realm_access.roles` contains `admin`
2. **Check Grafana logs**: `docker logs grafana` for role mapping errors
3. **Verify Keycloak mapper**: Ensure Realm Roles mapper exists for grafana-client
4. **Check scopes**: Verify `roles` scope is in the OAuth request
