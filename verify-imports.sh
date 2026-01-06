#!/bin/bash
echo "Verifying all imports and usage..."

echo "[1] Checking base64 import..."
grep "use base64" lianel/dc/profile-service/src/main.rs

echo ""
echo "[2] Checking if general_purpose is used correctly..."
if grep -q "general_purpose::" lianel/dc/profile-service/src/main.rs; then
  if grep -q "use base64.*general_purpose" lianel/dc/profile-service/src/main.rs; then
    echo "  ✅ general_purpose is imported"
  else
    echo "  ❌ general_purpose is used but not imported!"
    echo "  Current import:"
    grep "use base64" lianel/dc/profile-service/src/main.rs
  fi
else
  echo "  ⚠️  general_purpose not found in code"
fi

echo ""
echo "[3] Checking for any base64::engine::general_purpose usage (should be removed)..."
if grep -q "base64::engine::general_purpose" lianel/dc/profile-service/src/main.rs; then
  echo "  ❌ Found old base64::engine::general_purpose usage!"
  grep -n "base64::engine::general_purpose" lianel/dc/profile-service/src/main.rs
else
  echo "  ✅ No old base64::engine::general_purpose usage found"
fi
