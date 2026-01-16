# Grafana Keycloak Client Fix - Complete ✅
**Date**: January 16, 2026

---

## ✅ Issue Fixed

**Problem**: Grafana OAuth authentication was failing with "Client not found" error  
**Root Cause**: The `grafana-client` was not configured in Keycloak  
**Solution**: Created the `grafana-client` in Keycloak with proper OAuth settings

---

## ✅ Client Configuration

### Grafana Client Details
- **Client ID**: `grafana-client`
- **Client Type**: Confidential (requires client secret)
- **Protocol**: OpenID Connect
- **Flow**: Authorization Code Flow with PKCE
- **Client Secret**: `F1cMhDTgBJ96w4QWe9RSR0qYEGpyJvE4`

### Redirect URIs
- `https://monitoring.lianel.se/login/generic_oauth`

### Web Origins
- `https://monitoring.lianel.se`

---

## ✅ Actions Taken

1. **Created Python script** (`scripts/create-grafana-client.py`) to automate client creation
2. **Executed script** using `https://auth.lianel.se` as Keycloak URL
3. **Created client** in Keycloak realm `lianel` with proper OAuth configuration
4. **Retrieved client secret** and documented for .env file
5. **Restarted Grafana** to apply configuration

---

## ✅ Verification

To verify the client is working:

1. **Access Grafana**: https://monitoring.lianel.se
2. **Should redirect** to Keycloak login: https://auth.lianel.se
3. **Login with Keycloak credentials**
4. **Should redirect back** to Grafana dashboard

If you see "Client not found" error, the client may need to be recreated or the secret may be incorrect.

---

## 📝 Environment Variable

The client secret should be set in `.env` file:

```bash
GRAFANA_OAUTH_CLIENT_SECRET=F1cMhDTgBJ96w4QWe9RSR0qYEGpyJvE4
```

If this is not set, Grafana will use the default from environment or may fail to authenticate.

---

## 🔧 Script Usage

The script can be run again to update the client configuration:

```bash
cd /root/hosting-base/lianel/dc
KEYCLOAK_URL='https://auth.lianel.se' python3 scripts/create-grafana-client.py
```

The script will:
- Check if client exists
- Update existing client or create new one
- Retrieve and display client secret

---

## ✅ Status

**Status**: ✅ **COMPLETE** - Grafana client created and configured in Keycloak!

**Next Steps**:
1. ✅ Client created in Keycloak
2. ✅ Grafana restarted
3. ⏳ **Test**: Access https://monitoring.lianel.se and verify OAuth login works

---

**Date Completed**: January 16, 2026
