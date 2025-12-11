#!/bin/bash
# Build Docker images locally and deploy to remote host
# Usage: ./build-and-deploy.sh [frontend|backend|all]

set -e

REMOTE_HOST="${REMOTE_HOST:-72.60.80.84}"
REMOTE_USER="${REMOTE_USER:-root}"
REPO_DIR="${REPO_DIR:-/root/lianel/dc}"
DEPLOY_TARGET="${1:-all}"

echo "=== Building and Deploying Docker Images ==="
echo "Remote Host: $REMOTE_HOST"
echo "Deploy Target: $DEPLOY_TARGET"
echo ""

# Step 1: Build frontend image locally (if needed)
if [ "$DEPLOY_TARGET" = "frontend" ] || [ "$DEPLOY_TARGET" = "all" ]; then
    echo "Step 1: Building frontend image locally..."
    cd "$(dirname "$0")/../frontend" || exit 1
    docker build --platform linux/amd64 -t lianel-frontend:latest . || {
        echo "ERROR: Frontend build failed"
        exit 1
    }
    echo "✅ Frontend image built"

    # Step 2: Save frontend image as tar file
    echo ""
    echo "Step 2: Saving frontend image as tar file..."
    docker save lianel-frontend:latest -o /tmp/lianel-frontend.tar || {
        echo "ERROR: Failed to save frontend image"
        exit 1
    }
    echo "✅ Frontend image saved"

    # Step 3: Upload frontend image to remote host
    echo ""
    echo "Step 3: Uploading frontend image to remote host..."
    scp /tmp/lianel-frontend.tar ${REMOTE_USER}@${REMOTE_HOST}:/tmp/ || {
        echo "ERROR: Failed to upload frontend image"
        exit 1
    }
    echo "✅ Frontend image uploaded"

    # Step 4: Load frontend image into Docker on remote host
    echo ""
    echo "Step 4: Loading frontend image into Docker on remote host..."
    ssh ${REMOTE_USER}@${REMOTE_HOST} "docker load -i /tmp/lianel-frontend.tar && rm -f /tmp/lianel-frontend.tar" || {
        echo "ERROR: Failed to load frontend image"
        exit 1
    }
    echo "✅ Frontend image loaded"

    # Step 5: Deploy frontend container
    echo ""
    echo "Step 5: Deploying frontend container..."
    ssh ${REMOTE_USER}@${REMOTE_HOST} "cd ${REPO_DIR} && \
        docker stop lianel-frontend 2>/dev/null || true && \
        docker rm lianel-frontend 2>/dev/null || true && \
        docker compose -f docker-compose.frontend.yaml up -d frontend" || {
        echo "ERROR: Failed to deploy frontend container"
        exit 1
    }
    echo "✅ Frontend container deployed"

    # Cleanup local tar file
    rm -f /tmp/lianel-frontend.tar
fi

# Step 6: Build backend image locally (if needed)
if [ "$DEPLOY_TARGET" = "backend" ] || [ "$DEPLOY_TARGET" = "all" ]; then
    echo ""
    echo "Step 6: Building backend image locally..."
    cd "$(dirname "$0")/../profile-service" || exit 1
    docker build --platform linux/amd64 -t lianel-profile-service:latest . || {
        echo "ERROR: Backend build failed"
        exit 1
    }
    echo "✅ Backend image built"

    # Step 7: Save backend image as tar file
    echo ""
    echo "Step 7: Saving backend image as tar file..."
    docker save lianel-profile-service:latest -o /tmp/lianel-profile-service.tar || {
        echo "ERROR: Failed to save backend image"
        exit 1
    }
    echo "✅ Backend image saved"

    # Step 8: Upload backend image to remote host
    echo ""
    echo "Step 8: Uploading backend image to remote host..."
    scp /tmp/lianel-profile-service.tar ${REMOTE_USER}@${REMOTE_HOST}:/tmp/ || {
        echo "ERROR: Failed to upload backend image"
        exit 1
    }
    echo "✅ Backend image uploaded"

    # Step 9: Load backend image into Docker on remote host
    echo ""
    echo "Step 9: Loading backend image into Docker on remote host..."
    ssh ${REMOTE_USER}@${REMOTE_HOST} "docker load -i /tmp/lianel-profile-service.tar && rm -f /tmp/lianel-profile-service.tar" || {
        echo "ERROR: Failed to load backend image"
        exit 1
    }
    echo "✅ Backend image loaded"

    # Step 10: Deploy backend container
    echo ""
    echo "Step 10: Deploying backend container..."
    ssh ${REMOTE_USER}@${REMOTE_HOST} "cd ${REPO_DIR} && \
        docker stop lianel-profile-service 2>/dev/null || true && \
        docker rm lianel-profile-service 2>/dev/null || true && \
        docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml up -d profile-service" || {
        echo "ERROR: Failed to deploy backend container"
        exit 1
    }
    echo "✅ Backend container deployed"

    # Cleanup local tar file
    rm -f /tmp/lianel-profile-service.tar
fi

echo ""
echo "=== Deployment Complete ==="
echo ""
echo "Services deployed:"
echo "- Frontend: https://www.lianel.se"
echo "- Backend API: https://www.lianel.se/api/"
echo ""
echo "Next: Test the complete flow"

