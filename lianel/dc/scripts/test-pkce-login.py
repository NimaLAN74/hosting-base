#!/usr/bin/env python3
"""Test Keycloak login URL with PKCE"""

import requests
import base64
import hashlib
import secrets
import urllib3
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

# Generate a proper PKCE code_challenge (same as keycloak-js does)
code_verifier = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode('utf-8').rstrip('=')
code_challenge_bytes = hashlib.sha256(code_verifier.encode('utf-8')).digest()
code_challenge = base64.urlsafe_b64encode(code_challenge_bytes).decode('utf-8').rstrip('=')

test_url = 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth'
test_params = {
    'client_id': 'frontend-client',
    'redirect_uri': 'https://www.lianel.se/',
    'state': 'test-proper-pkce',
    'response_mode': 'fragment',
    'response_type': 'code',
    'scope': 'openid',
    'prompt': 'login',
    'code_challenge_method': 'S256',
    'code_challenge': code_challenge
}

print('=== Testing Keycloak Login URL with PKCE ===')
print(f'URL: {test_url}')
print(f'code_challenge: {code_challenge}')
print(f'code_challenge length: {len(code_challenge)} chars')
print()

try:
    response = requests.get(test_url, params=test_params, allow_redirects=False, verify=False, timeout=10)
    location = response.headers.get('Location', '')
    
    print(f'Status Code: {response.status_code}')
    print()
    
    if response.status_code == 302:
        if '/realms/lianel/login' in location or '/login' in location:
            print('✅ SUCCESS - Redirects to Keycloak login page')
            print(f'   PKCE challenge accepted!')
            print(f'   Location: {location[:120]}...')
            exit(0)
        elif 'error' in location:
            error_parts = location.split('#error=')[1].split('&') if '#error=' in location else []
            error = error_parts[0] if error_parts else 'unknown'
            desc_parts = [p for p in error_parts if p.startswith('error_description=')]
            desc = desc_parts[0].replace('error_description=', '')[:100] if desc_parts else 'unknown'
            
            print(f'❌ ERROR: {error}')
            print(f'Description: {desc}')
            if 'code_challenge' in desc or 'PKCE' in desc:
                print('   Issue: PKCE validation failed')
            exit(1)
        else:
            print(f'⚠️  Unexpected redirect: {location[:150]}')
            exit(1)
    else:
        print(f'❌ Unexpected status: {response.status_code}')
        print(f'Response: {response.text[:300]}')
        exit(1)
        
except Exception as e:
    print(f'❌ Error: {e}')
    import traceback
    traceback.print_exc()
    exit(1)
