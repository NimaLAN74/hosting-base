# Stock Monitoring – Finnhub Webhook and API Auth

**Last updated:** February 2026  
**Owner:** Stock Monitoring / Ops

---

## Overview

The Stock Monitoring backend integrates with [Finnhub.io](https://finnhub.io) for quotes and optional webhooks. Finnhub requires:

1. **Outbound requests:** All requests from our server must include the header `X-Finnhub-Secret: <secret>` when using webhook-related features.
2. **Webhook endpoint:** Must return a **2xx HTTP status** immediately to acknowledge receipt; Finnhub disables the endpoint if it fails to acknowledge over consecutive days. Any processing should happen after acknowledging.

This runbook documents the webhook URL, environment variables, and how to set them on the remote host (via pipeline or SSH) and locally.

---

## Webhook URL

After deployment, the Finnhub webhook URL is:

- **Path (backend):** `POST /internal/webhooks/finnhub`
- **Public URL (behind nginx):** `https://<your-host>/api/v1/stock-monitoring/internal/webhooks/finnhub`

Configure this URL in the Finnhub dashboard when setting up webhooks. The backend returns **200 OK** as soon as the request is received (after optional secret verification), then may process the body in the background.

---

## Environment variables

| Variable | Purpose |
|----------|---------|
| `FINNHUB_API_KEY` | Finnhub API key for quote requests. Backend also reads `STOCK_MONITORING_FINNHUB_API_KEY`. |
| `FINNHUB_WEBHOOK_SECRET` | Secret for (a) `X-Finnhub-Secret` on outbound Finnhub API requests, (b) verification of `X-Finnhub-Secret` on incoming webhook POSTs. Backend also reads `STOCK_MONITORING_FINNHUB_WEBHOOK_SECRET`. |

- If `FINNHUB_WEBHOOK_SECRET` is set, the webhook handler verifies the incoming `X-Finnhub-Secret` header and returns 401 when it does not match.
- If unset, the webhook still returns 200 (acknowledgement) and does not verify the header.
- **Do not commit real values to the repo.** Use GitHub secrets and/or remote `.env` only.

---

## Remote host .env (pipeline or SSH)

### Option A: Pipeline (GitHub secrets)

1. In the repo: **Settings → Secrets and variables → Actions**, add:
   - `FINNHUB_API_KEY` (your Finnhub API key)
   - `FINNHUB_WEBHOOK_SECRET` (your Finnhub webhook secret)
2. Push to `master`/`main` (or trigger the Stock Monitoring CI/deploy workflow). The deploy job passes these to the remote and **deploy-stock-monitoring-backend.sh** writes them into the remote `lianel/dc/.env` (or `hosting-base/lianel/dc/.env`), then restarts the stock-monitoring service.

### Option B: SSH script (keys not in GitHub)

From the repo root, with an SSH key that can log in to the remote host:

```bash
# Set vars (or put them in your local .env and source it)
export REMOTE_HOST=your-server
export REMOTE_USER=root
export FINNHUB_API_KEY=your-api-key
export FINNHUB_WEBHOOK_SECRET=your-webhook-secret

bash lianel/dc/scripts/deployment/add-finnhub-keys-remote-env.sh
```

The script SSHs to the remote, appends/updates `FINNHUB_API_KEY` and `FINNHUB_WEBHOOK_SECRET` in the remote `.env`, and restarts the stock-monitoring service.

---

## Local .env (development / script use)

Update your **local** `.env` so that:

1. **Local runs** of the backend (or UI) can use Finnhub if you want.
2. **add-finnhub-keys-remote-env.sh** can read `FINNHUB_API_KEY` and `FINNHUB_WEBHOOK_SECRET` from `.env` when you run it to push keys to the remote.

Copy the placeholder lines from **`.env.example`** into your local `.env` and fill in values (never commit the real `.env`):

```bash
# In repo root
grep -E 'FINNHUB|STOCK_MONITORING_DATA_PROVIDER' .env.example >> .env
# Then edit .env and set the values.
```

---

## Docker Compose

`docker-compose.stock-monitoring.yaml` passes these through so the container receives them from the host `.env`:

- `FINNHUB_API_KEY: ${FINNHUB_API_KEY:-}`
- `FINNHUB_WEBHOOK_SECRET: ${FINNHUB_WEBHOOK_SECRET:-}`

The remote host `.env` is updated by the pipeline (Option A) or the SSH script (Option B).
