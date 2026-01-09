# Frontend Pipeline Troubleshooting Guide

## Common Failure Points

### 1. Docker Build Failures

#### npm install fails
**Symptoms**: Error during `npm install` or `npm ci`
**Common Causes**:
- Peer dependency conflicts
- Network issues downloading packages
- Package.json syntax errors

**Fix Applied**:
- Updated Dockerfile to use `npm ci --legacy-peer-deps`
- Added fallback to `npm install --legacy-peer-deps`
- This handles recharts peer dependency issues

#### Build fails
**Symptoms**: Error during `npm run build`
**Common Causes**:
- Syntax errors in React code
- Missing dependencies
- TypeScript/ESLint errors

**Check**:
- Review build logs in GitHub Actions
- Test build locally: `npm run build`
- Check for import errors

### 2. SSH Connection Failures

**Symptoms**: "Permission denied" or "Connection refused"
**Common Causes**:
- SSH key not configured correctly
- Wrong REMOTE_HOST or REMOTE_USER
- Firewall blocking SSH

**Fix**:
- Verify SSH_PRIVATE_KEY secret contains complete private key
- Check REMOTE_HOST is correct IP
- Test SSH manually: `ssh -i ~/.ssh/key root@72.60.80.84`

### 3. Deployment Script Failures

**Symptoms**: Error in deploy-frontend.sh script
**Common Causes**:
- Script not found on remote host
- Docker not running on remote host
- Permission issues

**Fix**:
- Verify `/root/deploy-frontend.sh` exists on remote host
- Check Docker is running: `docker ps`
- Verify script has execute permissions

### 4. Package Registry Issues

**Symptoms**: "Failed to pull image" or "unauthorized"
**Common Causes**:
- Package not public
- GITHUB_TOKEN not working
- Registry authentication issues

**Fix**:
- Make package public manually: https://github.com/NimaLAN74?tab=packages
- Check package visibility settings
- Verify GITHUB_TOKEN has package read permissions

## Quick Diagnostic Commands

### Check if build works locally:
```bash
cd lianel/dc/frontend
npm install
npm run build
```

### Check Docker build:
```bash
cd lianel/dc/frontend
docker build -t test-frontend .
```

### Check remote host:
```bash
ssh root@72.60.80.84
docker ps | grep frontend
docker logs lianel-frontend
```

## Current Status

**Last Commit**: `b072f5a` - Fix Dashboard console logging and implement Phase 1 data visualization

**Changes**:
- Added recharts dependency
- Fixed Dashboard.js console logging
- Added EnergyCharts component
- Updated Dockerfile for better dependency handling

**Pipeline Should**:
1. ✅ Install recharts successfully
2. ✅ Build React app with charts
3. ✅ Deploy to remote host

## If Pipeline Still Fails

1. **Check GitHub Actions logs**:
   - Go to: https://github.com/NimaLAN74/hosting-base/actions
   - Click on the failed workflow
   - Review error messages

2. **Common fixes**:
   - If npm install fails: Check package.json syntax
   - If build fails: Check for React/import errors
   - If deploy fails: Check SSH and remote host

3. **Manual deployment** (if needed):
   ```bash
   # On remote host
   cd /root/lianel/dc
   docker-compose -f docker-compose.frontend.yaml pull
   docker-compose -f docker-compose.frontend.yaml up -d --force-recreate frontend
   ```
