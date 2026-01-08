#!/bin/bash
# Verify user roles in Keycloak
# Usage: ./verify-user-roles.sh <user_id_or_username>

set -euo pipefail

USER_IDENTIFIER=${1:-}
REALM=${REALM:-lianel}
SERVER_URL=${SERVER_URL:-http://localhost:8080}

if [[ -z "$USER_IDENTIFIER" ]]; then
    echo "Usage: $0 <user_id_or_username>" >&2
    echo "Example: $0 03aa0110-1f64-44dd-96f7-875713c1d15b" >&2
    exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required" >&2
    exit 1
fi

if ! docker ps --format '{{.Names}}' | grep -q '^keycloak$'; then
    echo "Keycloak container 'keycloak' not running. Start infra first." >&2
    exit 1
fi

# Load environment variables if .env exists
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

echo "=== Verifying User Roles in Keycloak ==="
echo "User identifier: $USER_IDENTIFIER"
echo "Realm: $REALM"
echo ""

# Login as admin to master realm
echo "[1/3] Authenticating admin..."
docker exec -e KEYCLOAK_ADMIN="$KEYCLOAK_ADMIN_USER" -e KEYCLOAK_ADMIN_PASSWORD="$KEYCLOAK_ADMIN_PASSWORD" keycloak \
  /opt/keycloak/bin/kcadm.sh config credentials --server "$SERVER_URL" --realm master \
  --user "$KEYCLOAK_ADMIN_USER" --password "$KEYCLOAK_ADMIN_PASSWORD" >/dev/null 2>&1

# Get user info
echo "[2/3] Getting user information..."
if [[ "$USER_IDENTIFIER" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]]; then
    # It's a UUID (user ID)
    USER_ID="$USER_IDENTIFIER"
    USER_JSON=$(docker exec keycloak /opt/keycloak/bin/kcadm.sh get users -r "$REALM" "$USER_ID" 2>/dev/null || echo "{}")
else
    # It's a username
    USER_JSON=$(docker exec keycloak /opt/keycloak/bin/kcadm.sh get users -r "$REALM" -q username="$USER_IDENTIFIER" 2>/dev/null || echo "[]")
    if [[ "$USER_JSON" == "[]" ]]; then
        echo "❌ User not found: $USER_IDENTIFIER"
        exit 1
    fi
    # Extract user ID from JSON
    USER_ID=$(echo "$USER_JSON" | grep -o '"id"\s*:\s*"[^"]*"' | head -n1 | sed 's/.*"id"\s*:\s*"\([^"]*\)".*/\1/')
fi

if [[ -z "$USER_ID" ]]; then
    echo "❌ Could not find user ID for: $USER_IDENTIFIER"
    exit 1
fi

# Extract username and email
USERNAME=$(echo "$USER_JSON" | grep -o '"username"\s*:\s*"[^"]*"' | head -n1 | sed 's/.*"username"\s*:\s*"\([^"]*\)".*/\1/' || echo "N/A")
EMAIL=$(echo "$USER_JSON" | grep -o '"email"\s*:\s*"[^"]*"' | head -n1 | sed 's/.*"email"\s*:\s*"\([^"]*\)".*/\1/' || echo "N/A")

echo "User ID: $USER_ID"
echo "Username: $USERNAME"
echo "Email: $EMAIL"
echo ""

# Get realm roles
echo "[3/3] Getting realm roles..."
ROLES_JSON=$(docker exec keycloak /opt/keycloak/bin/kcadm.sh get users -r "$REALM" "$USER_ID"/role-mappings/realm 2>/dev/null || echo "[]")

if [[ "$ROLES_JSON" == "[]" ]]; then
    echo "❌ No realm roles assigned to this user"
    echo ""
    echo "To assign the admin role, run:"
    echo "  docker exec keycloak /opt/keycloak/bin/kcadm.sh add-roles -r $REALM --uid $USER_ID --rolename admin"
    exit 1
fi

# Extract role names
ROLE_NAMES=$(echo "$ROLES_JSON" | grep -o '"name"\s*:\s*"[^"]*"' | sed 's/.*"name"\s*:\s*"\([^"]*\)".*/\1/' || echo "")

if [[ -z "$ROLE_NAMES" ]]; then
    echo "❌ Could not extract role names"
    echo "Raw JSON: $ROLES_JSON"
    exit 1
fi

echo "✅ Realm roles assigned:"
echo "$ROLE_NAMES" | while read -r role; do
    if [[ -n "$role" ]]; then
        echo "  - $role"
        if [[ "$role" == "admin" ]] || [[ "$role" == "Admin" ]] || [[ "$role" == "ADMIN" ]] || [[ "$role" == "realm-admin" ]]; then
            echo "    ✓ This is an admin role!"
        fi
    fi
done

echo ""
echo "=== Summary ==="
HAS_ADMIN=$(echo "$ROLE_NAMES" | grep -iE "^(admin|realm-admin)$" || echo "")
if [[ -n "$HAS_ADMIN" ]]; then
    echo "✅ User HAS admin role assigned"
    echo ""
    echo "Note: If the user still can't see admin services in the frontend:"
    echo "  1. User needs to log out and log back in to refresh the token"
    echo "  2. Check browser console for role debug logs"
    echo "  3. Verify the token includes realm_access.roles array"
else
    echo "❌ User does NOT have admin role assigned"
    echo ""
    echo "To assign the admin role, run:"
    echo "  docker exec keycloak /opt/keycloak/bin/kcadm.sh add-roles -r $REALM --uid $USER_ID --rolename admin"
fi
