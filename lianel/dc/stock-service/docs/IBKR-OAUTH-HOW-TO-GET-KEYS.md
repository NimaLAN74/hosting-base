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
   - Use the realm shown in the portal (e.g. `limited_poa` for production, or a test realm).  
   - This is `IBKR_OAUTH_REALM`.

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
  IBKR_OAUTH_REALM=limited_poa
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
