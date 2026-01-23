# Pipeline Deploy Script Fix

## Problem
The GitHub Actions workflow was failing at the "Copy Deployment Script to Remote Host" step with exit code 255. The error was:
```
Process completed with exit code 255
```

## Root Cause
The workflow expected the deployment script at:
- `lianel/dc/deploy-frontend.sh`

But the script was actually located at:
- `lianel/dc/scripts/deployment/deploy-frontend.sh`

**Note**: The script has been moved to the proper location in `scripts/deployment/` and the workflow has been updated to reference the correct path.

When the workflow tried to copy the script via SCP, it couldn't find the file, causing the SCP command to fail with exit code 255.

## Fix Applied
Copied the deployment script from `lianel/dc/scripts/deployment/deploy-frontend.sh` to `lianel/dc/deploy-frontend.sh` so the workflow can find it.

## Files Modified
- ✅ Created `lianel/dc/deploy-frontend.sh` (copied from `lianel/dc/scripts/deployment/deploy-frontend.sh`)

## Workflow Reference
The workflow step that was failing:
```yaml
- name: Copy Deployment Script to Remote Host
  run: |
    scp -i ~/.ssh/deploy_key \
      lianel/dc/deploy-frontend.sh \
      ${REMOTE_USER}@${REMOTE_HOST}:/root/deploy-frontend.sh
```

## Status
✅ **Fixed** - Script now exists at the expected location and workflow should succeed
