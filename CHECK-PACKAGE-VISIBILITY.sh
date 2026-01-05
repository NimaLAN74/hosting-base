#!/bin/bash
# Script to check package visibility and format

echo "=== Checking Package Visibility ==="
echo ""

PACKAGES=(
  "nimalan74/hosting-base/lianel-frontend"
  "nimalan74/hosting-base/lianel-profile-service"
  "nimalan74/lianel-frontend"
  "nimalan74/lianel-profile-service"
  "nimalan74/hosting-base-lianel-frontend"
  "nimalan74/hosting-base-lianel-profile-service"
)

for PACKAGE in "${PACKAGES[@]}"; do
  echo "Testing: $PACKAGE"
  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "https://ghcr.io/v2/${PACKAGE}/manifests/latest" 2>/dev/null)
  
  if [ "$HTTP_CODE" == "200" ]; then
    echo "  ✅ PUBLIC - HTTP $HTTP_CODE"
  elif [ "$HTTP_CODE" == "401" ]; then
    echo "  ❌ PRIVATE - HTTP $HTTP_CODE (authentication required)"
  elif [ "$HTTP_CODE" == "404" ]; then
    echo "  ⚠️  NOT FOUND - HTTP $HTTP_CODE"
  else
    echo "  ❓ UNKNOWN - HTTP $HTTP_CODE"
  fi
  echo ""
done

echo "=== Summary ==="
echo "If all return 401, packages are private"
echo "If all return 404, package names are wrong"
echo "If any return 200, that's the correct format and it's public"

