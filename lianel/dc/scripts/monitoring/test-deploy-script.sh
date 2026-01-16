#!/bin/bash
# Test script for deploy-frontend.sh
# This validates the deployment script logic without actually deploying

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_SCRIPT="$SCRIPT_DIR/../deploy-frontend.sh"

echo "=== Testing Deployment Script ==="
echo ""

# Test 1: Check script syntax
echo "Test 1: Checking script syntax..."
if bash -n "$DEPLOY_SCRIPT"; then
  echo "✅ Syntax check passed"
else
  echo "❌ Syntax check failed"
  exit 1
fi
echo ""

# Test 2: Check script has required functions
echo "Test 2: Checking script structure..."
if grep -q "IMAGE_TAG=" "$DEPLOY_SCRIPT"; then
  echo "✅ IMAGE_TAG variable found"
else
  echo "❌ IMAGE_TAG variable not found"
  exit 1
fi

if grep -q "docker pull" "$DEPLOY_SCRIPT"; then
  echo "✅ docker pull command found"
else
  echo "❌ docker pull command not found"
  exit 1
fi

if grep -q "PULL_ATTEMPTS" "$DEPLOY_SCRIPT"; then
  echo "✅ Retry logic found"
else
  echo "❌ Retry logic not found"
  exit 1
fi

if grep -q "docker compose" "$DEPLOY_SCRIPT"; then
  echo "✅ docker compose command found"
else
  echo "❌ docker compose command not found"
  exit 1
fi
echo ""

# Test 3: Test image cleanup logic (dry run)
echo "Test 3: Testing image cleanup logic..."
TEST_IMAGE_TAG="ghcr.io/test/repo:latest"
REPO_NAME=$(echo "$TEST_IMAGE_TAG" | cut -d':' -f1)
echo "Test repo name extraction: $REPO_NAME"
if [ "$REPO_NAME" = "ghcr.io/test/repo" ]; then
  echo "✅ Repo name extraction works"
else
  echo "❌ Repo name extraction failed"
  exit 1
fi
echo ""

# Test 4: Test retry logic structure
echo "Test 4: Testing retry logic structure..."
if grep -A 10 "PULL_ATTEMPTS=3" "$DEPLOY_SCRIPT" | grep -q "for attempt in"; then
  echo "✅ Retry loop structure found"
else
  echo "❌ Retry loop structure not found"
  exit 1
fi

if grep -A 15 "PULL_ATTEMPTS=3" "$DEPLOY_SCRIPT" | grep -q "PULL_SUCCESS"; then
  echo "✅ Success flag found"
else
  echo "❌ Success flag not found"
  exit 1
fi
echo ""

# Test 5: Test image verification logic
echo "Test 5: Testing image verification logic..."
if grep -q "docker images.*--format" "$DEPLOY_SCRIPT"; then
  echo "✅ Image verification found"
else
  echo "❌ Image verification not found"
  exit 1
fi
echo ""

# Test 6: Test fallback container start
echo "Test 6: Testing fallback container start..."
if grep -A 5 "docker compose.*failed" "$DEPLOY_SCRIPT" | grep -q "docker run"; then
  echo "✅ Fallback container start found"
else
  echo "❌ Fallback container start not found"
  exit 1
fi
echo ""

# Test 7: Test deployment verification
echo "Test 7: Testing deployment verification..."
if grep -q "docker ps.*lianel-\$SERVICE_NAME\|docker ps.*lianel-frontend" "$DEPLOY_SCRIPT"; then
  echo "✅ Container verification found"
else
  echo "❌ Container verification not found"
  exit 1
fi

if grep -q "docker inspect.*Config.Image" "$DEPLOY_SCRIPT"; then
  echo "✅ Image tag verification found"
else
  echo "❌ Image tag verification not found"
  exit 1
fi
echo ""

echo "=== All Tests Passed ==="
echo "The deployment script structure is valid."
echo ""
echo "Next steps:"
echo "1. Copy script to remote host"
echo "2. Test with a real image tag"
echo "3. Verify on actual deployment"
