# Swagger UI Assets MIME Type Fix

## 🔴 Issue
Swagger UI static assets (CSS, JS files) were being served as `text/html` instead of their correct MIME types, causing browser errors:
- `swagger-ui.css` blocked due to MIME type mismatch
- `swagger-ui-bundle.js` blocked
- `swagger-initializer.js` invalid JavaScript MIME type
- `index.css` not loaded

## 🔍 Root Cause
Swagger UI HTML references assets with relative paths like `./swagger-ui.css`. When loaded from `https://lianel.se/swagger-ui`, the browser requests:
- `https://lianel.se/swagger-ui.css` (without `/swagger-ui/` prefix)

The nginx config only matched `/swagger-ui/(.*\.(js|css|...))` which requires the `/swagger-ui/` prefix. Requests without the prefix fell through to the frontend route, returning HTML instead of the actual CSS/JS files.

## ✅ Fix Applied
Added a new nginx location block to handle direct asset requests (without `/swagger-ui/` prefix):
- `/swagger-ui.css`
- `/swagger-ui-bundle.js`
- `/swagger-ui-standalone-preset.js`
- `/index.css`
- `/swagger-initializer.js`

These are now correctly routed to the profile service at `/swagger-ui/{filename}`.

## 📋 Configuration

### Before
```nginx
location ~* ^/swagger-ui/(.*\.(js|css|png|ico|json|map))$ {
    # Only matched /swagger-ui/file.ext, not /swagger-ui.css
}
```

### After
```nginx
# Handle direct asset requests (e.g., /swagger-ui.css)
location ~* ^/(swagger-ui\.(css|js|bundle\.js|standalone-preset\.js)|index\.css|swagger-initializer\.js)$ {
    proxy_pass http://lianel-profile-service:3000/swagger-ui/$1;
}

# Handle assets in subdirectory (e.g., /swagger-ui/file.ext)
location ~* ^/swagger-ui/(.*\.(js|css|png|ico|json|map))$ {
    proxy_pass http://lianel-profile-service:3000/swagger-ui/$1;
}
```

## 🧪 Testing

1. Visit: https://lianel.se/swagger-ui
2. Check browser console - should be no MIME type errors
3. Verify assets load:
   - CSS files should load with `text/css` MIME type
   - JS files should load with `application/javascript` MIME type

## 🔧 Verification

```bash
# Test CSS file
curl -k -I https://lianel.se/swagger-ui.css
# Should return: Content-Type: text/css

# Test JS file
curl -k -I https://lianel.se/swagger-ui-bundle.js
# Should return: Content-Type: application/javascript
```

---

**Status**: ✅ Fixed - Swagger UI assets now load with correct MIME types

