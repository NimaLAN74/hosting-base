# Root causes from container logs (March 2026)

Findings from `docker logs` on the server – use these to fix Verify CSS red, Gateway login failure, and OAuth2-proxy restart loop.

---

## 1. Verify CSS red / Frontend login 400

**Keycloak logs:**
```text
type="LOGIN_ERROR" clientId="frontend-client" error="invalid_request"
reason="Missing parameter: code_challenge_method"
```

**Cause:** Keycloak client `frontend-client` has **PKCE enforced** (requires `code_challenge` and `code_challenge_method`), but the auth request reaching Keycloak does not include them, so Keycloak returns 400. Login never reaches the form; theme CSS verification may hit an error/redirect page.

**Fix:** Disable PKCE requirement for `frontend-client` in Keycloak.

- **On server:** Run  
  `bash lianel/dc/scripts/keycloak-setup/update-frontend-client-no-pkce-required.sh`  
  (needs `KEYCLOAK_ADMIN_PASSWORD` in `.env`).
- **Sync Nginx workflow** runs this script in step 8 (apply-remote) when the script exists on the server.

---

## 2. IBKR Gateway login failure (IBeam)

**ibkr-gateway (IBeam) logs:**
```text
Loading auth webpage at https://localhost:5000/sso/Login?forwardTo=22&RL=1&ip2loc=on
Gateway auth webpage loaded
Login attempt number 1
Submitting the form
TimeoutException: Timeout reached when waiting for authentication.
Consider increasing IBEAM_PAGE_LOAD_TIMEOUT.
```

**Cause:** IBeam submits the Gateway login form but then times out waiting for the post-login state (success/redirect). So either: (a) credentials are wrong or 2FA/captcha is required, (b) the page after login does not match what IBeam expects, (c) `IBEAM_PAGE_LOAD_TIMEOUT` is too low.

**Fixes:**
- Ensure `IBEAM_ACCOUNT` / `IBEAM_PASSWORD` (or `IBKR_*` mapped to them) in `.env` are correct and that the account does not require 2FA for this login.
- Increase `IBEAM_PAGE_LOAD_TIMEOUT` (e.g. 150) and `IBEAM_OAUTH_TIMEOUT` (e.g. 90) in server `.env`, then **recreate the container**:  
  `docker compose -f docker-compose.infra.yaml -f docker-compose.ibkr-ibeam.yaml up -d --force-recreate ibkr-gateway`  
  (Compose defaults are 150/90; if server `.env` sets lower values, they override.)
- If login still fails after "Submitting the form", the post-login page is not what IBeam expects (often 2FA or wrong credentials). Use the **cookie workaround**: log in at https://www.lianel.se/ibkr-gateway/, copy the `api` cookie, set `IBKR_GATEWAY_SESSION_COOKIE` on the server and restart stock-service.
- If the Gateway shows an error after submit (e.g. “Invalid credentials”), fix credentials; IBeam cannot complete 2FA or captcha in headless mode.

---

## 3. OAuth2-proxy restart loop

**oauth2-proxy logs:**
```text
invalid configuration:
  missing setting: cookie-secret or cookie-secret-file
  missing setting: client-secret or client-secret-file
```

**Cause:** The container is started without the required config. OAuth2-proxy expects `OAUTH2_PROXY_COOKIE_SECRET` and `OAUTH2_PROXY_CLIENT_SECRET` (or equivalent) to be set; on the server they are missing or not passed into the container.

**Fix:** On the server, ensure `.env` (or the env used by `docker-compose.oauth2-proxy.yaml`) contains:

- `OAUTH2_PROXY_COOKIE_SECRET` (e.g. 32-byte hex or base64)
- `OAUTH2_PROXY_CLIENT_SECRET` (Keycloak client secret for the client used by oauth2-proxy)

Then recreate the container:  
`docker compose -f docker-compose.infra.yaml -f docker-compose.oauth2-proxy.yaml up -d oauth2-proxy`

---

## 4. Summary

| Issue              | Container   | Root cause                          | Fix |
|--------------------|------------|--------------------------------------|-----|
| Verify CSS / login | Keycloak   | frontend-client PKCE required, request without PKCE | Run `update-frontend-client-no-pkce-required.sh` (or Sync Nginx step 8) |
| Gateway login      | ibkr-gateway | IBeam timeout after “Submitting the form”   | Correct credentials; increase `IBEAM_PAGE_LOAD_TIMEOUT`; no 2FA/captcha |
| OAuth2-proxy loop  | oauth2-proxy | missing cookie-secret / client-secret       | Set in server `.env` and recreate container |
