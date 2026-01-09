# E2E Test Results and Fixes

## Issues Found

### 1. Energy Data - Only 5 Records Showing

**Current Status:**
- API returns only 5 records total
- Expected: Hundreds/thousands of records after ingestion DAGs completed

**Root Cause:**
- Ingestion DAGs completed successfully
- Harmonization DAG is running
- Data may not be fully harmonized yet, or query filter is too restrictive

**Investigation:**
- Check `fact_energy_annual` table directly
- Verify `source_system = 'eurostat'` filter
- Check if harmonization has completed
- Verify data was actually inserted (not just test data)

**Fix:**
1. Wait for harmonization DAG to complete
2. Verify all ingestion DAGs inserted data (not just 5 test records)
3. Check if harmonization updates existing records vs creating new ones
4. Verify energy service query filters

### 2. Admin Services Not Visible

**Current Status:**
- Admin user cannot see Airflow/Grafana services
- Frontend uses backend API check (already implemented)
- Keycloak client scope may not be configured

**Root Cause:**
- `frontend-client` may not have `realm-roles` scope assigned
- User may not have admin role assigned
- Token may not contain roles (user needs to log out/in)

**Fix Steps:**

1. **Run Keycloak Client Scope Fix:**
   ```bash
   cd /root/lianel/dc
   source .env
   cd scripts
   bash fix-keycloak-client-scope-realm-roles.sh
   ```

2. **Assign Admin Role to User:**
   ```bash
   cd /root/lianel/dc/scripts
   bash assign-admin-role.sh <username>
   ```

3. **User Must Log Out and Log Back In:**
   - Token refresh won't work
   - Must completely log out and re-authenticate
   - This gets a new token with roles included

4. **Verify in Browser Console:**
   - Open browser console
   - Look for: `=== DASHBOARD ADMIN ROLE DEBUG ===`
   - Check: `Backend API isAdmin: true`
   - Check: `adminServices will be: SHOWN`

## Test Results

### Energy Data Test
```bash
# API Response
curl https://www.lianel.se/api/energy/annual?limit=5
# Result: Total: 5, Records: 5
```

**Status:** ❌ Only 5 records found

### Admin API Test
```bash
# Backend Admin Check Endpoint
curl https://www.lianel.se/api/admin/check
# Result: {"error":"Unauthorized"} (expected without auth)
```

**Status:** ✅ Endpoint accessible

### Keycloak Configuration Test
- Need to run: `fix-keycloak-client-scope-realm-roles.sh`
- Need to verify: User has admin role assigned
- Need to verify: User has logged out/in after role assignment

**Status:** ⚠️ Configuration needs verification

## Action Items

### Immediate Actions

1. **Wait for Harmonization DAG to Complete**
   - Check Airflow UI: https://airflow.lianel.se
   - Verify `eurostat_harmonization` DAG completes successfully
   - Check if more records appear after harmonization

2. **Fix Keycloak Client Scope**
   ```bash
   ssh root@72.60.80.84
   cd /root/lianel/dc
   source .env
   cd scripts
   bash fix-keycloak-client-scope-realm-roles.sh
   ```

3. **Assign Admin Role (if not already assigned)**
   ```bash
   cd /root/lianel/dc/scripts
   bash assign-admin-role.sh <admin-username>
   ```

4. **User Logout/Login**
   - User must completely log out
   - User must log back in
   - This refreshes the token with roles

5. **Verify Frontend**
   - Open https://www.lianel.se
   - Log in as admin user
   - Check browser console for debug output
   - Verify admin services (Airflow, Grafana) are visible

### Data Investigation

1. **Check Database Directly:**
   - Connect to PostgreSQL
   - Query: `SELECT COUNT(*) FROM fact_energy_annual WHERE source_system = 'eurostat';`
   - Check: `SELECT source_table, COUNT(*) FROM fact_energy_annual GROUP BY source_table;`

2. **Check Ingestion Logs:**
   - Check `meta_ingestion_log` table
   - Verify all ingestion DAGs logged successful runs
   - Check record counts per table

3. **Check Harmonization Status:**
   - Verify harmonization DAG completed
   - Check if `harmonisation_version` is set on records
   - Verify data was updated (not just inserted)

## Expected Results After Fixes

### Energy Data
- ✅ Hundreds/thousands of records available
- ✅ Multiple countries and years
- ✅ Data from all source tables (nrg_bal_s, nrg_cb_e, etc.)

### Admin Access
- ✅ Admin user sees Airflow service
- ✅ Admin user sees Grafana service
- ✅ Regular users only see Profile and Energy services
- ✅ Browser console shows: `Backend API isAdmin: true`
- ✅ Browser console shows: `adminServices will be: SHOWN`

## Testing Checklist

- [ ] Harmonization DAG completed successfully
- [ ] Energy API returns > 100 records
- [ ] Keycloak client scope fixed
- [ ] Admin role assigned to user
- [ ] User logged out and back in
- [ ] Admin services visible in frontend
- [ ] Browser console shows correct admin status
- [ ] Regular user cannot see admin services
