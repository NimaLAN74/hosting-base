# Keycloak 26.4.6 PKCE Requirement Issue
**Date**: January 16, 2026

---

## 🔍 Issue Discovered

**Problem**: Keycloak 26.4.6 **requires PKCE** for public clients, even when PKCE attributes are removed from the client configuration.

**Symptoms**:
- Without PKCE: `Missing parameter: code_challenge_method` (400 Bad Request)
- With invalid PKCE: `Invalid parameter: code_challenge` (400 Bad Request)
- PKCE attributes removed from client: Still requires PKCE

---

## 🔍 Root Cause

**Keycloak 26.4.6** has PKCE **enforced by default** for public clients. This is a security enhancement in Keycloak 26.x that cannot be disabled via client configuration alone.

---

## ✅ Solution

Since PKCE cannot be disabled, we need to:

1. **Re-enable PKCE in frontend** (`pkceMethod: 'S256'`)
2. **Ensure PKCE attributes are set in Keycloak client** (`pkce.code.challenge.method: S256`)
3. **Fix PKCE challenge generation** (may require upgrading keycloak-js)

---

## ⚠️ Current Status

- ✅ **PKCE re-enabled** in frontend
- ✅ **PKCE attributes set** in Keycloak client
- ⏳ **PKCE challenge issue** needs to be fixed (upgrade keycloak-js or fix generation)

---

## 🔧 Next Steps

### Option 1: Upgrade keycloak-js (Recommended)
```bash
cd lianel/dc/frontend
npm install keycloak-js@latest
```

### Option 2: Fix PKCE Challenge Generation
Ensure keycloak-js generates valid base64url-encoded SHA256 hashes.

### Option 3: Downgrade Keycloak
Downgrade to Keycloak version that doesn't enforce PKCE for public clients (not recommended - security risk).

---

## 📝 Keycloak 26.4.6 Behavior

- **Public clients**: PKCE required by default
- **Confidential clients**: PKCE optional
- **Cannot disable**: PKCE requirement enforced at realm/client type level

---

**Status**: ⚠️ **PKCE IS REQUIRED** - We must fix PKCE challenge generation, not disable it!
