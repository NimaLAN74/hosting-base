# OAuth2 Proxy: CORS "Access-Control-Allow-Origin missing" for auth URL

## Problem

When a **fetch()** or XHR from www.lianel.se hits a protected route (e.g. `/api/...`, `/monitoring/...`):

1. Nginx does `auth_request /oauth2/auth` → OAuth2 Proxy returns 401.
2. Nginx does `error_page 401 = /oauth2/sign_in` → client receives **302** to `https://auth.lianel.se/realms/.../auth?client_id=oauth2-proxy&...`.
3. The **fetch** API **follows** the redirect and requests auth.lianel.se (cross-origin).
4. Keycloak returns **200** (login page) but **without** `Access-Control-Allow-Origin` → browser blocks: "CORS header 'Access-Control-Allow-Origin' missing".

So the failure is: **fetch gets a redirect instead of 401**, then follows it cross-origin and hits CORS.

## Fix (no change to login flow)

- **For API/fetch requests** (`Accept: application/json`): return **401** so the client gets 401 and can do `window.location = '/oauth2/sign_in'` (full page redirect). No fetch follows a redirect → no CORS.
- **For browser requests** (e.g. `Accept: text/html`): keep **302** to `/oauth2/sign_in` so the user is redirected to sign-in as before.

Implementation:

1. **Map** in `http`: `$auth_401_api = 1` when `Accept` contains `application/json`.
2. **Named location** `@error_401`: if `$auth_401_api = 1` then `return 401`; else `return 302 $scheme://$host/oauth2/sign_in?rd=$request_uri`.
3. Replace `error_page 401 = /oauth2/sign_in` with `error_page 401 = @error_401` in all protected locations (www and monitoring server blocks).
4. Define `location @error_401` in both www.lianel.se and monitoring.lianel.se server blocks.

**Login flow unchanged**: browser navigation to a protected page still gets 302 → sign_in → Keycloak → callback.
