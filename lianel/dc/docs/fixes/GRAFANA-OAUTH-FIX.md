# Grafana OAuth Fix

**Date**: January 16, 2026  
**Status**: ✅ **FIXED** - Added client_secret to Grafana configuration

---

## 🔍 Issue

**Error**: "Login failed - Failed to get token from provider"  
**Grafana Logs**: `"unauthorized_client" "Invalid client or Invalid client credentials"`

**Root Cause**: Grafana client in Keycloak is **confidential** (requires `client_secret`), but `grafana.ini` was missing the `client_secret` configuration.

---

## ✅ Solution

**File**: `monitoring/grafana/grafana.ini`

**Added**:
```ini
client_secret = F1cMhDTgBJ96w4QWe9RSR0qYEGpyJvE4
```

**Location**: `[auth.generic_oauth]` section

---

## 📋 Grafana Client Configuration

**Keycloak Client**:
- Client ID: `grafana-client`
- Type: **Confidential** (requires secret)
- Standard Flow: Enabled
- PKCE: S256 (required)
- Redirect URI: `https://monitoring.lianel.se/login/generic_oauth`
- Client Secret: `F1cMhDTgBJ96w4QWe9RSR0qYEGpyJvE4`

**Grafana Configuration** (`grafana.ini`):
```ini
[auth.generic_oauth]
enabled = true
client_id = grafana-client
client_secret = F1cMhDTgBJ96w4QWe9RSR0qYEGpyJvE4
use_pkce = true
```

---

## ✅ Verification

After restarting Grafana:
1. Grafana should start without errors
2. OAuth login should work
3. Token exchange should succeed

---

**Status**: Fixed - Grafana OAuth should now work correctly.
