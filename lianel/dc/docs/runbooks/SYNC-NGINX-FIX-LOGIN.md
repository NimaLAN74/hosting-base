# Sync Nginx (Fix Login) – when and how to run

**Workflow:** GitHub Actions → **Sync Nginx (Fix Login)** (`.github/workflows/sync-nginx-login.yml`).

**Nginx sync is part of every production deploy.** The workflows **Deploy Frontend**, **Deploy Comp AI Service**, and **Deploy Profile Service** copy `nginx.conf` to the server and reload nginx after deploying. (Deploy Energy Service is paused; see ENERGY-PROJECT-PAUSED.md.) You should not need to run Sync Nginx manually after a normal deploy. Run the **Sync Nginx (Fix Login)** workflow only when:

## When to run (manual Sync Nginx workflow)

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

## If the workflow fails

- **Preflight SSH (retry on timeout) failed** – The server is unreachable from GitHub Actions (firewall, host down, or wrong REMOTE_HOST). Fix 502 by running on the server over SSH (from a machine that can reach the host):
  ```bash
  ssh REMOTE_USER@REMOTE_HOST 'bash -s' < lianel/dc/scripts/fix-502-keycloak-on-server.sh
  ```
  Or on the server directly:
  ```bash
  bash /root/lianel/dc/scripts/fix-502-keycloak-on-server.sh
  # or
  bash /root/hosting-base/lianel/dc/scripts/fix-502-keycloak-on-server.sh
  ```
  Ensure GitHub Actions runners can reach REMOTE_HOST (allow GitHub IP ranges or use a self-hosted runner) if you want the workflow to fix 502 automatically.
- **Apply on remote failed** – Check the run logs: Keycloak may fail to start (e.g. DB password mismatch, OOM). See **ROOT-CAUSE-LOGIN-AIRFLOW-KEYCLOAK.md** for manual recovery (SSH, inspect Keycloak container and DB, restart).
