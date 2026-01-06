# Swagger UI Assets Cache & Login Fix

## 🔴 Issues

1. **Swagger UI Assets Still Showing HTML**
   - `swagger-initializer.js` and `index.css` still returning `text/html`
   - Browser cache may be serving old responses

2. **Login Still Failing**
   - Frontend login not working despite correct configuration

## ✅ Fixes Applied

### 1. Nginx Configuration
- Updated location block to use `rewrite` rule instead of variable substitution
- This ensures correct routing for all Swagger UI assets
- Pattern: `rewrite ^/(.*)$ /swagger-ui/$1 break;`

### 2. Frontend Client Redirect URIs
- Updated `frontend-client` with all required redirect URIs:
  - `https://lianel.se`
  - `https://lianel.se/*`
  - `https://www.lianel.se`
  - `https://www.lianel.se/*`
  - `http://localhost:3000` (for local dev)
- Set `webOrigins: ["*"]` for CORS
- Enabled `frontchannelLogout`

## 🔧 Browser Cache Issue

If assets still show as HTML in browser:
1. **Hard refresh**: `Ctrl+Shift+R` (Windows/Linux) or `Cmd+Shift+R` (Mac)
2. **Clear cache**: Browser settings → Clear browsing data → Cached images and files
3. **Incognito mode**: Test in private/incognito window
4. **Disable cache**: DevTools → Network tab → Disable cache checkbox

## 🧪 Testing

### Swagger UI Assets
```bash
# Test with cache-busting query parameter
curl -k -I 'https://lianel.se/swagger-initializer.js?v=123'
# Should return: Content-Type: text/javascript

curl -k -I 'https://lianel.se/index.css?v=123'
# Should return: Content-Type: text/css
```

### Frontend Login
1. Visit: https://lianel.se
2. Click "Login"
3. Should redirect to: https://auth.lianel.se
4. Login with:
   - Username: `admin`
   - Password: `Admin123!Secure`
5. Should redirect back to https://lianel.se authenticated

## 📋 Configuration

### Nginx Location Block
```nginx
location ~* ^/(swagger-ui\.css|swagger-ui\.js|swagger-ui-bundle\.js|swagger-ui-standalone-preset\.js|index\.css|swagger-initializer\.js)$ {
    rewrite ^/(.*)$ /swagger-ui/$1 break;
    proxy_pass http://lianel-profile-service:3000;
    # ... headers ...
}
```

### Frontend Client
- **Client ID**: `frontend-client`
- **Public Client**: `true`
- **Standard Flow**: `true`
- **PKCE**: `S256`
- **Web Origins**: `*`

---

**Status**: ✅ Fixed - Nginx rewrite rule applied, frontend client updated

**Note**: If browser still shows HTML, clear cache or use incognito mode

