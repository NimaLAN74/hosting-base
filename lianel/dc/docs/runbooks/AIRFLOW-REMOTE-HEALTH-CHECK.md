# Airflow & Keycloak Remote Health Check

**Date**: January 2026  
**Server**: 72.60.80.84 (root, key `~/.ssh/id_ed25519_host`)

---

## 1. Why `dc-airflow-worker-1` stays in "Created" status

**Symptom**: `docker compose -f docker-compose.airflow.yaml ps -a` shows `dc-airflow-worker-1` with status **Created** (not Running).

**Cause**: The worker has `depends_on: airflow-apiserver: condition: service_healthy`. If at first `compose up` the apiserver was not yet healthy (or the worker was never started in that run), Docker creates the worker container but does not start it. Compose does not automatically retry starting dependent containers that were left in Created state.

**Fix** (on the server):

```bash
cd /root/lianel/dc
# Option A: Start the worker container directly
docker start dc-airflow-worker-1

# Option B: Let compose bring it up (will also wait for init if needed)
docker compose -f docker-compose.airflow.yaml up -d airflow-worker
```

Then verify:

```bash
docker ps -a --filter name=airflow-worker --format 'table {{.Names}}\t{{.Status}}'
```

---

## 2. Keycloak logs after failed Airflow login

After a failed Airflow SSO attempt, Keycloak logs (`docker logs keycloak 2>&1 | tail -100`) typically show:

| Keycloak log | Meaning |
|--------------|--------|
| `LOGIN_ERROR` … `clientId="airflow"` … `error="invalid_redirect_uri"` `redirect_uri="http://airflow.lianel.se/..."` | Airflow sent **http** redirect_uri. Fix: ensure nginx sets `X-Forwarded-Proto https` for `/auth/` and ProxyFix is applied in `webserver_config.py`. |
| `PKCE enforced Client without code challenge method` … `error="invalid_request"` `reason="Missing parameter: code_challenge_method"` `redirect_uri="https://airflow.lianel.se/..."` | Keycloak **airflow** client still has PKCE required. Fix: run `scripts/keycloak-setup/create-airflow-keycloak-client.sh` on the server (sets `pkce.code.challenge.method` to `''`). |
| `LOGIN_ERROR` … `clientId="frontend-client"` … `error="invalid_redirect_uri"` | Frontend/client redirect URI not in Keycloak client’s Valid Redirect URIs (separate from Airflow). |

---

## 3. Airflow apiserver logs after failed login

After a failed login, apiserver logs (`docker logs dc-airflow-apiserver-1 2>&1 | tail -200`) show:

| Airflow log | Meaning |
|-------------|--------|
| `Error authorizing OAuth access token: invalid_request: Missing parameter: code_challenge_method` | Keycloak rejected the **authorize** request (PKCE required). User never reached Keycloak login; Keycloak redirected back with this error. Fix: disable PKCE for airflow client (run create-airflow-keycloak-client.sh). |
| `Error authorizing OAuth access token: mismatching_state: CSRF Warning! State not equal in request and response.` | Callback state does not match session (e.g. different browser, session expired, or E2E test with fake state). User should retry from Airflow login page in same browser. |
| `Error authorizing OAuth access token: Missing "jwks_uri" in metadata` | User got past Keycloak (real `code` in callback). Authlib could not validate id_token because provider metadata had no jwks_uri. Fix: set `server_metadata_url` in `webserver_config.py` to Keycloak’s `.well-known/openid-configuration` (done in repo). Redeploy config and restart apiserver. |

---

## 4. Quick health check commands (on server)

```bash
cd /root/lianel/dc

# All Airflow + Keycloak containers
docker compose -f docker-compose.airflow.yaml ps -a
docker ps -a --format 'table {{.Names}}\t{{.Status}}' | grep -E 'airflow|keycloak'

# Worker was Created?
docker start dc-airflow-worker-1
docker ps -a --filter name=airflow-worker

# Keycloak recent errors
docker logs keycloak 2>&1 | tail -80 | grep -i 'ERROR\|LOGIN_ERROR\|invalid_redirect\|PKCE'

# Airflow OAuth/denied errors
docker logs dc-airflow-apiserver-1 2>&1 | tail -150 | grep -i 'Error authorizing OAuth\|denied\|oauth\|token'
```

---

## 5. Keycloak admin has no Airflow permission (403, can’t change roles)

**Symptom**: Logged in as Keycloak admin but in Airflow you get 403 when triggering a DAG or opening Admin → List Users.

**Cause**: New OAuth users were registered with a non-Admin role (e.g. User or Viewer). Only Airflow **Admin** can access Admin menu and edit user roles.

**Fix**:
1. **New users**: Set `AUTH_USER_REGISTRATION_ROLE = 'Admin'` in `webserver_config.py` (done in repo) and deploy; new Keycloak logins get Admin.
2. **Existing users**: Grant Admin to your Airflow user (run on server):
   ```bash
   docker exec dc-airflow-apiserver-1 airflow users list    # get username (e.g. email or preferred_username)
   docker exec dc-airflow-apiserver-1 airflow users add-role -u <username> -r Admin
   ```
   Or use: `bash scripts/maintenance/airflow-grant-admin-role.sh <username>`.

---

## 6. Fixes applied in repo

- **webserver_config.py**: `server_metadata_url` set to `https://auth.lianel.se/realms/lianel/.well-known/openid-configuration` so Authlib gets `jwks_uri` and can validate id_token (fixes "Missing jwks_uri in metadata" after successful Keycloak login).
- **create-airflow-keycloak-client.sh**: Sets `pkce.code.challenge.method` to `''` for the airflow client. **Run on server** after any Keycloak client reset so Keycloak does not require PKCE (FAB/authlib does not send code_challenge).
- **Worker in Created**: Use `docker start dc-airflow-worker-1` or `docker compose -f docker-compose.airflow.yaml up -d airflow-worker` (see §1).
- **Keycloak admin 403 in Airflow**: Set `AUTH_USER_REGISTRATION_ROLE = 'Admin'`; for existing users run `airflow users add-role -u <username> -r Admin` (see §5).
