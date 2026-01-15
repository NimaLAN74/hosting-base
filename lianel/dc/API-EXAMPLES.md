# API Examples and Code Samples
**Last Updated**: January 15, 2026  
**Base URL**: `https://www.lianel.se/api/v1`

---

## Table of Contents

1. [Getting an Access Token](#getting-an-access-token)
2. [Python Examples](#python-examples)
3. [JavaScript/TypeScript Examples](#javascripttypescript-examples)
4. [cURL Examples](#curl-examples)
5. [Common Use Cases](#common-use-cases)

---

## Getting an Access Token

### Using Keycloak Directly

```bash
# Get access token
TOKEN=$(curl -X POST "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=your-client-id" \
  -d "client_secret=your-client-secret" \
  -d "grant_type=client_credentials" \
  -d "scope=openid profile email" | jq -r '.access_token')

echo $TOKEN
```

### Using OAuth2 Authorization Code Flow (Web Apps)

```javascript
// Redirect user to Keycloak for authentication
const authUrl = `https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth?` +
  `client_id=your-client-id&` +
  `redirect_uri=${encodeURIComponent('https://your-app.com/callback')}&` +
  `response_type=code&` +
  `scope=openid profile email`;

window.location.href = authUrl;

// After callback, exchange code for token
const tokenResponse = await fetch('https://auth.lianel.se/realms/lianel/protocol/openid-connect/token', {
  method: 'POST',
  headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
  body: new URLSearchParams({
    grant_type: 'authorization_code',
    client_id: 'your-client-id',
    client_secret: 'your-client-secret',
    code: codeFromCallback,
    redirect_uri: 'https://your-app.com/callback'
  })
});

const { access_token } = await tokenResponse.json();
```

---

## Python Examples

### Example 1: Get Forecasting Dataset

```python
import requests
import json

# Get access token (using client credentials)
token_url = "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token"
token_data = {
    "client_id": "your-client-id",
    "client_secret": "your-client-secret",
    "grant_type": "client_credentials"
}
token_response = requests.post(token_url, data=token_data)
access_token = token_response.json()["access_token"]

# Make API request
api_url = "https://www.lianel.se/api/v1/datasets/forecasting"
headers = {
    "Authorization": f"Bearer {access_token}",
    "Content-Type": "application/json"
}

params = {
    "country_code": "DE",
    "year": 2023,
    "limit": 100
}

response = requests.get(api_url, headers=headers, params=params)
data = response.json()

print(json.dumps(data, indent=2))
```

### Example 2: Get Energy Data with Filtering

```python
import requests

def get_energy_data(country_code, start_year, end_year, access_token):
    """Get energy data for a country over a year range."""
    url = "https://www.lianel.se/api/v1/energy/annual"
    headers = {"Authorization": f"Bearer {access_token}"}
    
    params = {
        "country_code": country_code,
        "year_min": start_year,
        "year_max": end_year
    }
    
    response = requests.get(url, headers=headers, params=params)
    response.raise_for_status()
    return response.json()

# Usage
token = get_access_token()  # Your token function
data = get_energy_data("DE", 2020, 2023, token)
print(f"Found {len(data['data'])} records")
```

### Example 3: Get Geo Features

```python
import requests

def get_geo_features(region_id, feature_types=None, access_token=None):
    """Get OSM features for a region."""
    url = "https://www.lianel.se/api/v1/geo/features"
    headers = {}
    
    if access_token:
        headers["Authorization"] = f"Bearer {access_token}"
    
    params = {"region_id": region_id}
    
    if feature_types:
        params["feature_types"] = ",".join(feature_types)
    
    response = requests.get(url, headers=headers, params=params)
    response.raise_for_status()
    return response.json()

# Usage
features = get_geo_features(
    region_id=123,
    feature_types=["power", "wind", "solar"],
    access_token=token
)

for feature_type, items in features.items():
    print(f"{feature_type}: {len(items)} features")
```

### Example 4: Error Handling

```python
import requests
from requests.exceptions import RequestException

def safe_api_call(url, headers, params=None):
    """Make API call with proper error handling."""
    try:
        response = requests.get(url, headers=headers, params=params, timeout=30)
        response.raise_for_status()
        return response.json()
    except requests.exceptions.HTTPError as e:
        if e.response.status_code == 401:
            print("Authentication failed. Check your token.")
        elif e.response.status_code == 404:
            print("Resource not found.")
        elif e.response.status_code == 429:
            print("Rate limit exceeded. Please retry later.")
        else:
            print(f"HTTP error: {e.response.status_code}")
        raise
    except requests.exceptions.Timeout:
        print("Request timed out.")
        raise
    except requests.exceptions.RequestException as e:
        print(f"Request failed: {e}")
        raise

# Usage
try:
    data = safe_api_call(api_url, headers, params)
except RequestException:
    # Handle error
    pass
```

---

## JavaScript/TypeScript Examples

### Example 1: React Hook for API Calls

```typescript
import { useState, useEffect } from 'react';

interface ApiResponse<T> {
  data: T;
  error?: string;
  loading: boolean;
}

function useApiData<T>(url: string, token: string | null): ApiResponse<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!token) {
      setError('No access token');
      setLoading(false);
      return;
    }

    fetch(url, {
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      }
    })
      .then(response => {
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        return response.json();
      })
      .then(data => {
        setData(data);
        setError(null);
      })
      .catch(err => {
        setError(err.message);
      })
      .finally(() => {
        setLoading(false);
      });
  }, [url, token]);

  return { data, error, loading };
}

// Usage in component
function EnergyDataComponent() {
  const token = useAuthToken(); // Your auth hook
  const { data, error, loading } = useApiData(
    'https://www.lianel.se/api/v1/energy/annual?country_code=DE&year=2023',
    token
  );

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error}</div>;
  if (!data) return <div>No data</div>;

  return <div>{/* Render data */}</div>;
}
```

### Example 2: Fetch with Retry Logic

```javascript
async function fetchWithRetry(url, options, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const response = await fetch(url, options);
      
      if (response.status === 429) {
        // Rate limited - wait and retry
        const retryAfter = response.headers.get('Retry-After') || Math.pow(2, i);
        await new Promise(resolve => setTimeout(resolve, retryAfter * 1000));
        continue;
      }
      
      response.raise_for_status();
      return await response.json();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      await new Promise(resolve => setTimeout(resolve, Math.pow(2, i) * 1000));
    }
  }
}

// Usage
const data = await fetchWithRetry(
  'https://www.lianel.se/api/v1/datasets/forecasting',
  {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json'
    }
  }
);
```

### Example 3: Batch Requests

```javascript
async function batchFetch(urls, token) {
  const requests = urls.map(url =>
    fetch(url, {
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      }
    }).then(res => res.json())
  );

  return Promise.all(requests);
}

// Usage
const countries = ['DE', 'FR', 'IT', 'ES'];
const urls = countries.map(code =>
  `https://www.lianel.se/api/v1/energy/annual?country_code=${code}&year=2023`
);

const results = await batchFetch(urls, token);
results.forEach((data, index) => {
  console.log(`${countries[index]}: ${data.data.length} records`);
});
```

---

## cURL Examples

### Example 1: Basic API Call

```bash
# Get access token
TOKEN=$(curl -X POST "https://auth.lianel.se/realms/lianel/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=your-client-id" \
  -d "client_secret=your-client-secret" \
  -d "grant_type=client_credentials" | jq -r '.access_token')

# Get forecasting data
curl -X GET "https://www.lianel.se/api/v1/datasets/forecasting?country_code=DE&year=2023&limit=10" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" | jq
```

### Example 2: Filtered Query

```bash
# Get energy data with multiple filters
curl -X GET "https://www.lianel.se/api/v1/energy/annual?country_code=DE&year_min=2020&year_max=2023&energy_type=renewable" \
  -H "Authorization: Bearer $TOKEN" | jq '.data[] | {country: .country_code, year: .year, value: .value}'
```

### Example 3: Geo Features Query

```bash
# Get OSM features for a region
curl -X GET "https://www.lianel.se/api/v1/geo/features?region_id=123&feature_types=power,wind,solar" \
  -H "Authorization: Bearer $TOKEN" | jq
```

### Example 4: Pagination

```bash
# Get first page
curl -X GET "https://www.lianel.se/api/v1/datasets/forecasting?limit=100&offset=0" \
  -H "Authorization: Bearer $TOKEN" | jq

# Get second page
curl -X GET "https://www.lianel.se/api/v1/datasets/forecasting?limit=100&offset=100" \
  -H "Authorization: Bearer $TOKEN" | jq
```

---

## Common Use Cases

### Use Case 1: Download All Data for Analysis

```python
import requests
import json
from typing import List, Dict

def download_all_forecasting_data(access_token: str) -> List[Dict]:
    """Download all forecasting dataset records."""
    all_data = []
    offset = 0
    limit = 1000
    
    while True:
        url = f"https://www.lianel.se/api/v1/datasets/forecasting?limit={limit}&offset={offset}"
        headers = {"Authorization": f"Bearer {access_token}"}
        
        response = requests.get(url, headers=headers)
        response.raise_for_status()
        result = response.json()
        
        all_data.extend(result.get("data", []))
        
        # Check if there's more data
        if len(result.get("data", [])) < limit:
            break
        
        offset += limit
        print(f"Downloaded {len(all_data)} records...")
    
    return all_data

# Usage
data = download_all_forecasting_data(token)
print(f"Total records: {len(data)}")
```

### Use Case 2: Compare Countries

```python
def compare_countries(countries: List[str], year: int, access_token: str):
    """Compare energy metrics across countries."""
    results = {}
    
    for country in countries:
        url = f"https://www.lianel.se/api/v1/energy/annual"
        params = {"country_code": country, "year": year}
        headers = {"Authorization": f"Bearer {access_token}"}
        
        response = requests.get(url, headers=headers, params=params)
        data = response.json().get("data", [])
        
        results[country] = {
            "total_energy": sum(r["value"] for r in data),
            "renewable_pct": next(
                (r["renewable_percentage"] for r in data if "renewable_percentage" in r),
                None
            )
        }
    
    return results

# Usage
comparison = compare_countries(["DE", "FR", "IT"], 2023, token)
for country, metrics in comparison.items():
    print(f"{country}: {metrics['total_energy']} GWh, {metrics['renewable_pct']}% renewable")
```

### Use Case 3: Time Series Analysis

```python
import pandas as pd
from datetime import datetime

def get_time_series(country_code: str, start_year: int, end_year: int, access_token: str):
    """Get time series data and convert to pandas DataFrame."""
    url = "https://www.lianel.se/api/v1/energy/annual"
    params = {
        "country_code": country_code,
        "year_min": start_year,
        "year_max": end_year
    }
    headers = {"Authorization": f"Bearer {access_token}"}
    
    response = requests.get(url, headers=headers, params=params)
    data = response.json().get("data", [])
    
    # Convert to DataFrame
    df = pd.DataFrame(data)
    df['date'] = pd.to_datetime(df['year'], format='%Y')
    df = df.set_index('date').sort_index()
    
    return df

# Usage
df = get_time_series("DE", 2020, 2023, token)
print(df[['value', 'renewable_percentage']].describe())
```

### Use Case 4: Real-time Monitoring

```python
import time
from datetime import datetime

def monitor_api_health(interval=60):
    """Monitor API health and response times."""
    while True:
        try:
            start = time.time()
            response = requests.get(
                "https://www.lianel.se/api/v1/health",
                timeout=5
            )
            elapsed = time.time() - start
            
            status = "✅ OK" if response.status_code == 200 else "❌ ERROR"
            print(f"{datetime.now()}: {status} - {elapsed:.3f}s")
            
        except Exception as e:
            print(f"{datetime.now()}: ❌ ERROR - {e}")
        
        time.sleep(interval)

# Usage (run in background thread)
# threading.Thread(target=monitor_api_health, daemon=True).start()
```

---

## Best Practices

### 1. Token Management
- Store tokens securely (environment variables, secret management)
- Refresh tokens before expiration
- Handle token expiration gracefully

### 2. Error Handling
- Always check HTTP status codes
- Implement retry logic for transient errors
- Log errors for debugging

### 3. Rate Limiting
- Respect rate limits (check `Retry-After` header)
- Implement exponential backoff
- Cache responses when possible

### 4. Performance
- Use pagination for large datasets
- Request only needed fields
- Implement client-side caching

### 5. Security
- Never commit tokens to version control
- Use HTTPS only
- Validate and sanitize inputs

---

**Status**: Active  
**Last Review**: January 15, 2026
