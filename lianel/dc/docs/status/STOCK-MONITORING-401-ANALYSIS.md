# Stock monitoring 401 and Keycloak CSS – root cause and fix

## Why profile works and stock did not

- **Profile service** (`/api/profile`): Validates the Bearer token by calling **Keycloak’s userinfo endpoint** (`KEYCLOAK_URL/realms/{realm}/protocol/openid-connect/userinfo`) with the token. If Keycloak returns 200, the token is accepted. No JWT issuer check – so tokens from `www.lianel.se/auth` or `auth.lianel.se` both work.
- **Stock backend** (before fix): Validated the token **locally with JWKS** and required the JWT `iss` claim to match a configured issuer. Tokens issued by `https://www.lianel.se/auth/realms/lianel` did not match `https://auth.lianel.se/realms/lianel`, so the backend returned 401 even when the frontend sent the token.

## Fix: same pattern as profile (userinfo)

The stock-monitoring backend now validates the token **the same way as profile-service**:

1. **Primary:** Call Keycloak userinfo with the Bearer token (same URL pattern as profile). If Keycloak returns 200, use the userinfo response for identity. This accepts tokens from any Keycloak frontend URL (www.lianel.se/auth, auth.lianel.se) because the same Keycloak server validates them.
2. **Fallback:** If the userinfo request fails (e.g. network), fall back to existing JWKS + multi-issuer validation.

So the stock page uses the **exact same auth pattern** as the rest of the FE (Profile, Comp-AI, etc.): same `authenticatedFetch` and same backend validation method (userinfo).

- **Code:** `lianel/dc/stock-monitoring/backend/src/auth.rs` – `validate_bearer_identity_via_userinfo()`; `app.rs` – `require_auth` tries userinfo first, then JWKS.
- **Config:** `KEYCLOAK_URL` (and optionally `KEYCLOAK_REALM`) must be set so the backend can reach Keycloak (e.g. `https://auth.lianel.se` or `http://keycloak:8080` inside Docker). No `KEYCLOAK_ISSUER_ALT` needed for SSO when using userinfo.

## Keycloak CSS MIME type ("text/html" instead of "text/css")

- **Symptom:** Browser reports that `/auth/resources/.../patternfly.min.css` (and other theme assets) have MIME type `text/html`, so styles don’t load.
- **Cause:** Either the request is not handled by the intended nginx location, or the **deployed** nginx config is not the one from the repo (old config, different file, or not reloaded).
- **Repo config:** In `lianel/dc/nginx/config/nginx.conf`, the **www.lianel.se** server block has:
  - `location ^~ /auth/resources/` – proxies to Keycloak with `Host: auth.lianel.se`, rewrites path to `/resources/...`, and sets `Content-Type` from `$resources_content_type` (map by extension: `.css` → `text/css`, `.js` → `application/javascript`).
- **What to verify on the server:**
  1. The file actually used by nginx is the one from the repo: e.g. the container mounts `/root/lianel/dc/nginx/config/nginx.conf` (see `docker-compose.infra.yaml`). Ensure that path is updated (e.g. by the deploy workflow) and nginx is reloaded (`docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload`).
  2. From the server: `curl -sI "https://www.lianel.se/auth/resources/lbvvu/common/keycloak/vendor/patternfly-v5/patternfly.min.css"` and check that the response has `Content-Type: text/css`. If it’s `text/html`, the active nginx config is wrong or a different server/location is handling the request.

## Deploy checklist

- Deploy **frontend** (so `getToken()` and token persistence are current).
- Rebuild and deploy **stock-monitoring backend** (userinfo validation).
- Sync **nginx** config to the path the nginx container uses and reload nginx (so `/auth/resources/` and Content-Type fix are active).
