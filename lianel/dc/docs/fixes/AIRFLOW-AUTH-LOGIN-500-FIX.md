# Airflow /auth/login Internal Server Error (500) Fix

**Date**: January 2026  
**Issue**: https://airflow.lianel.se/auth/login/?next=https://airflow.lianel.se/ returns Internal Server Error  
**Status**: Fix applied and deployed (Jan 2026). Airflow uses dedicated DB `airflow` via `POSTGRES_DB_AIRFLOW`.

---

## Cause

Airflow 3 api-server uses Flask AppBuilder (FAB) for auth. FAB loads OAuth config from a **webserver config file**. The default path is `{AIRFLOW_HOME}/webserver_config.py` (in Docker: `/opt/airflow/webserver_config.py`). Our file is mounted at **`/opt/airflow/config/webserver_config.py`** via the `config` volume. Because the path was never set, FAB did not load our Keycloak OAuth config; the auth stack then failed when serving `/auth/login`, resulting in 500.

---

## Fix Applied

In **`lianel/dc/docker-compose.airflow.yaml`** (under `x-airflow-common` → `environment`):

- Set **`AIRFLOW__FAB__CONFIG_FILE: /opt/airflow/config/webserver_config.py`**

So the api-server (and any other FAB-using component) loads the existing `config/webserver_config.py` (Keycloak OAuth, callback `https://airflow.lianel.se/oauth-authorized/keycloak`).

---

## Dedicated Airflow database (required)

Airflow must use its **own** database, not `lianel_energy`. In `docker-compose.airflow.yaml` the DB is set via **`POSTGRES_DB_AIRFLOW`** (default `airflow`). On the host Postgres, create the DB and grant the Airflow user:

- `CREATE DATABASE airflow OWNER airflow;` (or grant `airflow` user all on `airflow`).
- Ensure `.env` has **`AIRFLOW_OAUTH_CLIENT_SECRET`** and does **not** override `POSTGRES_DB_AIRFLOW` with `lianel_energy` for Airflow (or set `POSTGRES_DB_AIRFLOW=airflow`).

## Deploy Steps (on remote host)

1. Pull the latest code (or sync `docker-compose.airflow.yaml` and `config/webserver_config.py`).
2. Ensure **`.env`** on the host has **`AIRFLOW_OAUTH_CLIENT_SECRET`** set to the Airflow Keycloak client secret.
3. Restart Airflow services so they pick up the new env:
   - `docker compose -f docker-compose.airflow.yaml down`
   - `docker compose -f docker-compose.airflow.yaml up -d`
4. Check api-server logs for any config/import errors:  
   `docker logs <airflow-apiserver-container> 2>&1 | tail -100`
5. Open https://airflow.lianel.se/auth/login/ — you should get the login page (or redirect to Keycloak).

---

## If 500 Persists

- Confirm **`AIRFLOW_OAUTH_CLIENT_SECRET`** is set in the environment of the api-server container:  
  `docker exec <airflow-apiserver-container> env | grep AIRFLOW_OAUTH`
- Confirm **`config/webserver_config.py`** is present in the container:  
  `docker exec <airflow-apiserver-container> cat /opt/airflow/config/webserver_config.py`
- Check api-server logs for the exact traceback (e.g. missing env, import error, or Keycloak unreachable).

---

## References

- FAB config: [Configuring Flask Application for Airflow Webserver](https://airflow.apache.org/docs/apache-airflow-providers-fab/stable/auth-manager/configuring-flask-app.html)
- FAB config option: `[fab] config_file` / `AIRFLOW__FAB__CONFIG_FILE` (default: `{AIRFLOW_HOME}/webserver_config.py`)
- Local OAuth setup: `lianel/dc/config/webserver_config.py`, `lianel/dc/docs/fixes/KEYCLOAK-PATTERN-GRAFANA-AIRFLOW-FRONTEND.md`
