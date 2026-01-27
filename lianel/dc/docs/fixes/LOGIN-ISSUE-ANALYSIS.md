# Login Issue – Deep Analysis

## 1. Intended flow (success path)

1. User opens **https://www.lianel.se/**
2. User clicks "Log in" / "Sign In"
3. Frontend calls `login()` → builds `redirectUri = https://www.lianel.se/` (or current path)
4. Frontend redirects browser to **https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth?client_id=frontend-client&redirect_uri=...&...**
5. Nginx (www.lianel.se, `location /auth/`) proxies to **http://keycloak:8080/** with **Host: auth.lianel.se**, URI path **realms/lianel/protocol/openid-connect/auth?...**
6. Keycloak (KC_HOSTNAME=auth.lianel.se) shows login page
7. User submits credentials → Keycloak redirects to **redirect_uri?code=...&state=...** (i.e. https://www.lianel.se/?code=...)
8. Frontend loads, keycloak.init() sees `?code=...`, exchanges code at **https://www.lianel.se/auth/realms/.../token** (same-origin)
9. User ends on https://www.lianel.se/ (or original path), logged in

## 2. Config dependencies (all must hold)

| Layer | Config | Purpose |
|-------|--------|---------|
| Frontend build | REACT_APP_KEYCLOAK_URL=https://www.lianel.se/auth | keycloak-js hits same-origin /auth |
| Frontend runtime | login() uses createLoginUrl + await, redirectUri = origin + path | No .includes on Promise; correct callback |
| Nginx www | location /auth/ → proxy_pass http://keycloak:8080/, Host: auth.lianel.se | Keycloak sees correct host |
| Nginx ↔ Keycloak | Same Docker network (lianel-network), keycloak:8080 resolvable | No 502 |
| Keycloak | KC_HOSTNAME=auth.lianel.se, KC_PROXY_HEADERS=xforwarded | Accepts proxy requests |
| Keycloak client | frontend-client redirectUris includes https://www.lianel.se/*, baseUrl/rootUrl=https://www.lianel.se | Post-login redirect to app |
| Realm | frontendUrl=https://www.lianel.se | Keycloak “back to app” uses app URL |

## 3. Failure points and symptoms

| Failure | Symptom | Root cause |
|---------|---------|------------|
| A. Nginx cannot reach keycloak:8080 | **502 Bad Gateway** on GET https://www.lianel.se/auth/... | keycloak not running, or nginx/keycloak not on same Docker network, or wrong upstream |
| B. Frontend uses old bundle | "e.includes is not a function", "Keycloak login URL generated: Promise" | Deployed JS still uses createLoginUrl result as string |
| C. Keycloak URL wrong at build time | keycloak-js talks to auth.lianel.se or wrong path | REACT_APP_KEYCLOAK_URL not https://www.lianel.se/auth in build |
| D. redirect_uri not allowed | Keycloak "Invalid redirect uri" | frontend-client redirectUris missing https://www.lianel.se/ or variant |
| E. Post-login redirect wrong | User lands on auth.lianel.se/admin or /login | baseUrl/rootUrl or realm frontendUrl not https://www.lianel.se |

## 4. Request path for login (www.lianel.se)

- Browser → **https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth?...**
- Nginx server_name www.lianel.se, location **/auth/** matches.
- proxy_pass **http://keycloak:8080/**; request URI after /auth/ is **realms/lianel/protocol/openid-connect/auth?...**
- Upstream request: **GET http://keycloak:8080/realms/lianel/protocol/openid-connect/auth?...**  
  Headers: **Host: auth.lianel.se**, X-Forwarded-* set.
- For this to return 200/302, **(i)** name "keycloak" must resolve in nginx container’s network, **(ii)** Keycloak must be listening on 8080, **(iii)** Keycloak must accept Host auth.lianel.se (KC_HOSTNAME=auth.lianel.se).

## 5. 502 root cause (evidence-based)

502 = “upstream sent invalid response or connection failed”. So one of:

1. **keycloak:8080 not resolvable** – nginx and keycloak on different Docker networks, or keycloak not running.
2. **Connection refused** – nothing listening on keycloak:8080.
3. **Keycloak returns 5xx** – less common; would need server logs.

**Conclusion:** 502 on /auth/ means the machine running nginx cannot complete a TCP request to keycloak:8080 (resolve + connect). Fix is on the server: same network, keycloak container up, and nginx config pointing at that keycloak.

## 6. Auth via auth.lianel.se (alternative path)

- **https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth?...** is handled by server block **auth.lianel.se**, location **/** → proxy_pass **http://keycloak:8080** (no trailing slash), Host from **$host** (auth.lianel.se).
- Same upstream **keycloak:8080**. So if www.lianel.se/auth/ gives 502, auth.lianel.se/realms/... will also give 502 unless a different nginx/stack handles auth.lianel.se. From the same nginx config, both use keycloak:8080 → **502 cannot be fixed by switching to auth.lianel.se** without fixing keycloak reachability.

## 7. Order of operations for fixes

1. **Ensure 502 is resolved** (server): keycloak on lianel-network, container up, nginx upstream keycloak:8080.
2. **Ensure deployed frontend has login fix** (build/deploy): login() uses createLoginUrl + await, no .includes on URL; build with REACT_APP_KEYCLOAK_URL=https://www.lianel.se/auth.
3. **Ensure redirect after login** (Keycloak): run fix-keycloak-https.sh + update-keycloak-frontend-client.sh so frontendUrl and baseUrl/rootUrl = https://www.lianel.se.

## 8. Fix 502 on the server

Run on the **server** (where Docker/Keycloak/nginx run):

```bash
bash /root/lianel/dc/scripts/fix-502-keycloak-on-server.sh
# or, if dc lives under hosting-base:
bash /root/hosting-base/lianel/dc/scripts/fix-502-keycloak-on-server.sh
```

That script starts keycloak and nginx from `docker-compose.infra.yaml` so both use `lianel-network` and nginx can reach `keycloak:8080`. If nginx is currently run by another compose, you may need to stop that stack and use infra for both (or ensure that compose puts nginx on `lianel-network`).

## 9. Tests to run after each change

- **T1** – Open https://www.lianel.se/ → click Login → must get Keycloak login page (no 502, no JS error).
- **T2** – Submit credentials → must land on https://www.lianel.se/ (or same path) with user logged in.
- **T3** – No "e.includes is not a function" or "Keycloak login URL generated: Promise" in console.
