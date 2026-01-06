#!/bin/bash
echo "Checking for syntax errors..."

# Check for unclosed blocks
echo "[1] Checking for unclosed blocks..."
BRACES=$(grep -o '{' lianel/dc/profile-service/src/main.rs | wc -l)
CLOSING=$(grep -o '}' lianel/dc/profile-service/src/main.rs | wc -l)
echo "Opening braces: $BRACES"
echo "Closing braces: $CLOSING"

# Check base64 usage
echo ""
echo "[2] Checking base64 usage..."
grep -n "general_purpose" lianel/dc/profile-service/src/main.rs | head -5

# Check for common Rust errors
echo ""
echo "[3] Checking for common errors..."
if grep -q "or_else(|_|" lianel/dc/profile-service/src/main.rs; then
  echo "  Found or_else pattern"
  # Check if it's properly closed
  if grep -A 10 "or_else(|_|" lianel/dc/profile-service/src/main.rs | grep -q "});"; then
    echo "  ✅ or_else block appears closed"
  else
    echo "  ⚠️  or_else block might not be closed properly"
  fi
fi
