#!/bin/bash
echo "Checking Rust syntax in profile service..."

# Check for common syntax errors
echo "[1/3] Checking for missing braces..."
if grep -n "fn new.*-> Self$" lianel/dc/profile-service/src/main.rs; then
  echo "  ⚠️  Found potential missing brace"
else
  echo "  ✅ No missing braces found"
fi

echo ""
echo "[2/3] Checking base64 usage..."
if grep -q "base64::engine::general_purpose::URL_SAFE_NO_PAD" lianel/dc/profile-service/src/main.rs; then
  echo "  ✅ base64 usage found"
  echo "  Checking if Engine trait is imported..."
  if grep -q "use base64::Engine" lianel/dc/profile-service/src/main.rs; then
    echo "  ✅ Engine trait imported"
  else
    echo "  ❌ Engine trait not imported!"
  fi
else
  echo "  ⚠️  No base64 usage found"
fi

echo ""
echo "[3/3] Checking for unclosed blocks..."
BRACES=$(grep -o '{' lianel/dc/profile-service/src/main.rs | wc -l)
CLOSING=$(grep -o '}' lianel/dc/profile-service/src/main.rs | wc -l)
echo "  Opening braces: $BRACES"
echo "  Closing braces: $CLOSING"
if [ "$BRACES" -eq "$CLOSING" ]; then
  echo "  ✅ Braces balanced"
else
  echo "  ❌ Braces not balanced! ($BRACES vs $CLOSING)"
fi
