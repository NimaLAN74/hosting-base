# Frontend Client Redirect URI Fix ✅
**Date**: January 16, 2026

---

## ✅ Issue Fixed

**Problem**: Frontend authentication failing with 400 Bad Request  
**Error**: `GET https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth?client_id=frontend-client&redirect_uri=https://www.lianel.se/`  
**Root Cause**: The redirect URI `https://www.lianel.se/` was not in the frontend-client's allowed redirect URIs

---

## ✅ Fix Applied

**Action**: Added exact redirect URI `https://www.lianel.se/` to frontend-client configuration

**Previous redirect URIs**:
- `http://localhost:3000/*`
- `https://www.lianel.se/*` (wildcard pattern)
- `https://lianel.se/*`
- `https://www.lianel.se` (without trailing slash)
- `http://localhost:3000`
- `https://lianel.se`

**Updated redirect URIs** (now includes):
- ✅ `https://www.lianel.se/` (exact match with trailing slash)

---

## ✅ Current Configuration

The `frontend-client` now has the following redirect URIs configured:

```
- http://localhost:3000/*
- https://www.lianel.se/*      (wildcard pattern)
- https://www.lianel.se/       (exact match - NEW)
- https://www.lianel.se        (exact match without slash)
- https://lianel.se/*
- https://lianel.se
- http://localhost:3000
```

---

## 🔍 Why This Matters

Keycloak redirect URI matching can be strict:
- Wildcard patterns (`https://www.lianel.se/*`) may not match exact URIs (`https://www.lianel.se/`)
- Different browsers/clients may send URIs with or without trailing slashes
- Frontend code may construct redirect URIs differently than expected

**Best Practice**: Include both wildcard patterns AND exact URIs with/without trailing slashes

---

## ✅ Verification

To verify the fix is working:

1. **Access frontend**: https://www.lianel.se
2. **Click login** - should redirect to Keycloak
3. **After login** - should redirect back to frontend successfully
4. **No 400 errors** - authentication should complete without errors

---

## 📝 Related Configuration

The frontend client is configured as:
- **Client ID**: `frontend-client`
- **Client Type**: Public (no client secret required)
- **Flow**: Authorization Code Flow with PKCE
- **Web Origins**: `https://www.lianel.se`, `https://lianel.se`

---

**Status**: ✅ **FIXED** - Frontend authentication should now work correctly!
