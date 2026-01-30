# Diagnosis: Keycloak 502 and Ollama Not Running

**Date**: January 2026  
**Scope**: Root cause only — no code or config changes in this doc.

---

## 1. Keycloak 502 Bad Gateway

### What you see

- **GET https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth** → **502 Bad Gateway** (42 ms).

### Root cause (confirmed on server)

1. **Keycloak is failing to start**  
   Logs show:
   ```text
   FATAL: password authentication failed for user "keycloak"
   Failed to obtain JDBC connection
   Failed to start server in (production) mode
   ```
   So Keycloak never listens on 8080 → nginx gets “connection refused” → **502**.

2. **Why Keycloak was recreated**  
   When we ran on the server:
   ```bash
   docker compose -f docker-compose.infra.yaml -f docker-compose.comp-ai.yaml up -d comp-ai-service
   ```
   Docker Compose **recreated the Keycloak container** as well (output showed “Container keycloak Recreated”).  
   Reason: **comp-ai-service** has `depends_on: keycloak`, and keycloak is in the same project (infra). Compose may treat keycloak as part of the “up” and recreated it (e.g. after pulling images or env changes).

3. **Why Keycloak fails after recreate**  
   After recreate, Keycloak uses the **current** env from the compose + `.env` on the server:
   - `KC_DB_USERNAME=keycloak`
   - `KC_DB_PASSWORD=keycloak` (from `KEYCLOAK_DB_PASSWORD`)
   - `KC_DB_URL=jdbc:postgresql://172.18.0.1:5432/keycloak`

   Postgres is rejecting that user/password. So either:
   - The **password for user `keycloak` in Postgres** is not `keycloak`, or  
   - The **user or database** was created with different credentials earlier.

So the 502 is **not** caused by nginx or by our nginx/config changes. It is caused by **Keycloak failing to start** because of **Postgres authentication failure for user `keycloak`** after Keycloak was recreated.

### What to do

- **Option 0 – Run the pipeline (recommended)**  
  In GitHub: **Actions → "Fix Keycloak 502 on Remote" → Run workflow**.  
  This SSHs to the server, runs `set-keycloak-db-password-on-server.sh` (syncs Postgres `keycloak` user password from `.env`), brings up Keycloak, reloads nginx, and verifies auth.lianel.se. Requires `REMOTE_HOST`, `REMOTE_USER`, `SSH_PRIVATE_KEY` (and optionally `REMOTE_PORT`) in repository secrets.

- **Option A – Align Postgres with current .env**  
  On the server, ensure the Postgres user `keycloak` has the same password as in your `.env` (e.g. `KEYCLOAK_DB_PASSWORD`). If needed, reset the password:
  ```bash
  # On server, connect to Postgres and set keycloak user password to match .env
  # (exact command depends on how you run Postgres)
  ```
  Then restart Keycloak:  
  `docker start keycloak` (or `docker compose -f docker-compose.infra.yaml up -d keycloak`).

- **Option B – Align .env with Postgres**  
  If the `keycloak` user in Postgres has a different password, set `KEYCLOAK_DB_PASSWORD` in the server’s `.env` to that password, then recreate Keycloak so it picks up the new env:
  ```bash
  docker compose -f docker-compose.infra.yaml up -d keycloak
  ```

- **Avoid recreating Keycloak when deploying only Comp-AI**  
  Later, you can change **comp-ai** so it does **not** have `depends_on: keycloak` (Comp-AI only needs Keycloak at runtime for token validation). Then `up -d comp-ai-service` will not recreate Keycloak. That would be a separate, careful change.

---

## 2. Ollama “not running”

### What you see

- Comp-AI 503 “Local model unavailable” when Ollama is configured but not running.

### Root cause (confirmed on server)

1. **No Ollama container**  
   `docker ps -a --filter name=ollama` shows **no** container. Ollama was never started on this host.

2. **Why**  
   In `docker-compose.comp-ai.yaml`, the **ollama** service is under the **profile `local-model`**:
   ```yaml
   ollama:
     ...
     profiles:
       - local-model
   ```
   So Ollama only starts if you run:
   ```bash
   docker compose -f docker-compose.comp-ai.yaml --profile local-model up -d
   ```
   (or include `--profile local-model` when using infra + comp-ai).  
   Normal `up -d comp-ai-service` does **not** start Ollama.

3. **Comp-AI on this server**  
   On the server, `COMP_AI_OLLAMA_URL` and `COMP_AI_OLLAMA_MODEL` are **empty**, so Comp-AI is using **mock** and does not call Ollama. So 503 would only happen if you had set those env vars (e.g. in `.env`) without starting Ollama; the “Ollama not running” part is simply that the Ollama container was never brought up (profile not used).

### What to do (no changes from me)

- **If you want to use Ollama**  
   On the server:
   ```bash
   cd /root/hosting-base/lianel/dc   # or your path
   # Set in .env: COMP_AI_OLLAMA_URL=http://ollama:11434, COMP_AI_OLLAMA_MODEL=tinyllama
   docker compose -f docker-compose.infra.yaml -f docker-compose.comp-ai.yaml --profile local-model up -d ollama
   docker exec lianel-ollama ollama pull tinyllama
   # Restart comp-ai-service so it picks up COMP_AI_OLLAMA_* from .env
   docker compose -f docker-compose.infra.yaml -f docker-compose.comp-ai.yaml up -d comp-ai-service
   ```

- **If you don’t want Ollama**  
  Leave `COMP_AI_OLLAMA_URL` / `COMP_AI_OLLAMA_MODEL` unset (or leave Ollama profile off). Comp-AI will keep using mock (or hosted API when you add it). With `COMP_AI_OLLAMA_FALLBACK_TO_MOCK=true`, even if Ollama is set but down, the API returns mock instead of 503 once the new image is deployed.

---

## 3. Summary

| Issue | Root cause | Next step (you) |
|-------|------------|------------------|
| **Keycloak 502** | Keycloak was recreated; Postgres rejects user `keycloak` with the password from current `.env`. | Fix DB password (Postgres ↔ `.env`) and restart Keycloak; optionally remove `depends_on: keycloak` from comp-ai later. |
| **Ollama not running** | Ollama service is under profile `local-model` and was never started. | Start with `--profile local-model` and pull a model if you want Ollama; otherwise leave as is. |

No code or config changes were made in this diagnosis; only server checks and this doc.
