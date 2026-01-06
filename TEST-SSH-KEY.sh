#!/bin/bash
# Test script for SSH key connection

echo "=== Testing SSH Key Connection ==="
echo ""

# Test basic connection
echo "1. Testing basic SSH connection..."
if ssh -i ~/.ssh/github_actions_deploy -o StrictHostKeyChecking=no -o ConnectTimeout=10 root@72.60.80.84 "echo '✅ Connection successful' && pwd && whoami && hostname"; then
  echo "✅ Basic connection works!"
else
  echo "❌ Connection failed"
  exit 1
fi

echo ""
echo "2. Testing remote directory access..."
ssh -i ~/.ssh/github_actions_deploy -o StrictHostKeyChecking=no root@72.60.80.84 "cd /root/lianel/dc && pwd && ls -la docker-compose*.yaml 2>/dev/null | head -5"

echo ""
echo "3. Testing Docker access..."
ssh -i ~/.ssh/github_actions_deploy -o StrictHostKeyChecking=no root@72.60.80.84 "docker --version && (docker compose version 2>/dev/null || docker-compose --version 2>/dev/null)"

echo ""
echo "4. Checking Docker network..."
ssh -i ~/.ssh/github_actions_deploy -o StrictHostKeyChecking=no root@72.60.80.84 "docker network ls | grep lianel || echo 'No lianel network found'"

echo ""
echo "5. Checking existing images..."
ssh -i ~/.ssh/github_actions_deploy -o StrictHostKeyChecking=no root@72.60.80.84 "docker images | grep -E 'lianel|ghcr' | head -5 || echo 'No lianel images found'"

echo ""
echo "=== Test Complete ==="

