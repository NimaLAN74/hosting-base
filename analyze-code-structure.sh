#!/bin/bash
echo "Analyzing code structure for potential issues..."

# Check if all if blocks are properly closed
echo "[1] Checking if-let blocks..."
grep -n "if let Ok(claims_bytes)" lianel/dc/profile-service/src/main.rs | while read line; do
  LINE_NUM=$(echo $line | cut -d: -f1)
  echo "  Found at line $LINE_NUM"
  # Check if there's a closing brace after
  sed -n "${LINE_NUM},$((LINE_NUM+30))p" lianel/dc/profile-service/src/main.rs | grep -n "}" | head -1
done

echo ""
echo "[2] Checking for unreachable code..."
# Check if there are any statements after return that would be unreachable
grep -B2 -A5 "return Ok(claims)" lianel/dc/profile-service/src/main.rs | head -15

echo ""
echo "[3] Checking variable usage..."
# Check if claims_json is used correctly
grep -B3 -A10 "let claims_json" lianel/dc/profile-service/src/main.rs | head -20
