# Verify FE and BE via pipeline and remote server (no local builds)

**Do not build locally.** Use GitHub Actions to build and deploy. Verify by pipeline status and, when needed, by checking the remote server over SSH.

---

## Who builds what

| What | Pipeline | Builds where | Deploys to |
|------|----------|-------------|------------|
| **Main frontend** (incl. keycloak.js, /stock UI) | **Deploy Frontend to Production** | GitHub Actions (Docker build) | Remote via `deploy-frontend.sh` |
| **Stock monitoring backend** | **Stock Service Backend – CI and Deploy** | GitHub Actions (cargo test, cargo build, Docker build) | Remote via `deploy-stock-service-backend.sh` |

---

## 1. Trigger and verify via pipeline

- **Frontend (latest keycloak.js, stock UI)**  
  - Push to `master`/`main` with changes under:  
    `lianel/dc/frontend/**`, `lianel/dc/nginx/**`, `lianel/dc/docker-compose.frontend.yaml`, etc.  
  - Run: **Deploy Frontend to Production** (or it runs automatically on push).  
  - In GitHub: **Actions** → open the run → confirm job **Build and Deploy Frontend** and steps **Build and push**, **Deploy to Remote Host**, **Verify deployment** are green.

- **Stock monitoring backend (latest auth, config, multi-issuer)**  
  - Push to `master`/`main` with changes under:  
    `lianel/dc/stock-service/backend/**`, `lianel/dc/docker-compose.stock-service.yaml`, etc.  
  - Run: **Stock Service Backend – CI and Deploy** (or it runs automatically on push).  
  - In GitHub: **Actions** → open the run → confirm:  
    - **CI**: **Run tests**, **Build release binary**, **Build and push Docker image** are green.  
    - **CD**: **Deploy to remote host**, **Verify deployment (health)** are green.

If a step is red, fix the code or config and push again; the pipeline is the source of truth for “does it build and deploy”.

---

## 2. Verify on the remote server (SSH)

From repo root, run the verification script (uses key and host from `lianel/dc/scripts/SSH-CONFIG.md`):

```bash
bash lianel/dc/scripts/verify-fe-be-remote.sh
```

Or with explicit env: `REMOTE_HOST=72.60.80.84 REMOTE_USER=root SSH_KEY=~/.ssh/id_ed25519_host bash lianel/dc/scripts/verify-fe-be-remote.sh`

From a machine that has SSH access to the deploy host (e.g. same secrets as the workflow: `REMOTE_HOST`, `REMOTE_USER`, `SSH_PRIVATE_KEY`).

**Frontend (main app + /stock):**

```bash
# Containers
ssh -i <key> <REMOTE_USER>@<REMOTE_HOST> "docker ps --format '{{.Names}} {{.Status}}' | grep -E 'lianel-frontend|nginx'"

# Nginx serves /stock from frontend
ssh -i <key> <REMOTE_USER>@<REMOTE_HOST> "curl -sS -o /dev/null -w '%{http_code}' http://localhost/stock/ -H 'Host: www.lianel.se'"
# Expect 200 (or 302 then 200 via frontend)
```

**Stock monitoring backend:**

```bash
# Container and health
ssh -i <key> <REMOTE_USER>@<REMOTE_HOST> "docker ps --format '{{.Names}} {{.Status}}' | grep stock-service"
ssh -i <key> <REMOTE_USER>@<REMOTE_HOST> "docker exec lianel-stock-service curl -fsS http://localhost:3003/health"
# Expect: ok

# KEYCLOAK_ISSUER_ALT present (for www.lianel.se/auth tokens)
ssh -i <key> <REMOTE_USER>@<REMOTE_HOST> "docker exec lianel-stock-service env | grep KEYCLOAK"
# Should include KEYCLOAK_ISSUER_ALT or KEYCLOAK_URL
```

**Public HTTPS (from your laptop):**

```bash
# Backend health (no auth)
curl -sS -o /dev/null -w '%{http_code}' https://www.lianel.se/api/v1/stock-service/health
# Expect 200

# Protected endpoint without token → 401
curl -sS https://www.lianel.se/api/v1/stock-service/me
# Expect 401 and JSON body with "error"
```

---

## 3. Summary

- **Builds**: Only in GitHub Actions (no `npm run build` or `cargo build` locally for deploy).
- **Deploy**: Pipelines run the deploy scripts on the remote host.
- **Verification**: Green pipeline run + optional SSH and `curl` checks above.
