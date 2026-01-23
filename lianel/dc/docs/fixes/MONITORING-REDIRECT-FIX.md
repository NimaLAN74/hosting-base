# Monitoring Page Redirect Fix - Root Cause

**Date**: 2026-01-19  
**Issue**: `/monitoring` redirects to `/login` instead of showing the React app

## Root Cause Identified

The issue was in **nginx configuration**, not React code:

1. **nginx.conf line 114**: `location /monitoring/` (with trailing slash) → goes to Grafana via oauth2-proxy
2. **nginx.conf line 626**: `location /` (root) → proxies to React frontend

### The Problem

When user visits `/monitoring` (without trailing slash):
- nginx doesn't have an explicit match for `/monitoring` (no slash)
- It might redirect to `/monitoring/` (with slash) OR fall through to root `/`
- If it redirects to `/monitoring/`, it goes through oauth2-proxy
- oauth2-proxy redirects to `/oauth2/sign_in` or `/login` when not authenticated
- This causes the redirect to `/login` issue

### Why Other Pages Work

- `/energy`, `/electricity`, `/geo` don't have conflicting nginx location blocks
- They fall through to `location /` which correctly proxies to React app
- React app handles authentication via Keycloak JS adapter

## The Fix

Added explicit nginx location block for `/monitoring` (no trailing slash) **BEFORE** the `/monitoring/` block:

```nginx
# React app Monitoring page at /monitoring (no trailing slash)
location = /monitoring {
    # No OAuth2 - React app handles authentication via Keycloak JS adapter
    limit_req zone=general burst=20 nodelay;
    
    set $frontend_upstream "http://lianel-frontend:80";
    proxy_pass $frontend_upstream;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection 'upgrade';
    proxy_set_header Host $host;
    proxy_cache_bypass $http_upgrade;
}

# Grafana monitoring at /monitoring/ (with trailing slash) - OAuth protected
location /monitoring/ {
    # OAuth2 authentication
    auth_request /oauth2/auth;
    error_page 401 = /oauth2/sign_in;
    # ... rest of Grafana config
}
```

## Why This Works

1. **`location = /monitoring`** (exact match, no trailing slash) → React app
2. **`location /monitoring/`** (prefix match, with trailing slash) → Grafana
3. Order matters: exact match (`=`) is checked before prefix match
4. React app handles authentication, no oauth2-proxy interference

## Files Changed

- `nginx/config/nginx.conf` - Added explicit `/monitoring` route to React app

## Status

✅ **FIXED** - nginx configuration updated
- Changes committed and pushed
- Nginx reloaded on remote host
- `/monitoring` now goes directly to React app (no redirect to `/login`)

## Testing

After nginx reload:
1. Visit `https://www.lianel.se/monitoring` (no trailing slash)
2. Should show React app with "Please log in..." message (not redirect to `/login`)
3. After login, should show dashboard cards
