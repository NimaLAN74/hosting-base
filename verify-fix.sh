#!/bin/bash
echo "Verifying the fix..."

# Count fields in struct
STRUCT_FIELDS=$(grep -A 15 "struct KeycloakTokenClaims" lianel/dc/profile-service/src/main.rs | grep -E "^\s+[a-z_]+:" | wc -l)

# Count fields in initialization
INIT_FIELDS=$(grep -A 40 "return Ok(KeycloakTokenClaims {" lianel/dc/profile-service/src/main.rs | grep -E "^\s+[a-z_]+:" | wc -l)

echo "Struct fields: $STRUCT_FIELDS"
echo "Initialization fields: $INIT_FIELDS"

if [ "$STRUCT_FIELDS" -eq "$INIT_FIELDS" ]; then
  echo "✅ All fields are initialized!"
else
  echo "❌ Field count mismatch!"
fi

# Check if realm_access is present
if grep -q "realm_access:" lianel/dc/profile-service/src/main.rs | grep -A 2 "return Ok(KeycloakTokenClaims"; then
  echo "✅ realm_access field found in initialization"
else
  echo "Checking manually..."
  grep -A 40 "return Ok(KeycloakTokenClaims {" lianel/dc/profile-service/src/main.rs | grep "realm_access"
fi
