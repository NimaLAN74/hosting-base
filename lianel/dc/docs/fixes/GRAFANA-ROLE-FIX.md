# Grafana Role Assignment Fix

## Problem
Keycloak admin users were assigned "Viewer" role in Grafana, preventing them from seeing configuration options and datasources.

## Root Cause
The `role_attribute_path` was configured, but `auto_assign_org_role` was set to `Viewer`, which overrides the role attribute path for new users.

## Fix Applied
Changed `auto_assign_org_role` from `Viewer` to `Admin` in `grafana.ini`:

```ini
[auth.generic_oauth]
role_attribute_path = contains(realm_access.roles[*], 'admin') && 'Admin' || contains(realm_access.roles[*], 'editor') && 'Editor' || 'Viewer'
allow_assign_grafana_admin = true
auto_assign_org_role = Admin  # Changed from Viewer to Admin
auto_login = true
```

## Impact

### Before:
- All users (including admins) assigned "Viewer" role
- No access to configuration options
- No access to datasource management
- Can only view dashboards

### After:
- Admin users get "Admin" role (via `auto_assign_org_role = Admin`)
- Editor users get "Editor" role (via `role_attribute_path`)
- Regular users get "Viewer" role (fallback)
- Admin users can now:
  - See configuration options
  - Manage datasources
  - Edit dashboards
  - Access all Grafana features

## Verification

After Grafana restart:
1. Log in to Grafana as admin user
2. Go to Configuration → Data Sources
3. Should now see all datasources and be able to edit them
4. Should see Configuration menu in Grafana sidebar

## Status
✅ **Fixed** - Admin users now get Admin role in Grafana

## Note
This change affects new users. Existing users may need to have their roles manually updated in Grafana, or they need to log out and log back in.
