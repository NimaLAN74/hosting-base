#!/bin/bash
echo "Checking base64 API usage..."

# Check if we're using the correct base64 API for version 0.22
echo "[1] Checking base64 version in Cargo.toml..."
grep "base64" lianel/dc/profile-service/Cargo.toml

echo ""
echo "[2] Checking base64 usage in code..."
grep -n "base64::" lianel/dc/profile-service/src/main.rs

echo ""
echo "[3] Checking if Engine trait is imported..."
grep -n "use base64" lianel/dc/profile-service/src/main.rs

echo ""
echo "Note: base64 0.22 uses Engine trait, and URL_SAFE_NO_PAD should work."
echo "If there's an error, it might be:"
echo "  - Missing Engine trait import"
echo "  - Wrong API usage"
echo "  - Need to use prelude::*"
