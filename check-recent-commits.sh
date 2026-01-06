#!/bin/bash
echo "Recent commits that might have caused pipeline failure:"
git log --oneline -5

echo ""
echo "Files changed in last commit:"
git show --name-only --pretty=format: HEAD | head -10

echo ""
echo "Checking for common Rust compilation issues..."
echo "[1] Checking for missing semicolons or unclosed blocks..."
if grep -n "if let Ok" lianel/dc/profile-service/src/main.rs | grep -v ";$" | head -5; then
  echo "  Found potential issues"
else
  echo "  ✅ No obvious syntax issues"
fi
