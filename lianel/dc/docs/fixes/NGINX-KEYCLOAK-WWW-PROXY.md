# Nginx: Keycloak /realms/ and /resources/ on www.lianel.se

When the login form or theme assets are requested on **www.lianel.se** (e.g. POST to `/realms/lianel/login-actions/authenticate`, GET for `/resources/.../patternfly.min.css` or `/resources/.../authChecker.js`), nginx must proxy those to Keycloak with **Host: auth.lianel.se** and correct URI.

**Cause of 404 / wrong MIME**: Using a **variable** in `proxy_pass` (e.g. `proxy_pass $keycloak_upstream/resources/;`) can change how nginx passes the request URI to the backend; Keycloak then returns 404 or wrong content-type.

**Fix**: Use a literal upstream and literal `proxy_pass` so URI rewriting is reliable.

## Config (`nginx/config/nginx.conf`)

- **Upstream** (in `http` block):

  ```nginx
  upstream keycloak_backend {
      server keycloak:8080;
  }
  ```

- **www.lianel.se** server block — Keycloak locations:
  - `location /auth/` → `proxy_pass http://keycloak_backend/;` + `proxy_set_header Host auth.lianel.se;`
  - `location ^~ /realms/` → `proxy_pass http://keycloak_backend/realms/;` + Host auth.lianel.se
  - `location ^~ /resources/` → `proxy_pass http://keycloak_backend/resources/;` + Host auth.lianel.se

## Deploy

1. Copy updated `lianel/dc/nginx/config/nginx.conf` to the server (e.g. `/root/lianel/dc/nginx/config/nginx.conf`).
2. Reload nginx:  
   `docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload`

**Note**: Local `nginx -t` may fail with “host not found” for `keycloak` until run inside the Docker network where that hostname resolves.
