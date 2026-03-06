# Ensure All Services Up on Remote (stock, comp-ai, monitoring, airflow)

Use when containers fail to start, after server restarts, or when you need stock, comp-ai, monitoring, and Airflow all running.

## What gets started (in order)

1. **Network** – `lianel-network`
2. **Infra** – keycloak, nginx (from `docker-compose.infra.yaml`)
3. **Monitoring** – loki, promtail, prometheus, grafana, cadvisor, node-exporter
4. **Airflow** – redis, scheduler, worker, apiserver, triggerer, flower
5. **Stock** – stock-service
6. **Comp-AI** – comp-ai-service
7. **Frontend** – main app
8. **Backend** – profile-service
9. **OAuth2** – oauth2-proxy
10. **Nginx** – config reload

## Run from GitHub Actions

1. **Actions** → **Ensure All Services on Remote** → **Run workflow**.
2. Uses secrets: `REMOTE_HOST`, `REMOTE_USER`, `SSH_PRIVATE_KEY` (optional: `REMOTE_PORT`).
3. Check the job log for the final container status table and summary.

## Run from your machine (SSH)

From repo root, with `REMOTE_HOST` and SSH key:

```bash
REMOTE_HOST=your-server SSH_KEY=~/.ssh/deploy_key bash lianel/dc/scripts/run-ensure-services-remote.sh
```

Or set `REMOTE_HOST`, `REMOTE_USER`, `REMOTE_PORT`, `SSH_KEY` in `.env` at repo root, then:

```bash
bash lianel/dc/scripts/run-ensure-services-remote.sh
```

## Script location

- **Server script** (run on the host): `lianel/dc/scripts/maintenance/ensure-all-services-up-on-server.sh`
- **Local runner** (SSH + pipe script): `lianel/dc/scripts/run-ensure-services-remote.sh`

The server script looks for the compose directory in `/root/lianel/dc` or `/root/hosting-base/lianel/dc` and runs `docker compose up -d` for each stack in dependency order, then prints container status and a short summary of key containers (OK/DOWN).
