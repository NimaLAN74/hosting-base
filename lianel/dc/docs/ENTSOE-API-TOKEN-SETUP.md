# ENTSO-E API Token Setup Guide

## Overview

The ENTSO-E (European Network of Transmission System Operators for Electricity) Transparency Platform API requires registration and an API token to access data.

## How to Get an ENTSO-E API Token

### Step 1: Register on ENTSO-E Transparency Platform

1. Visit: https://transparency.entsoe.eu/
2. Click on "Registration" or "Sign Up"
3. Create an account with your email
4. Verify your email address

### Step 2: Request API Access

1. Log in to your account
2. Navigate to "Web Services" or "API Access" section
3. Request API access (may require approval)
4. Once approved, you'll receive an API token

### Step 3: Set Token in Airflow

Once you have the token, set it in Airflow Variables:

```bash
# On remote host
docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN "your-token-here"
```

Or via Airflow UI:
1. Go to Admin → Variables
2. Add new variable:
   - Key: `ENTSOE_API_TOKEN`
   - Value: `your-token-here`

### Step 4: Verify Token is Set

```bash
docker exec dc-airflow-apiserver-1 airflow variables get ENTSOE_API_TOKEN
```

### Step 5: Re-run DAG

After setting the token, trigger the `entsoe_ingestion` DAG to start ingesting data.

## Important Notes

- The API token is required for all ENTSO-E API requests
- Without the token, the API will return empty responses
- The token should be kept secure and not committed to version control
- Token may expire and need renewal

## Alternative: Test Without Token

Some endpoints may work without a token for limited testing, but production use requires registration and a valid token.

## References

- ENTSO-E Transparency Platform: https://transparency.entsoe.eu/
- API Documentation: https://transparency.entsoe.eu/content/static_content/Static%20content/web%20api/Guide.html
