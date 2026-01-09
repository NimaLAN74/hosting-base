# Frontend Deployment Image Update Fix

## Issue
The frontend deployment pipeline was not updating the Docker image on the remote host when a new image was pushed with the same tag (`:latest`). Docker was using a cached version instead of pulling the latest image.

## Root Cause
When Docker pulls an image with the same tag, it may use a cached version if:
1. The local image tag already exists
2. Docker thinks it has the "latest" version
3. The image digest hasn't changed in Docker's cache

## Solution Applied
Updated `/root/deploy-frontend.sh` on the remote host to:

1. **Remove old local tags** before pulling:
   ```bash
   docker rmi "$LOCAL_TAG" 2>/dev/null || true
   docker rmi "$IMAGE_TAG" 2>/dev/null || true
   ```

2. **Force pull** the latest image:
   ```bash
   docker pull "$IMAGE_TAG"
   ```

3. **Verify image digest** after deployment:
   ```bash
   docker inspect "$IMAGE_TAG" --format '{{.RepoDigests}}'
   ```

## Changes Made
- Script updated on remote host: `/root/deploy-frontend.sh`
- Removes old image tags before pulling
- Forces fresh pull of latest image
- Better error handling and verification

## Testing
To verify the fix works:
1. Push a new frontend change
2. Check GitHub Actions pipeline completes successfully
3. Verify on remote host:
   ```bash
   ssh root@72.60.80.84
   docker images | grep lianel-frontend
   docker ps | grep lianel-frontend
   ```

## Date
2026-01-09
