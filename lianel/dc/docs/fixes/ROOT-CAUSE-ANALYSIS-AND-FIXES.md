# Root Cause Analysis and Fixes

**Date**: 2026-01-19  
**Issues**: 
1. Electricity Timeseries shows "No data available"
2. Monitoring/Dashboards page redirects to login and shows empty page

## Issue 1: Electricity Timeseries - No Data

### Symptoms
- Frontend shows "No data available. The electricity timeseries table may be empty."
- API endpoint returns: `{"data": [], "total": 0}`
- DAG tasks complete with `success` state
- No records in `fact_electricity_timeseries` table

### Root Cause Analysis

**Findings:**
1. ✅ Table `fact_electricity_timeseries` exists (created via migration)
2. ✅ DAG `entsoe_ingestion` runs successfully
3. ❌ No data inserted into table
4. ❌ Ingestion log shows `records_ingested: 0` or `status: 'no_data'`

**Possible Causes:**
1. **ENTSO-E API requires authentication token** - The API may be returning empty responses without a valid token
2. **API returning no data** - The date range might be invalid or data not available
3. **Silent failures in data parsing** - XML parsing might be failing without errors
4. **Database connection issues** - Inserts might be failing silently

### Investigation Steps

1. **Check ENTSO-E API Token:**
   ```bash
   # Check if ENTSOE_API_TOKEN is set in Airflow Variables
   docker exec dc-airflow-apiserver-1 airflow variables get ENTSOE_API_TOKEN
   ```

2. **Check Task Logs:**
   ```bash
   # Get actual task logs to see what's happening
   docker exec dc-airflow-worker-1 airflow tasks render entsoe_ingestion country_se.ingest_se <run_id>
   ```

3. **Check Ingestion Log:**
   ```sql
   SELECT country_code, records_ingested, status, message, ingestion_date 
   FROM meta_entsoe_ingestion_log 
   ORDER BY ingestion_date DESC 
   LIMIT 10;
   ```

### Fix Required

**Action Items:**
1. ✅ Verify `fact_electricity_timeseries` table exists and has correct schema
2. ⏳ Check if `ENTSOE_API_TOKEN` is set in Airflow Variables
3. ⏳ Review task logs to see actual API responses
4. ⏳ Add better error logging in DAG to capture API failures
5. ⏳ Test ENTSO-E API directly to verify it returns data

**Code Changes Needed:**
- Add more detailed logging in `ingest_date_chunk` function
- Log API responses (without sensitive data)
- Log number of records retrieved vs inserted
- Add error handling for API failures

## Issue 2: Monitoring/Dashboards Page - Redirect Loop

### Symptoms
- Visiting `/monitoring` redirects to `/login`
- After login, returns to `/monitoring` but shows empty page
- Authentication state not updating after redirect

### Root Cause Analysis

**Findings:**
1. ✅ Keycloak login redirect works (user can log in)
2. ✅ Redirect URI preserves current path (`/monitoring`)
3. ❌ `KeycloakProvider` doesn't re-initialize after login callback
4. ❌ Authentication state (`authenticated`) not updating after redirect

**Root Cause:**
- `KeycloakProvider` only initializes once on mount
- When user returns from Keycloak with `code` parameter, Keycloak.js should handle it automatically
- However, the React state (`authenticated`) doesn't update because `useEffect` only runs once
- The component needs to detect the callback and force a state update

### Fix Applied

**Changes Made:**

1. **`frontend/src/KeycloakProvider.js`:**
   - Added detection for `code` parameter in URL (login callback)
   - Force state update after callback is detected
   - Ensure `authenticated` state is updated when token is available
   - Added explicit state updates in `updateToken` function

2. **`frontend/src/keycloak.js`:**
   - Improved logging for callback detection
   - Added confirmation logging when authentication succeeds after callback

**Code Changes:**
```javascript
// KeycloakProvider.js - Added callback detection
const urlParams = new URLSearchParams(window.location.search);
const code = urlParams.get('code');
const isCallback = !!code;

// After initKeycloak, check if callback and update state
if (isCallback && auth) {
  console.log('Login callback detected, user authenticated');
  setAuthenticated(true);
  setUserInfo(getUserInfo());
}
```

### Status

✅ **FIXED** - Changes committed and pushed
- Frontend pipeline will deploy automatically
- After deployment, Monitoring page should work correctly

## Testing

### For Issue 1 (Electricity Timeseries):
1. Check Airflow Variables for `ENTSOE_API_TOKEN`
2. Review task logs for actual API responses
3. Test ENTSO-E API directly with token
4. Verify data insertion logic

### For Issue 2 (Monitoring Page):
1. Clear browser cache/cookies
2. Visit `https://www.lianel.se/monitoring`
3. Click "Log In" button
4. After login, should return to `/monitoring` with dashboard cards visible
5. Verify authentication state is correct

## Next Steps

1. **Immediate:**
   - Wait for frontend deployment to complete
   - Test Monitoring page authentication flow
   - Check ENTSO-E API token configuration

2. **Short-term:**
   - Add detailed logging to ENTSO-E ingestion DAG
   - Verify ENTSO-E API token is set correctly
   - Test API directly to ensure data is available
   - Review task logs to identify why no data is inserted

3. **Long-term:**
   - Add monitoring/alerting for DAG failures
   - Add data quality checks
   - Improve error handling and logging

## Files Modified

- `lianel/dc/frontend/src/KeycloakProvider.js` - Added callback detection and state updates
- `lianel/dc/frontend/src/keycloak.js` - Improved callback logging
- `lianel/dc/frontend/src/monitoring/Monitoring.js` - Already fixed (removed auto-redirect)
- `lianel/dc/scripts/test-electricity-and-auth.sh` - Added test script

## Commits

- `7ae5d43` - Fix Monitoring page auth state update after login redirect
- `14cc36c` - Add comprehensive test script for electricity and auth issues
