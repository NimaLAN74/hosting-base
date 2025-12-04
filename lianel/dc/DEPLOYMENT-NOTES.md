# Frontend Deployment Notes

## Important: Cross-Platform Builds

**Local Platform**: macOS (ARM64)  
**Remote Platform**: Linux (AMD64/x86_64)

When building the frontend image locally, you **MUST** specify the target platform:

```bash
# Correct build command (from lianel/dc directory)
docker buildx build --platform linux/amd64 -t lianel-frontend:latest -f frontend/Dockerfile frontend/

# OR using docker-compose with platform specification
docker-compose -f docker-compose.frontend.yaml build --build-arg BUILDPLATFORM=linux/amd64
```

## Current Deployment Workflow

### Step 1: Build Image (Local - macOS)
```bash
cd /Users/r04470/practis/hosting-base/lianel/dc

# Build for AMD64 platform
docker buildx build --platform linux/amd64 -t lianel-frontend:latest -f frontend/Dockerfile frontend/
```

### Step 2: Save Image
```bash
docker save lianel-frontend:latest | gzip > lianel-frontend-amd64.tar.gz
```

### Step 3: Transfer to Remote Host
```bash
scp lianel-frontend-amd64.tar.gz root@72.60.80.84:/root/lianel/dc/
```

### Step 4: Load and Deploy (Remote Host)
```bash
ssh root@72.60.80.84

cd /root/lianel/dc

# Load the image
docker load < lianel-frontend-amd64.tar.gz

# Deploy using docker-compose.yaml (image only, no build)
docker-compose -f docker-compose.yaml up -d frontend

# OR if using docker-compose.frontend.yaml (ensure image is already loaded)
docker-compose -f docker-compose.frontend.yaml up -d
```

## Docker Compose Files

### `docker-compose.frontend.yaml` (Local Development)
- Contains `build:` section for local development
- Can be used locally to build images
- **Note**: On remote host, this file still has `build:` but should only be used after image is loaded

### `docker-compose.yaml` (Remote Deployment)
- Contains only `image:` reference (no build)
- Safe to use on remote host
- Requires image to be loaded first

## Verification

After deployment, verify the image platform:
```bash
# On remote host
docker inspect lianel-frontend --format '{{.Architecture}}'
# Should output: amd64
```

## Troubleshooting

### Issue: Image built for wrong platform
**Symptoms**: Container fails to start or shows architecture mismatch errors  
**Solution**: Rebuild with `--platform linux/amd64` flag

### Issue: Build fails on macOS
**Solution**: Ensure Docker Desktop has buildx enabled:
```bash
docker buildx version  # Should show version
docker buildx create --use  # Create builder if needed
```

### Issue: Image too large
**Solution**: Use multi-stage build (already in Dockerfile) and ensure `.dockerignore` excludes unnecessary files

## Future Automation

Once automation script is created, it should:
1. Build with `--platform linux/amd64`
2. Save image with proper naming
3. Transfer via SCP
4. Load on remote
5. Deploy using docker-compose
6. Verify deployment

