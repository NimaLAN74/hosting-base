#!/bin/bash
echo "Checking for potential compilation errors..."

# Check if all fields in KeycloakTokenClaims are being set
echo "[1] Checking KeycloakTokenClaims struct initialization..."
grep -A 20 "return Ok(KeycloakTokenClaims {" lianel/dc/profile-service/src/main.rs | head -25

echo ""
echo "[2] Checking struct definition..."
grep -A 15 "struct KeycloakTokenClaims" lianel/dc/profile-service/src/main.rs | head -20

echo ""
echo "[3] Checking for missing fields..."
# Count fields in struct
STRUCT_FIELDS=$(grep -A 15 "struct KeycloakTokenClaims" lianel/dc/profile-service/src/main.rs | grep -E "^\s+[a-z_]+:" | wc -l)
echo "Struct fields found: $STRUCT_FIELDS"

# Count fields in initialization
INIT_FIELDS=$(grep -A 35 "return Ok(KeycloakTokenClaims {" lianel/dc/profile-service/src/main.rs | grep -E "^\s+[a-z_]+:" | wc -l)
echo "Initialization fields found: $INIT_FIELDS"
