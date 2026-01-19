#!/bin/bash
# Restart monitoring containers on remote host
# This applies Grafana configuration changes and dashboard updates

set -e

REPO_DIR="${REPO_DIR:-/root/hosting-base/lianel/dc}"

echo "=== Restarting Monitoring Containers ==="
echo "Repository: $REPO_DIR"
echo ""

cd "$REPO_DIR" || exit 1

# Pull latest changes
echo "Pulling latest changes from git..."
git pull origin master || echo "Warning: Could not pull latest changes"

# Restart Grafana to apply configuration and dashboard changes
echo ""
echo "Restarting Grafana..."
docker-compose -f docker-compose.monitoring.yaml restart grafana

# Wait a moment for Grafana to start
sleep 3

# Check Grafana status
echo ""
echo "Checking Grafana status..."
docker ps --filter "name=grafana" --format "table {{.Names}}\t{{.Status}}\t{{.Image}}"

echo ""
echo "=== Restart Complete ==="
echo ""
echo "Grafana should now have:"
echo "  - Updated dashboard queries with correct column names"
echo "  - Updated OAuth configuration for admin access"
echo "  - Connections feature enabled"
echo ""
echo "Next steps:"
echo "1. Log out and log back in to Grafana to get fresh token"
echo "2. Verify Configuration menu is visible"
echo "3. Check Data Quality dashboard shows all columns correctly"
