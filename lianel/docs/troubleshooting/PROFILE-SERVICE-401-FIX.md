# Profile Service 401 Unauthorized Fix

## Issue
Profile Service API (`/api/profile`) was returning `401 Unauthorized` when frontend sent tokens from `frontend-client`.

## Root Cause
Tokens from `frontend-client` (a public client) cannot be introspected by `backend-api` (a confidential client). This is a Keycloak limitation - tokens from public clients are not introspectable by other clients.

## Solution
Modified the profile service token validation to:
1. **First try userinfo endpoint** - Works with tokens from any client (public or confidential)
2. **Fall back to introspection** - For tokens from confidential clients

## Code Changes
Updated `lianel/dc/profile-service/src/main.rs`:
- Changed `validate_token()` method to use userinfo endpoint first
- Falls back to introspection if userinfo fails
- Constructs `KeycloakTokenClaims` from userinfo response

## Testing
After the fix is deployed:
1. Get a token from `frontend-client`:
   ```bash
   curl -X POST "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "username=admin" \
     -d "password=Admin123!Secure" \
     -d "grant_type=password" \
     -d "client_id=frontend-client"
   ```

2. Test profile API with token:
   ```bash
   curl -H "Authorization: Bearer <token>" \
     "https://lianel.se/api/profile"
   ```

3. Should return user profile data instead of 401.

## Deployment
The fix is in the codebase. The GitHub Actions pipeline will rebuild and deploy the profile service automatically.

## Related Issues
- Frontend sends tokens from `frontend-client` (public client)
- Backend was trying to introspect with `backend-api` (confidential client)
- Introspection fails for public client tokens

## Status
✅ Code fix committed
⏳ Waiting for pipeline to rebuild and deploy

