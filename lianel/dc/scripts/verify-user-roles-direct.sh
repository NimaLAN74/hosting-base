#!/bin/bash
# Direct verification using docker exec - can be run remotely
# Usage: Run this on the remote host or via: ssh user@host "bash -s" < verify-user-roles-direct.sh

USER_ID="03aa0110-1f64-44dd-96f7-875713c1d15b"
REALM="lianel"
SERVER_URL="http://localhost:8080"

echo "=== Verifying User Roles in Keycloak ==="
echo "User ID: $USER_ID"
echo "Realm: $REALM"
echo ""

# Check if keycloak container exists
if ! docker ps --format '{{.Names}}' | grep -q '^keycloak$'; then
    echo "❌ Keycloak container 'keycloak' not running"
    exit 1
fi

# Load environment variables
if [ -f /root/lianel/dc/.env ]; then
    export $(cat /root/lianel/dc/.env | grep -v '^#' | xargs)
fi

# Login as admin
echo "[1/3] Authenticating admin..."
docker exec -e KEYCLOAK_ADMIN="$KEYCLOAK_ADMIN_USER" -e KEYCLOAK_ADMIN_PASSWORD="$KEYCLOAK_ADMIN_PASSWORD" keycloak \
  /opt/keycloak/bin/kcadm.sh config credentials --server "$SERVER_URL" --realm master \
  --user "$KEYCLOAK_ADMIN_USER" --password "$KEYCLOAK_ADMIN_PASSWORD" >/dev/null 2>&1

# Get user info
echo "[2/3] Getting user information..."
USER_JSON=$(docker exec keycloak /opt/keycloak/bin/kcadm.sh get users -r "$REALM" "$USER_ID" 2>/dev/null || echo "{}")

if [[ "$USER_JSON" == "{}" ]]; then
    echo "❌ User not found: $USER_ID"
    exit 1
fi

USERNAME=$(echo "$USER_JSON" | grep -o '"username"\s*:\s*"[^"]*"' | head -n1 | sed 's/.*"username"\s*:\s*"\([^"]*\)".*/\1/' || echo "N/A")
EMAIL=$(echo "$USER_JSON" | grep -o '"email"\s*:\s*"[^"]*"' | head -n1 | sed 's/.*"email"\s*:\s*"\([^"]*\)".*/\1/' || echo "N/A")

echo "User ID: $USER_ID"
echo "Username: $USERNAME"
echo "Email: $EMAIL"
echo ""

# Get realm roles
echo "[3/3] Getting realm roles..."
ROLES_JSON=$(docker exec keycloak /opt/keycloak/bin/kcadm.sh get users -r "$REALM" "$USER_ID"/role-mappings/realm 2>/dev/null || echo "[]")

if [[ "$ROLES_JSON" == "[]" ]] || [[ -z "$ROLES_JSON" ]]; then
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
HAS_ADMIN=false
echo "$ROLE_NAMES" | while read -r role; do
    if [[ -n "$role" ]]; then
        echo "  - $role"
        if [[ "$role" == "admin" ]] || [[ "$role" == "Admin" ]] || [[ "$role" == "ADMIN" ]] || [[ "$role" == "realm-admin" ]]; then
            echo "    ✓ This is an admin role!"
            HAS_ADMIN=true
        fi
    fi
done

echo ""
echo "=== Summary ==="
if echo "$ROLE_NAMES" | grep -qiE "^(admin|realm-admin)$"; then
    echo "✅ User HAS admin role assigned"
    echo ""
    echo "Note: If the user still can't see admin services:"
    echo "  1. User must log out and log back in to refresh the token"
    echo "  2. Check browser console for role debug logs"
else
    echo "❌ User does NOT have admin role assigned"
    echo ""
    echo "To assign the admin role, run:"
    echo "  docker exec keycloak /opt/keycloak/bin/kcadm.sh add-roles -r $REALM --uid $USER_ID --rolename admin"
fi
