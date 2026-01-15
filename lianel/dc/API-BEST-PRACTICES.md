# API Best Practices
**Last Updated**: January 15, 2026

---

## Table of Contents

1. [Performance Optimization](#performance-optimization)
2. [Error Handling](#error-handling)
3. [Security](#security)
4. [Rate Limiting](#rate-limiting)
5. [Caching Strategies](#caching-strategies)
6. [Data Handling](#data-handling)
7. [Monitoring and Logging](#monitoring-and-logging)

---

## Performance Optimization

### 1. Use Pagination

**Always use pagination for large datasets:**

```python
# Good: Paginated request
def get_all_data(endpoint, token, page_size=1000):
    all_data = []
    offset = 0
    
    while True:
        response = requests.get(
            f"{endpoint}?limit={page_size}&offset={offset}",
            headers={"Authorization": f"Bearer {token}"}
        )
        data = response.json().get("data", [])
        all_data.extend(data)
        
        if len(data) < page_size:
            break
        offset += page_size
    
    return all_data
```

**Avoid**: Fetching all data in a single request without pagination.

### 2. Filter Server-Side

**Use query parameters to filter data:**

```python
# Good: Server-side filtering
response = requests.get(
    "https://www.lianel.se/api/v1/energy/annual",
    params={"country_code": "DE", "year": 2023},
    headers={"Authorization": f"Bearer {token}"}
)

# Bad: Client-side filtering
response = requests.get(
    "https://www.lianel.se/api/v1/energy/annual",
    headers={"Authorization": f"Bearer {token}"}
)
data = [r for r in response.json()["data"] if r["country_code"] == "DE" and r["year"] == 2023]
```

### 3. Request Only Needed Fields

**Use field selection when available:**

```python
# If API supports field selection
response = requests.get(
    "https://www.lianel.se/api/v1/datasets/forecasting",
    params={"fields": "country_code,year,value"},
    headers={"Authorization": f"Bearer {token}"}
)
```

### 4. Implement Connection Pooling

**Reuse HTTP connections:**

```python
import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

session = requests.Session()

# Configure retry strategy
retry_strategy = Retry(
    total=3,
    backoff_factor=1,
    status_forcelist=[429, 500, 502, 503, 504]
)
adapter = HTTPAdapter(max_retries=retry_strategy)
session.mount("https://", adapter)

# Reuse session for multiple requests
response = session.get(url, headers=headers)
```

### 5. Use Async Requests for Multiple Calls

**For parallel requests:**

```python
import asyncio
import aiohttp

async def fetch_data(session, url, headers):
    async with session.get(url, headers=headers) as response:
        return await response.json()

async def fetch_multiple(endpoints, token):
    headers = {"Authorization": f"Bearer {token}"}
    async with aiohttp.ClientSession() as session:
        tasks = [fetch_data(session, url, headers) for url in endpoints]
        return await asyncio.gather(*tasks)

# Usage
endpoints = [
    "https://www.lianel.se/api/v1/energy/annual?country_code=DE",
    "https://www.lianel.se/api/v1/energy/annual?country_code=FR",
    "https://www.lianel.se/api/v1/energy/annual?country_code=IT"
]
results = asyncio.run(fetch_multiple(endpoints, token))
```

---

## Error Handling

### 1. Check HTTP Status Codes

**Always check response status:**

```python
response = requests.get(url, headers=headers)

if response.status_code == 200:
    data = response.json()
elif response.status_code == 401:
    # Token expired or invalid
    token = refresh_token()
    # Retry request
elif response.status_code == 404:
    # Resource not found
    print("Resource not found")
elif response.status_code == 429:
    # Rate limited
    retry_after = int(response.headers.get("Retry-After", 60))
    time.sleep(retry_after)
    # Retry request
elif response.status_code >= 500:
    # Server error
    print(f"Server error: {response.status_code}")
    # Implement retry with exponential backoff
else:
    response.raise_for_status()
```

### 2. Implement Retry Logic

**Handle transient errors:**

```python
from tenacity import retry, stop_after_attempt, wait_exponential

@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=4, max=10)
)
def make_request(url, headers):
    response = requests.get(url, headers=headers, timeout=30)
    response.raise_for_status()
    return response.json()
```

### 3. Validate Response Data

**Always validate API responses:**

```python
def validate_response(response):
    """Validate API response structure."""
    if not response.ok:
        raise ValueError(f"Request failed: {response.status_code}")
    
    data = response.json()
    
    if "data" not in data:
        raise ValueError("Invalid response format: missing 'data' field")
    
    return data
```

### 4. Handle Timeouts

**Set appropriate timeouts:**

```python
# Set timeout for all requests
response = requests.get(url, headers=headers, timeout=30)

# Or configure session timeout
session = requests.Session()
session.timeout = 30
```

---

## Security

### 1. Secure Token Storage

**Never commit tokens to version control:**

```python
# Good: Use environment variables
import os
token = os.getenv("API_TOKEN")

# Good: Use secret management
from keyring import get_password
token = get_password("lianel_api", "token")

# Bad: Hardcoded tokens
token = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJ..."
```

### 2. Use HTTPS Only

**Always use HTTPS:**

```python
# Good: HTTPS
url = "https://www.lianel.se/api/v1/health"

# Bad: HTTP (insecure)
url = "http://www.lianel.se/api/v1/health"
```

### 3. Validate Inputs

**Validate all inputs before sending:**

```python
def validate_country_code(code):
    """Validate country code format."""
    if not isinstance(code, str) or len(code) != 2:
        raise ValueError("Country code must be 2 characters")
    return code.upper()

def validate_year(year):
    """Validate year range."""
    if not isinstance(year, int) or year < 2000 or year > 2100:
        raise ValueError("Year must be between 2000 and 2100")
    return year
```

### 4. Sanitize Logs

**Never log sensitive data:**

```python
# Good: Log without sensitive data
logger.info(f"API request to {endpoint}")

# Bad: Log tokens
logger.info(f"API request with token {token}")
```

---

## Rate Limiting

### 1. Respect Rate Limits

**Check rate limit headers:**

```python
def check_rate_limit(response):
    """Check and handle rate limits."""
    remaining = response.headers.get("X-RateLimit-Remaining")
    reset_time = response.headers.get("X-RateLimit-Reset")
    
    if remaining and int(remaining) < 10:
        # Slow down requests
        time.sleep(1)
    
    return remaining, reset_time
```

### 2. Implement Exponential Backoff

**For rate limit errors:**

```python
import time
import random

def exponential_backoff(attempt, base_delay=1, max_delay=60):
    """Calculate exponential backoff delay."""
    delay = min(base_delay * (2 ** attempt), max_delay)
    # Add jitter to prevent thundering herd
    jitter = random.uniform(0, delay * 0.1)
    return delay + jitter

def make_request_with_backoff(url, headers, max_retries=5):
    """Make request with exponential backoff."""
    for attempt in range(max_retries):
        try:
            response = requests.get(url, headers=headers)
            if response.status_code != 429:
                return response
            
            # Rate limited
            delay = exponential_backoff(attempt)
            time.sleep(delay)
        except requests.exceptions.RequestException as e:
            if attempt == max_retries - 1:
                raise
            delay = exponential_backoff(attempt)
            time.sleep(delay)
    
    raise Exception("Max retries exceeded")
```

### 3. Batch Requests When Possible

**Combine multiple requests:**

```python
# If API supports batch requests
def batch_request(endpoint, items, token):
    """Send batch request."""
    response = requests.post(
        f"{endpoint}/batch",
        json={"items": items},
        headers={"Authorization": f"Bearer {token}"}
    )
    return response.json()
```

---

## Caching Strategies

### 1. Cache Token Responses

**Cache authentication tokens:**

```python
from functools import lru_cache
from datetime import datetime, timedelta

class TokenCache:
    def __init__(self):
        self.token = None
        self.expires_at = None
    
    def get_token(self, client_id, client_secret):
        """Get cached token or fetch new one."""
        if self.token and self.expires_at and datetime.now() < self.expires_at:
            return self.token
        
        # Fetch new token
        token_data = get_access_token(client_id, client_secret)
        self.token = token_data["access_token"]
        self.expires_at = datetime.now() + timedelta(seconds=token_data["expires_in"] - 30)
        
        return self.token
```

### 2. Cache API Responses

**Cache static or slowly-changing data:**

```python
from functools import lru_cache
import hashlib
import json

def cache_key(*args, **kwargs):
    """Generate cache key from arguments."""
    key_data = json.dumps({"args": args, "kwargs": kwargs}, sort_keys=True)
    return hashlib.md5(key_data.encode()).hexdigest()

@lru_cache(maxsize=128)
def get_cached_data(endpoint, params):
    """Get cached API response."""
    # Implementation
    pass
```

### 3. Use ETags for Conditional Requests

**If API supports ETags:**

```python
def get_with_etag(url, headers, etag=None):
    """Make conditional request with ETag."""
    if etag:
        headers["If-None-Match"] = etag
    
    response = requests.get(url, headers=headers)
    
    if response.status_code == 304:
        # Not modified, use cached data
        return None
    
    new_etag = response.headers.get("ETag")
    return response.json(), new_etag
```

---

## Data Handling

### 1. Handle Large Datasets

**Stream large responses:**

```python
def stream_large_response(url, headers):
    """Stream large API response."""
    response = requests.get(url, headers=headers, stream=True)
    
    for line in response.iter_lines():
        if line:
            data = json.loads(line)
            yield data
```

### 2. Validate Data Types

**Validate response data:**

```python
from typing import List, Dict

def validate_energy_data(data: List[Dict]) -> List[Dict]:
    """Validate energy data structure."""
    required_fields = ["country_code", "year", "value"]
    
    for record in data:
        for field in required_fields:
            if field not in record:
                raise ValueError(f"Missing required field: {field}")
        
        if not isinstance(record["value"], (int, float)):
            raise ValueError(f"Invalid value type: {type(record['value'])}")
    
    return data
```

### 3. Handle Null Values

**Handle missing or null data:**

```python
def safe_get(data, key, default=None):
    """Safely get value from dictionary."""
    value = data.get(key)
    if value is None:
        return default
    return value

# Usage
energy_value = safe_get(record, "value", 0)
renewable_pct = safe_get(record, "renewable_percentage", None)
```

---

## Monitoring and Logging

### 1. Log API Calls

**Log all API interactions:**

```python
import logging

logger = logging.getLogger(__name__)

def log_api_call(method, url, status_code, duration):
    """Log API call details."""
    logger.info(
        f"API {method} {url} - Status: {status_code} - Duration: {duration:.3f}s"
    )
```

### 2. Monitor Response Times

**Track API performance:**

```python
import time

def timed_request(url, headers):
    """Make request and measure duration."""
    start = time.time()
    response = requests.get(url, headers=headers)
    duration = time.time() - start
    
    log_api_call("GET", url, response.status_code, duration)
    
    return response
```

### 3. Track Error Rates

**Monitor API errors:**

```python
from collections import defaultdict

error_counts = defaultdict(int)

def track_errors(status_code):
    """Track error rates."""
    if status_code >= 400:
        error_counts[status_code] += 1
    
    # Alert if error rate is high
    total_errors = sum(error_counts.values())
    if total_errors > 100:
        logger.warning(f"High error rate: {total_errors} errors")
```

---

## Summary Checklist

- [ ] Use pagination for large datasets
- [ ] Filter data server-side
- [ ] Implement proper error handling
- [ ] Use HTTPS only
- [ ] Store tokens securely
- [ ] Respect rate limits
- [ ] Implement retry logic
- [ ] Cache tokens and responses
- [ ] Validate all inputs
- [ ] Log API calls (without sensitive data)
- [ ] Monitor response times
- [ ] Handle timeouts appropriately

---

**Status**: Active  
**Last Review**: January 15, 2026
