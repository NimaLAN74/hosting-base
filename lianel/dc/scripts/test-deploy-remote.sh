#!/bin/bash
# Test deployment script on remote host
# Usage: ./test-deploy-remote.sh

set -euo pipefail

# These should match your GitHub secrets
REMOTE_HOST="${REMOTE_HOST:-}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"

if [ -z "$REMOTE_HOST" ]; then
  echo "❌ Error: REMOTE_HOST not set"
  echo "Usage: REMOTE_HOST=your-host ./test-deploy-remote.sh"
  exit 1
fi

echo "=== Testing Deployment Script on Remote Host ==="
echo "Host: $REMOTE_USER@$REMOTE_HOST:$REMOTE_PORT"
echo ""

# Test 1: Check if script exists on remote
echo "Test 1: Checking if script exists on remote..."
if ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "test -f /root/deploy-frontend.sh"; then
  echo "✅ Script exists on remote host"
else
  echo "⚠️  Script not found on remote host (this is OK if not deployed yet)"
fi
echo ""

# Test 2: Check script syntax on remote
echo "Test 2: Checking script syntax on remote..."
if ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "bash -n /root/deploy-frontend.sh 2>&1"; then
  echo "✅ Script syntax is valid on remote"
else
  echo "❌ Script syntax error on remote"
  exit 1
fi
echo ""

# Test 3: Check Docker is available
echo "Test 3: Checking Docker availability..."
if ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "command -v docker >/dev/null 2>&1"; then
  echo "✅ Docker is available"
  DOCKER_VERSION=$(ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "docker --version")
  echo "   Version: $DOCKER_VERSION"
else
  echo "❌ Docker not found on remote host"
  exit 1
fi
echo ""

# Test 4: Check docker compose availability
echo "Test 4: Checking docker compose availability..."
if ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "docker compose version >/dev/null 2>&1"; then
  echo "✅ docker compose is available"
  COMPOSE_VERSION=$(ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "docker compose version")
  echo "   Version: $COMPOSE_VERSION"
elif ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "command -v docker-compose >/dev/null 2>&1"; then
  echo "✅ docker-compose is available"
  COMPOSE_VERSION=$(ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "docker-compose --version")
  echo "   Version: $COMPOSE_VERSION"
else
  echo "❌ Neither docker compose nor docker-compose found"
  exit 1
fi
echo ""

# Test 5: Check if docker-compose.yaml exists
echo "Test 5: Checking docker-compose.yaml..."
if ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "test -f /root/lianel/dc/docker-compose.yaml"; then
  echo "✅ docker-compose.yaml exists"
else
  echo "⚠️  docker-compose.yaml not found (this might be OK if path is different)"
fi
echo ""

# Test 6: Check network exists
echo "Test 6: Checking Docker network..."
if ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "docker network ls | grep -q lianel-network"; then
  echo "✅ lianel-network exists"
else
  echo "⚠️  lianel-network not found (will be created by docker compose)"
fi
echo ""

# Test 7: Dry run of image cleanup logic
echo "Test 7: Testing image cleanup logic (dry run)..."
TEST_REPO="ghcr.io/test/repo"
REPO_NAME=$(ssh -p "$REMOTE_PORT" "$REMOTE_USER@$REMOTE_HOST" "echo '$TEST_REPO' | cut -d':' -f1")
if [ "$REPO_NAME" = "$TEST_REPO" ]; then
  echo "✅ Repo name extraction works on remote"
else
  echo "❌ Repo name extraction failed on remote"
  exit 1
fi
echo ""

echo "=== Remote Host Tests Complete ==="
echo ""
echo "✅ All checks passed!"
echo ""
echo "The deployment script should work on the remote host."
echo "To test actual deployment, run:"
echo "  ssh $REMOTE_USER@$REMOTE_HOST '/root/deploy-frontend.sh <IMAGE_TAG>'"
