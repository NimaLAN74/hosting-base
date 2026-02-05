# Sync Nginx (Fix Login) – when and how to run

**Workflow:** GitHub Actions → **Sync Nginx (Fix Login)** (`.github/workflows/sync-nginx-login.yml`).

## When to run

- **502 Bad Gateway** on `https://www.lianel.se/auth/` or login page not loading (Keycloak down or unreachable).
- After changing `lianel/dc/nginx/config/nginx.conf` or `lianel/dc/docker-compose.infra.yaml` and you want them applied on the server (Keycloak hostname, auth routes).
- After a server reboot or Keycloak container stop – to bring Keycloak back up with the correct DB password and hostname.

## What it does

1. SSHs to the remote host and:
   - Pulls latest nginx.conf and docker-compose.infra.yaml from git.
   - Syncs Keycloak DB password (script `set-keycloak-db-password-on-server.sh`).
   - Starts Keycloak (`docker compose up -d keycloak`) and waits 45s.
   - Sets KC_HOSTNAME / KC_HOSTNAME_ADMIN in compose.
   - Reloads nginx.
   - Runs Keycloak frontendUrl fix script if present.
2. Copies the **runner’s** nginx.conf to the server (so the exact version from the workflow run is deployed) and reloads nginx again.
3. Verifies theme CSS (with retries) and runs E2E login flow test.

## How to run

1. **GitHub:** Repo → **Actions** → **Sync Nginx (Fix Login)** → **Run workflow** → **Run workflow**.
2. **CLI:** From repo root: `gh workflow run "Sync Nginx (Fix Login)"`.

## Required secrets

- `REMOTE_HOST`, `REMOTE_USER`, `REMOTE_PORT` (optional, default 22).
- Deploy key for SSH (e.g. `DEPLOY_KEY` or the key configured in the workflow).

## If it still fails

- Check the run logs for the “Apply on remote” step: Keycloak may fail to start (e.g. DB password mismatch, OOM).
- See **ROOT-CAUSE-LOGIN-AIRFLOW-KEYCLOAK.md** for manual recovery (SSH, inspect Keycloak container and DB, restart).
