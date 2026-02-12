# Energy Project – Paused

**Date**: February 2026  
**Status**: ⏸️ **PAUSED** – Resources freed for new SaaS project.

---

## Summary

The energy/geo-enrichment project is **paused**. The energy service is no longer deployed or started by default. CI/CD no longer builds or deploys it on push. This frees server and pipeline resources for the new SaaS (Comp-AI and related) work.

---

## What was changed

| Item | Change |
|------|--------|
| **Deploy Energy Service workflow** | No longer runs on push to `master`/`main`. Trigger only: **workflow_dispatch** (manual). File: `.github/workflows/deploy-energy-service.yml`. |
| **docker-compose.backend.yaml** | **energy-service** is commented out. Compose no longer starts the energy container. |
| **Production server** | Stop the energy container if it is still running: `docker stop lianel-energy-service` (optional). |
| **Nginx** | Config still has `/api/energy/` routes. Requests to `/api/energy/` will return **502** while the service is stopped. To resume, start the service again. |
| **Airflow DAGs** | Energy-related DAGs (e.g. ENTSOE ingestion, Eurostat, geo-enrichment) remain in the repo. To free scheduler resources, **pause** them in the Airflow UI (DAGs → toggle pause) if desired. |

---

## What was not removed

- **Code**: `lianel/dc/energy-service/` is unchanged and remains in the repo.
- **Scripts**: Energy-related scripts (e.g. under `scripts/`, `scripts/deployment/deploy-energy-service.sh`) are kept.
- **DAGs**: `lianel/dc/dags/` (e.g. `entsoe_ingestion_dag.py`, eurostat DAGs) are kept; pause in Airflow if you want to stop them.
- **Docs**: Energy-related documentation is kept for when the project is resumed.
- **Database**: No data or schemas were dropped. The `lianel_energy` DB and tables remain.

---

## How to resume the energy project

1. **Compose**: In `lianel/dc/docker-compose.backend.yaml`, uncomment the `energy-service` service.
2. **Workflow**: In `.github/workflows/deploy-energy-service.yml`, add back the `push` trigger (paths for `lianel/dc/energy-service/**`, etc.) if you want deploy on push.
3. **Server**: Start the service, e.g.  
   `docker compose -f docker-compose.backend.yaml up -d energy-service`  
   (from the directory that has the compose file and `.env`).
4. **Airflow**: Unpause any energy-related DAGs if you had paused them.

---

## References

- Deployment status (energy): `lianel/dc/docs/status/DEPLOYMENT-STATUS.md`
- Energy service API: `lianel/dc/energy-service/README.md`
- Workflows overview: `.github/workflows/README.md`
