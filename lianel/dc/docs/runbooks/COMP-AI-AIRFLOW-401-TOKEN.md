# Comp-AI DAG: 401 Unauthorized or Non-ASCII Token

**Symptoms:** `comp_ai_control_tests` or `comp_ai_alerts` DAG fails with:
- `HTTPError: 401 Client Error: Unauthorized for url: .../api/v1/controls/gaps` (or `/api/v1/tests`)
- `WARNING - COMP_AI_TOKEN or COMP_AI_BASE_URL contained non-ASCII characters`

---

## Immediate fix (recommended: client credentials)

**Option A – GitHub Secrets (then re-sync DAGs)**  
Add these repository secrets, then run **Sync Airflow DAGs to Remote Host** so it sets the Variables in Airflow:

- `COMP_AI_KEYCLOAK_URL` – Keycloak base URL reachable from the server (e.g. `https://www.lianel.se/auth` or `http://keycloak:8080` if Airflow runs in Docker on same host)
- `COMP_AI_CLIENT_ID` – e.g. `comp-ai-service`
- `COMP_AI_CLIENT_SECRET` – client secret for that Keycloak client  
Optionally: `COMP_AI_KEYCLOAK_REALM` (default `lianel`).  
Keep `COMP_AI_BASE_URL` set (e.g. `http://lianel-comp-ai-service:3002`).

**Option B – Set Airflow Variables manually**  
In Airflow UI: Admin → Variables. Add:

- `COMP_AI_KEYCLOAK_URL` – e.g. `https://www.lianel.se/auth`
- `COMP_AI_CLIENT_ID` – `comp-ai-service`
- `COMP_AI_CLIENT_SECRET` – (secret from Keycloak for comp-ai-service client)
- `COMP_AI_KEYCLOAK_REALM` – `lianel` (optional)

Ensure `COMP_AI_BASE_URL` is set (e.g. `http://lianel-comp-ai-service:3002`). The DAG will fetch a fresh token each run and 401 from expiry goes away.

---

**Causes:**
1. **Expired or missing token** – `COMP_AI_TOKEN` is not set, or it expired (Keycloak access tokens are short-lived). Using client_credentials avoids this.
2. **Non-ASCII in token** – The Airflow Variable `COMP_AI_TOKEN` was set with a non-ASCII character (e.g. ellipsis `…`). The client strips non-ASCII; if the token was corrupted, it becomes invalid → 401.

**If you prefer a stored token instead of client credentials:**

### 1. Refresh the stored token (ASCII-safe)

On the server where Airflow runs:

```bash
cd /root/lianel/dc  # or your repo path
source .env  # or set KEYCLOAK_ADMIN_USER, KEYCLOAK_ADMIN_PASSWORD, COMP_AI_KEYCLOAK_CLIENT_SECRET
bash scripts/maintenance/set-comp-ai-airflow-token.sh
```

The script now strips non-ASCII from the token before setting the Variable. Ensure Keycloak client has a long enough access token lifespan (e.g. 3600s) if you rely on stored token only:

```bash
bash scripts/maintenance/keycloak-comp-ai-extend-token-lifespan.sh
```

### 2. GitHub Actions sync – sanitize secret

If the sync workflow sets `COMP_AI_TOKEN` from a repo secret, ensure the secret value contains **only ASCII**. The workflow now sanitizes the token before setting the Variable; if the secret was wrong, re-save the secret with a token that has no special Unicode characters (e.g. copy the token again from Keycloak or from the script output).

**Verify:** After fixing, trigger the DAG manually and check logs for no 401 and no non-ASCII warning.
