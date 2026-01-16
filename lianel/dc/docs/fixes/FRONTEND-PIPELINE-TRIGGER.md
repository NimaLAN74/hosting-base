# Frontend Pipeline Trigger - PKCE Fix
**Date**: January 16, 2026

---

## ✅ Pipeline Triggered

**Commit**: Trigger frontend rebuild for PKCE fix  
**File Changed**: `lianel/dc/frontend/.trigger`  
**Purpose**: Force GitHub Actions workflow to rebuild and redeploy frontend with PKCE disabled

---

## 📋 What Happens Next

1. **GitHub Actions Workflow**: `deploy-frontend.yml` is triggered
2. **Build**: Frontend Docker image is built with PKCE disabled
3. **Push**: Image is pushed to GitHub Container Registry (ghcr.io)
4. **Deploy**: Image is deployed to remote host and container is restarted

---

## 🔍 Monitor Deployment

You can monitor the deployment at:
- **GitHub Actions**: https://github.com/NimaLAN74/hosting-base/actions/workflows/deploy-frontend.yml
- **Latest Run**: Should show "Trigger frontend rebuild for PKCE fix" as the commit message

---

## ✅ Expected Result

After deployment completes:
- ✅ Frontend will have PKCE disabled (`pkceMethod` commented out)
- ✅ Keycloak authentication will work without `Invalid parameter: code_challenge` error
- ✅ Users can log in to the frontend successfully

---

## 📝 Verification Steps

After deployment, verify:

1. **Check Frontend Container**:
   ```bash
   docker ps | grep frontend
   ```

2. **Check Frontend Logs**:
   ```bash
   docker logs lianel-frontend --tail 50
   ```

3. **Test Login**:
   - Go to: https://www.lianel.se
   - Click Login
   - Should redirect to Keycloak without 400 error
   - After login, should redirect back to frontend

---

**Status**: ✅ **PIPELINE TRIGGERED** - Frontend will be rebuilt and redeployed automatically!
