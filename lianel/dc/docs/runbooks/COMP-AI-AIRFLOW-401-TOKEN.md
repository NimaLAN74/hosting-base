# Comp-AI DAG: 401 Unauthorized or Non-ASCII Token

**Symptoms:** `comp_ai_control_tests` or `comp_ai_alerts` DAG fails with:
- `HTTPError: 401 Client Error: Unauthorized for url: .../api/v1/controls/gaps` (or `/api/v1/tests`)
- `WARNING - COMP_AI_TOKEN or COMP_AI_BASE_URL contained non-ASCII characters`

**Causes:**
1. **Non-ASCII in token** – The Airflow Variable `COMP_AI_TOKEN` (or GitHub secret) was set with a non-ASCII character (e.g. ellipsis `…` instead of `...`). The client strips non-ASCII for HTTP headers; if the token was corrupted when set, it becomes invalid → 401.
2. **Expired token** – Keycloak access token has a short lifespan (e.g. 5 min); the stored token is no longer valid.

**Fixes (choose one):**

### 1. Use client credentials (recommended – no expiry issues)

Set these Airflow Variables so the DAG fetches a fresh token each run:

- `COMP_AI_KEYCLOAK_URL` – e.g. `https://auth.lianel.se`
- `COMP_AI_CLIENT_ID` – e.g. `comp-ai-service`
- `COMP_AI_CLIENT_SECRET` – client secret for the Comp-AI Keycloak client
- Optional: `COMP_AI_KEYCLOAK_REALM` – default `lianel`

Keep `COMP_AI_BASE_URL` set. You can leave `COMP_AI_TOKEN` set or remove it; the client will use client_credentials when the Keycloak variables are present.

### 2. Refresh the stored token (ASCII-safe)

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

### 3. GitHub Actions sync – sanitize secret

If the sync workflow sets `COMP_AI_TOKEN` from a repo secret, ensure the secret value contains **only ASCII**. The workflow now sanitizes the token before setting the Variable; if the secret was wrong, re-save the secret with a token that has no special Unicode characters (e.g. copy the token again from Keycloak or from the script output).

**Verify:** After fixing, trigger the DAG manually and check logs for no 401 and no non-ASCII warning.
