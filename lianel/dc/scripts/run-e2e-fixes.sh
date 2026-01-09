#!/bin/bash
# Run E2E Fixes: Keycloak Client Scope and Admin Role
# This script runs the necessary fixes for admin access

set -e

cd /root/lianel/dc

# Load environment variables
if [ -f .env ]; then
    set -a
    source .env
    set +a
    echo "✅ Environment variables loaded"
else
    echo "❌ .env file not found"
    exit 1
fi

echo ""
echo "=== Running E2E Fixes ==="
echo ""

# Fix Keycloak Client Scope
echo "1. Fixing Keycloak Client Scope..."
cd scripts
bash fix-keycloak-client-scope-realm-roles.sh

echo ""
echo "2. Admin Role Assignment"
echo "Please provide the username to assign admin role to:"
read -r USERNAME

if [ -n "$USERNAME" ]; then
    echo "Assigning admin role to $USERNAME..."
    bash assign-admin-role.sh "$USERNAME"
else
    echo "⚠️  Skipping admin role assignment"
fi

echo ""
echo "=== Fixes Complete ==="
echo ""
echo "⚠️  IMPORTANT: User must LOG OUT and LOG BACK IN to get new token with roles!"
echo ""
echo "Next steps:"
echo "1. User logs out completely from https://www.lianel.se"
echo "2. User logs back in"
echo "3. Check browser console for: === DASHBOARD ADMIN ROLE DEBUG ==="
echo "4. Verify admin services (Airflow, Grafana) are visible"
