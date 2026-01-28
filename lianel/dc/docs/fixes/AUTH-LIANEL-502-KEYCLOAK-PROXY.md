# auth.lianel.se 502 Bad Gateway (Keycloak proxy) Fix

**Date**: January 2026  
**Issue**: GET https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth returns **502 Bad Gateway**  
**Status**: Nginx config fix applied

---

## Cause

On **auth.lianel.se** the Keycloak `location /` block used a **variable** `proxy_pass` (`set $keycloak_upstream "http://keycloak:8080"; proxy_pass $keycloak_upstream`). Variable `proxy_pass` can lead to 502 or wrong URI handling. The **www.lianel.se** Keycloak proxy already used a **literal** upstream (`keycloak_backend`) for reliability.

---

## Fix Applied

In **`lianel/dc/nginx/config/nginx.conf`**, in the **auth.lianel.se** HTTPS server block:

- Replaced variable `proxy_pass $keycloak_upstream` with **literal** `proxy_pass http://keycloak_backend/;` (same upstream as www).
- Added **timeouts**: `proxy_connect_timeout 10s`, `proxy_read_timeout 90s`, `proxy_send_timeout 90s` so slow Keycloak responses (e.g. ~4s) don’t cause 502.

---

## Deploy on Remote Host

1. Sync the updated **`nginx/config/nginx.conf`** to the server (e.g. copy to `/root/hosting-base/lianel/dc/nginx/config/` or `/root/lianel/dc/nginx/config/`).
2. Reload nginx:  
   `docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload`
3. Retest:  
   https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth?client_id=frontend-client&redirect_uri=https://www.lianel.se/&response_type=code&scope=openid&...

---

## If 502 Persists

- Confirm **Keycloak** is running:  
  `docker ps | grep keycloak`
- Confirm **nginx** and **keycloak** are on the same Docker network (e.g. `lianel-network`) so `keycloak:8080` resolves:  
  `docker network inspect lianel-network` (or the network used by nginx).
- Check nginx error log:  
  `docker logs nginx-proxy 2>&1 | tail -50`
- Check Keycloak logs:  
  `docker logs keycloak 2>&1 | tail -50`
