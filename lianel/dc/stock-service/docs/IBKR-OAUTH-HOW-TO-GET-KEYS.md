# How to Get IBKR OAuth Keys and Secrets

You need **API** credentials (consumer key, access token, secret, and PEM files), not just your normal IBKR login. Those are created in IBKR’s **OAuth Self Service** portal. Your **IBKR username and password** are only used to **log in to that portal**.

---

## Step 1: Log in to the IBKR OAuth Self Service Portal

1. Open: **[IBKR Self Service OAuth](https://ndcdyn.interactivebrokers.com/sso/Login?action=OAUTH&RL=1&ip2loc=US)**  
   (Or: Account Management → Settings → API → OAuth, or search “IBKR API OAuth” in IBKR’s site.)

2. Sign in with your **normal IBKR username and password** (the same you use for the client or client portal).

3. You are now in the **OAuth / API configuration** area where you create an “application” and get keys.

---

## Step 2: Create an Application and Get Keys

In the Self Service portal you will:

1. **Create or select an application** (e.g. “My Stock Service”).

2. **Consumer key**  
   - Create a **consumer key** (often a 9‑character string).  
   - Save it → this is `IBKR_OAUTH_CONSUMER_KEY`.

3. **Generate and download PEM files**  
   The portal lets you generate:
   - **DH parameter** → download as something like `dhparam.pem`.
   - **Private encryption key** → e.g. `private_encryption.pem`.
   - **Private signature key** → e.g. `private_signature.pem`.  

   Save these three files; the stock service expects them on the server (see below).

4. **Access token and access token secret**  
   - In the portal, complete the flow to **generate** an **access token** and **access token secret**.  
   - The secret is often shown in **base64**; use it as-is for `IBKR_OAUTH_ACCESS_TOKEN_SECRET`.  
   - Copy the access token → `IBKR_OAUTH_ACCESS_TOKEN`.

5. **Realm**  
   - **What to use:** The app **defaults to `test_realm`** (for TESTCONS or testing). For production (live) with your own consumer key, set **`IBKR_OAUTH_REALM=limited_poa`** in your `.env`.  
   - Per IBKR docs: `test_realm` for TESTCONS/paper; `limited_poa` for your own consumer key.

---

## Step 3: Put Everything on the Server

- **PEM files**  
  Copy the three PEM files to the server directory you prepared, e.g.:
  - `/root/lianel/dc/stock-service-ibkr/dhparam.pem`
  - `/root/lianel/dc/stock-service-ibkr/private_encryption.pem`
  - `/root/lianel/dc/stock-service-ibkr/private_signature.pem`

- **Environment variables**  
  In the same directory’s `.env` (e.g. `/root/lianel/dc/.env`), set:

  ```bash
  IBKR_OAUTH_CONSUMER_KEY=Your9CharConsumerKey
  IBKR_OAUTH_ACCESS_TOKEN=the_access_token_from_portal
  IBKR_OAUTH_ACCESS_TOKEN_SECRET=the_base64_secret_from_portal
  # Omit IBKR_OAUTH_REALM for testing (defaults to test_realm). For production: IBKR_OAUTH_REALM=limited_poa
  ```

  The PEM **paths** are already set by the deploy (container paths like `/app/ibkr-conf/dhparam.pem`); you don’t need to change them unless you use different paths.

- **Restart the stock service**  
  So it picks up the new env and PEMs:

  ```bash
  cd /root/lianel/dc
  docker compose -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d --force-recreate --no-deps stock-service
  ```

---

## Summary

| What you have              | Where it’s used |
|----------------------------|------------------|
| IBKR **username / password** | Only to **log in** to the OAuth Self Service portal |
| **Consumer key, access token, secret, PEMs** | Created **inside that portal**; you then put them in server `.env` and the `stock-service-ibkr/` folder |

If the portal URL or menu changes, check IBKR’s current documentation (e.g. [IBKR API OAuth](https://www.interactivebrokers.com/campus/ibkr-api-page/oauth-1-0a-extended/) or “Web API” / “OAuth” in their site).

---

## Troubleshooting: “invalid consumer” (401)

If the API returns `401 Unauthorized` with an error like `"invalid consumer"` (error ids 30883, 31470, etc. all mean the same), check:

1. **Consumer key**
   - Must match **exactly** the value from the IBKR OAuth Self Service portal (same application).
   - No leading/trailing spaces or quotes in `.env`.
   - Case-sensitive.
   - New keys can take until **after midnight** (portal region time) to become active; if you just created the app, try again later.

2. **Realm**
   - `IBKR_OAUTH_REALM`: use `test_realm` for TESTCONS/testing (app default), or `limited_poa` for production with your own consumer key.

3. **Application status**
   - In the portal, confirm the application is **active** and approved for API access (no pending steps).

4. **Environment**
   - Production: base URL should be `https://api.ibkr.com/v1/api` (default). Paper/demo may use a different base URL; set `IBKR_API_BASE_URL` if required.

5. **Access token and secret**
   - Must be from the **same** application as the consumer key. Regenerate token/secret in the portal if you recreated the app or consumer key.

---

## Regenerate access token and secret

If you get **401 Unauthorized** or **invalid consumer** and the consumer key is correct, regenerate the access token and secret:

1. Log in to **[IBKR Self Service OAuth](https://ndcdyn.interactivebrokers.com/sso/Login?action=OAUTH&RL=1&ip2loc=US)**.
2. On the configuration page, scroll to **Access Request Token**.
3. Click **Generate Token**. New **Access Token** and **Access Token Secret** will appear.
4. **Copy both values immediately** (they disappear when you leave or refresh the page).
5. Update the server and restart the stock service (from repo root):

   ```bash
   IBKR_OAUTH_ACCESS_TOKEN='<paste_access_token>' \
   IBKR_OAUTH_ACCESS_TOKEN_SECRET='<paste_secret>' \
   REMOTE_HOST=72.60.80.84 \
   bash lianel/dc/scripts/deployment/update-ibkr-tokens-remote-env.sh
   ```

   Or set `REMOTE_HOST` (and optionally `IBKR_OAUTH_ACCESS_TOKEN`, `IBKR_OAUTH_ACCESS_TOKEN_SECRET`) in `.env`, then:

   ```bash
   source .env
   bash lianel/dc/scripts/deployment/update-ibkr-tokens-remote-env.sh
   ```

6. Open the Stock Service page and use **Verify IBKR** / watchlist; prices should load.
