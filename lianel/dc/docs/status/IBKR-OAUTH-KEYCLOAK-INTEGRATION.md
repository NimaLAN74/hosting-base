# IBKR OAuth and Keycloak SSO Integration

## How it fits together

- **Keycloak** remains the **only** identity provider for the app. Users log in with Keycloak (SSO); the frontend sends the Keycloak JWT to the stock-service backend; the backend validates it via Keycloak userinfo (same as profile-service).
- **IBKR Web API** is used **server-side only**. The backend needs a valid **Live Session Token (LST)** to call `https://api.ibkr.com/v1/api` (quotes, history, etc.). The LST is obtained using **first-party OAuth 1.0a** credentials stored in the environment (or in `.env`).

So:

- **Keycloak SSO** = who is allowed to use the app and the stock page (same token for all pages).
- **IBKR OAuth (env)** = one shared IBKR account used by the backend to talk to IBKR on behalf of all authenticated app users.

There is no “login with IBKR” in Keycloak. IBKR is not configured as a Keycloak identity provider. The integration is: **Keycloak for people, IBKR env credentials for market data**.

## What is implemented

1. **`.env.example`**  
   Placeholders for:
   - Optional `IBKR_USERNAME` / `IBKR_PASSWORD` (e.g. for portal or future use; not used for api.ibkr.com OAuth).
   - First-party OAuth: `IBKR_OAUTH_CONSUMER_KEY`, `IBKR_OAUTH_ACCESS_TOKEN`, `IBKR_OAUTH_ACCESS_TOKEN_SECRET`, `IBKR_OAUTH_REALM`, and paths to PEM files (`IBKR_OAUTH_DH_PARAM_PATH`, `IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH`, `IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH`), plus `IBKR_API_BASE_URL`.

2. **Backend config** (`config.rs`)  
   - New optional fields for all of the above.  
   - `ibkr_oauth_configured()`: true only when consumer key, access token, access token secret, and all three PEM paths are set.

3. **IBKR OAuth module** (`ibkr.rs`)  
   - **First-party Live Session Token (LST)**:
     - Load DH params and RSA keys from PEM paths.
     - Decrypt `access_token_secret` with the private encryption key → prepend.
     - Build Diffie–Hellman challenge, sign the request with the private signature key (RSA-SHA256), POST to `.../oauth/live_session_token`.
     - From the response compute LST (HMAC-SHA1 with K and prepend), validate with `live_session_token_signature`, cache LST until expiry (~24h).
   - **Signing later API requests**: `sign_request(method, url, body)` uses the cached LST to build an `Authorization` header (HMAC-SHA256 with LST as key) for calls to IBKR.

4. **Status API**  
   - `GET /api/v1/status` (and thus `/api/v1/stock-service/status` when proxied) includes `ibkr_oauth_configured: true/false` so the frontend can show “IBKR connected” or “Connect IBKR” (or hide IBKR features when false).

5. **AppState**  
   - Holds optional `config: Arc<AppConfig>` so status (and future handlers) can read `ibkr_oauth_configured` without parsing env again.

## Getting first-party OAuth credentials

1. Go to [IBKR Self Service OAuth](https://ndcdyn.interactivebrokers.com/sso/Login?action=OAUTH&RL=1&ip2loc=US) and sign in.
2. Create/obtain:
   - **Consumer key** (9 characters).
   - **Access token** and **Access token secret** (from the portal after completing the flow).
   - Three PEM files from the portal: **DH param**, **private encryption key**, **private signature key** (often named e.g. `dhparam.pem`, `private_encryption.pem`, `private_signature.pem`).
3. Put the paths to these PEMs in env (e.g. in `.env`):
   - `IBKR_OAUTH_DH_PARAM_PATH`
   - `IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH`
   - `IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH`
4. Set `IBKR_OAUTH_CONSUMER_KEY`, `IBKR_OAUTH_ACCESS_TOKEN`, `IBKR_OAUTH_ACCESS_TOKEN_SECRET`, and optionally `IBKR_OAUTH_REALM` (e.g. `limited_poa` for production, `test_realm` for paper).

After that, the backend can obtain and cache the LST and sign requests to IBKR. No Keycloak change is required.

## Optional: IBKR username/password in `.env`

You can add `IBKR_USERNAME` and `IBKR_PASSWORD` to `.env` for:

- Reference (e.g. which account the OAuth credentials belong to).
- Future use (e.g. a different auth path or portal automation).
- They are **not** sent to `api.ibkr.com`; only the OAuth consumer key, access token, secret, and PEMs are used for the LST.

## Summary

- **Keycloak SSO**: unchanged; same token for main app and stock page; backend validates with Keycloak userinfo.
- **IBKR**: backend uses env-based first-party OAuth 1.0a to get a Live Session Token and call IBKR APIs; no Keycloak–IBKR IdP link.
- **Integration**: “Integration with Keycloak SSO” means: once the user is logged in with Keycloak, they can use the stock UI; the backend uses the single IBKR credential set from env for all IBKR calls.
