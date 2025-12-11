#!/bin/bash
# Deployment script for frontend updates
# Usage: ./deploy-frontend.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR" || exit 1

echo "🚀 Deploying frontend updates..."
echo ""

# Step 1: Ensure network exists
echo "1️⃣  Ensuring lianel-network exists..."
docker network create lianel-network 2>/dev/null || true
echo "   ✅ Network ready"
echo ""

# Step 2: Remove old container
echo "2️⃣  Removing old frontend container..."
docker rm -f lianel-frontend 2>/dev/null || true
echo "   ✅ Old container removed"
echo ""

# Step 3: Start new container with infra compose file
echo "3️⃣  Starting new frontend container..."
docker compose -f docker-compose.infra.yaml -f docker-compose.yaml up -d frontend
sleep 3
echo "   ✅ Container started"
echo ""

# Step 4: Fix network configuration
echo "4️⃣  Configuring network..."
docker network disconnect dc_default lianel-frontend 2>/dev/null || true
docker network connect lianel-network lianel-frontend 2>/dev/null || true
echo "   ✅ Network configured"
echo ""

# Step 5: Verify
echo "5️⃣  Verifying deployment..."
STATUS=$(curl -sk -w '%{http_code}' -o /dev/null 'https://lianel.se/' 2>/dev/null || echo "000")
if [ "$STATUS" = "200" ]; then
    echo "   ✅ Frontend is responding (HTTP $STATUS)"
    echo ""
    echo "🎉 Deployment successful!"
    echo "   Frontend: https://lianel.se"
else
    echo "   ⚠️  Frontend returned HTTP $STATUS (expected 200)"
    echo "   Check with: curl -sk 'https://lianel.se/'"
fi
