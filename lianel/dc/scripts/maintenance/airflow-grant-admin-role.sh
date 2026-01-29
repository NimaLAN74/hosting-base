#!/bin/bash
# Grant Airflow Admin role to an existing user (fixes 403 when Keycloak admin has no Airflow permission).
# Run on the host where Airflow runs. Requires container name and username.
#
# Usage:
#   AIRFLOW_CONTAINER=dc-airflow-apiserver-1 bash scripts/maintenance/airflow-grant-admin-role.sh <username>
#   bash scripts/maintenance/airflow-grant-admin-role.sh <username>   # auto-detect container
#
# To list Airflow usernames first:
#   docker exec <apiserver-container> airflow users list

set -e

USERNAME="${1:-}"
CONTAINER="${AIRFLOW_CONTAINER:-}"

if [ -z "$USERNAME" ]; then
  echo "Usage: $0 <airflow_username>"
  echo "Example: $0 john@example.com"
  echo ""
  echo "To list existing users: docker exec <apiserver> airflow users list"
  exit 1
fi

if [ -z "$CONTAINER" ]; then
  CONTAINER=$(docker ps --format '{{.Names}}' | grep -E 'airflow.*apiserver|apiserver.*airflow' | head -1)
fi
if [ -z "$CONTAINER" ]; then
  echo "ERROR: Airflow apiserver container not found. Set AIRFLOW_CONTAINER or run from host where Airflow runs."
  exit 1
fi

echo "Adding Admin role to user '$USERNAME' in container $CONTAINER..."
docker exec "$CONTAINER" airflow users add-role -u "$USERNAME" -r Admin
echo "Done. User $USERNAME now has Admin role (trigger DAGs, Admin menu, edit users)."
