#!/bin/bash
# Assign admin role to a user in Keycloak
# Usage: ./assign-admin-role.sh <user_id_or_username>

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

echo "=== Assigning Admin Role ==="
echo "User identifier: $USER_IDENTIFIER"
echo "Realm: $REALM"
echo ""

# Login as admin
echo "[1/3] Authenticating admin..."
docker exec -e KEYCLOAK_ADMIN="$KEYCLOAK_ADMIN_USER" -e KEYCLOAK_ADMIN_PASSWORD="$KEYCLOAK_ADMIN_PASSWORD" keycloak \
  /opt/keycloak/bin/kcadm.sh config credentials --server "$SERVER_URL" --realm master \
  --user "$KEYCLOAK_ADMIN_USER" --password "$KEYCLOAK_ADMIN_PASSWORD" >/dev/null 2>&1

# Get user ID
echo "[2/3] Finding user..."
if [[ "$USER_IDENTIFIER" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]]; then
    USER_ID="$USER_IDENTIFIER"
else
    USER_JSON=$(docker exec keycloak /opt/keycloak/bin/kcadm.sh get users -r "$REALM" -q username="$USER_IDENTIFIER" 2>/dev/null || echo "[]")
    if [[ "$USER_JSON" == "[]" ]]; then
        echo "❌ User not found: $USER_IDENTIFIER"
        exit 1
    fi
    USER_ID=$(echo "$USER_JSON" | grep -o '"id"\s*:\s*"[^"]*"' | head -n1 | sed 's/.*"id"\s*:\s*"\([^"]*\)".*/\1/')
fi

if [[ -z "$USER_ID" ]]; then
    echo "❌ Could not find user ID"
    exit 1
fi

# Assign admin role
echo "[3/3] Assigning 'admin' realm role..."
set +e
docker exec keycloak /opt/keycloak/bin/kcadm.sh add-roles -r "$REALM" --uid "$USER_ID" --rolename admin 2>&1
RC=$?
set -e

if [[ $RC -eq 0 ]]; then
    echo ""
    echo "✅ Successfully assigned 'admin' role to user $USER_ID"
    echo ""
    echo "⚠️  IMPORTANT: User must log out and log back in to refresh their token!"
else
    echo ""
    echo "❌ Failed to assign admin role. Error code: $RC"
    echo ""
    echo "Possible reasons:"
    echo "  1. The 'admin' realm role doesn't exist. Create it first in Keycloak Admin Console"
    echo "  2. User already has the role assigned"
    echo ""
    echo "To verify, run: ./scripts/verify-user-roles.sh $USER_IDENTIFIER"
    exit 1
fi
