# 404 Error Fix - Energy Service Info Endpoint

## Issue
User getting 404 error when calling `/api/energy/info`:
```
GET https://www.lianel.se/api/energy/info
[HTTP/1.1 404 Not Found 44ms]
```

## Root Cause

**Route Mismatch**:
- Frontend calls: `/api/energy/info`
- Nginx rewrites: `/api/energy/info` → `/info` (strips `/api/energy` prefix)
- Service route was: `/api/info` ❌
- Service receives: `/info` but expects `/api/info` → **404 Not Found**

## Fix Applied

Changed service route in `energy-service/src/main.rs`:
- **Before**: `.route("/api/info", get(handlers::metadata::get_service_info))`
- **After**: `.route("/info", get(handlers::metadata::get_service_info))`

Also updated OpenAPI annotation in `handlers/metadata.rs`:
- **Before**: `path = "/api/energy/api/info"`
- **After**: `path = "/api/energy/info"`

## Route Flow (After Fix)

1. Frontend calls: `/api/energy/info`
2. Nginx rewrites: `/api/energy/info` → `/info`
3. Service route: `/info` ✅
4. Service receives: `/info` → **200 OK**

## Status

- ✅ Code fixed
- ✅ Committed and pushed
- ⏳ Waiting for energy service deployment pipeline
- ⏳ Will test after deployment

## Test After Deployment

```bash
# Should return service info
curl 'https://www.lianel.se/api/energy/info' -H "Authorization: Bearer <token>"
```

Expected: JSON response with service information (database stats, etc.)
