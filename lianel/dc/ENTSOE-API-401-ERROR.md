# ENTSO-E API 401 Unauthorized Error

## Problem

The ENTSO-E API is returning **401 Unauthorized** errors when attempting to fetch data, even though the token is set correctly.

## Evidence

- ✅ Token is set in Airflow Variables: `ENTSOE_API_TOKEN`
- ✅ Token is being sent in API requests (visible in request URL)
- ❌ API returns `401 Client Error` for all requests
- ✅ DAG completes successfully but inserts 0 records
- ✅ No data in `fact_electricity_timeseries` table

## API Error Details

```
401 Client Error: for url: https://web-api.tp.entsoe.eu/api?securityToken=bb068848-4b90-4a96-b2ad-43e06c402d03&documentType=A65&processType=A16&outBiddingZone_Domain=10YSE-1--------K&periodStart=20260119&periodEnd=20260120
```

## Possible Causes

1. **Token is invalid or expired**
   - The token may have been revoked or expired
   - Check token status in ENTSO-E account

2. **Token needs activation/approval**
   - New tokens may require manual approval from ENTSO-E
   - Check your ENTSO-E account for pending approvals

3. **Token format is incorrect**
   - Verify the token format matches ENTSO-E requirements
   - Ensure no extra spaces or characters

4. **API endpoint or authentication method changed**
   - ENTSO-E may have updated their API
   - Check ENTSO-E API documentation for changes

5. **Account restrictions**
   - Your account may have restrictions on API access
   - Some accounts may need additional permissions

## Solutions

### Option 1: Verify Token in ENTSO-E Account

1. Log in to https://transparency.entsoe.eu/
2. Navigate to "Web Services" or "API Access"
3. Check token status:
   - Is it active?
   - Is it approved?
   - Has it expired?
4. If expired or invalid, generate a new token

### Option 2: Generate New Token

1. Log in to ENTSO-E Transparency Platform
2. Go to API Access section
3. Revoke old token (if needed)
4. Generate new token
5. Update Airflow variable:
   ```bash
   cd /root/lianel/dc
   ./scripts/set-entsoe-token.sh <new-token>
   ```
6. Trigger DAG again

### Option 3: Check API Documentation

1. Review ENTSO-E API documentation:
   - https://transparency.entsoe.eu/content/static_content/Static%20content/web%20api/Guide.html
2. Verify:
   - API endpoint URL is correct
   - Authentication method is correct
   - Parameter format is correct

### Option 4: Contact ENTSO-E Support

If the token should be working but isn't:
1. Contact ENTSO-E support
2. Provide:
   - Your account email
   - Token (first/last few characters)
   - Error message (401 Unauthorized)
   - Request details

## Testing Token

To test if a token works, you can use:

```bash
# On remote host
docker exec dc-airflow-apiserver-1 python3 -c "
import sys
sys.path.insert(0, '/opt/airflow/dags/utils')
from entsoe_client import ENTSOEClient
from airflow.models import Variable

token = Variable.get('ENTSOE_API_TOKEN')
client = ENTSOEClient(api_token=token)

# Test with a recent date
load_data = client.get_load_data('SE', '2026-01-19', '2026-01-20')
print(f'Records: {len(load_data)}')
"
```

If this returns 0 records and shows 401 errors in logs, the token is invalid.

## Current Status

- **Token**: `bb068848-4b90-4a96-b2ad-43e06c402d03` (set in Airflow)
- **Status**: ❌ Rejected by API (401 Unauthorized)
- **DAG**: ✅ Running correctly (completes but no data)
- **Table**: ❌ Empty (0 records)

## Next Steps

1. Verify token status in ENTSO-E account
2. Generate new token if needed
3. Update Airflow variable with new token
4. Re-run DAG to test
