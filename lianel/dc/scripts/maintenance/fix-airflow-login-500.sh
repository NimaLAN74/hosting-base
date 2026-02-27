#!/usr/bin/env bash
# Fix or diagnose Airflow /auth/login/ 500 Internal Server Error.
# Run on the host where Airflow runs (e.g. ssh REMOTE "bash -s" < fix-airflow-login-500.sh).
set -euo pipefail

DC_DIR="${DC_DIR:-}"
for d in /root/hosting-base/lianel/dc /root/lianel/dc .; do
  [ -f "${d}/docker-compose.airflow.yaml" ] && DC_DIR="$d" && break
done
[ -n "$DC_DIR" ] || { echo "Could not find docker-compose.airflow.yaml"; exit 1; }
cd "$DC_DIR"

CONTAINER=$(docker ps -q -f name=airflow-apiserver | head -1)
[ -n "$CONTAINER" ] || { echo "No running airflow-apiserver container found"; exit 1; }

echo "=== Airflow apiserver container: $CONTAINER ==="
echo ""
echo "--- Last 80 lines of logs (look for traceback, PendingRollbackError, OAuth, Keycloak) ---"
docker logs "$CONTAINER" 2>&1 | tail -80
echo ""
echo "--- Quick fix: restart apiserver (clears DB connection / config load issues) ---"
echo "  cd $DC_DIR"
echo "  docker compose -f docker-compose.airflow.yaml restart airflow-apiserver"
echo ""
if [[ "${RESTART:-}" == "1" || "${1:-}" == "--restart" ]]; then
  echo "Restarting airflow-apiserver (RESTART=1 or --restart)."
  docker compose -f docker-compose.airflow.yaml restart airflow-apiserver
  echo "Restarted. Wait ~30s then try https://airflow.lianel.se/auth/login/"
else
  read -r -p "Restart airflow-apiserver now? [y/N] " ans 2>/dev/null || true
  if [[ "${ans,,}" == "y" || "${ans,,}" == "yes" ]]; then
    docker compose -f docker-compose.airflow.yaml restart airflow-apiserver
    echo "Restarted. Wait ~30s then try https://airflow.lianel.se/auth/login/"
  else
    echo "To restart non-interactively: RESTART=1 $0  (or run the restart command above)."
  fi
fi
