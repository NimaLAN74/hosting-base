#!/usr/bin/env python3
"""Disable all conditional authenticators that might cause issues before user authentication"""

import requests
import json
import os
import urllib3
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

KEYCLOAK_URL = os.getenv('KEYCLOAK_URL', 'https://auth.lianel.se')
REALM_NAME = os.getenv('REALM_NAME', 'lianel')
ADMIN_PASSWORD = os.getenv('KEYCLOAK_ADMIN_PASSWORD', 'D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA')

# Get admin token
token_url = f'{KEYCLOAK_URL}/realms/master/protocol/openid-connect/token'
token_data = {'username': 'admin', 'password': ADMIN_PASSWORD, 'grant_type': 'password', 'client_id': 'admin-cli'}
token_resp = requests.post(token_url, data=token_data, timeout=10, verify=False)
token = token_resp.json().get('access_token')
headers = {'Authorization': f'Bearer {token}'}

# Get browser flow executions
executions_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/browser/executions'
executions_resp = requests.get(executions_url, headers=headers, timeout=10, verify=False)
executions = executions_resp.json()

print('=== Disabling Problematic Conditional Authenticators ===\n')

# List of problematic authenticators that check user before user exists
problematic_providers = [
    'conditional-user-configured',
    'conditional-credential',
    'identity-provider-redirector',
]

# Also check for Organization-related conditionals
problematic_keywords = ['Organization', 'Identity-First']

fixed = []
already_disabled = []
not_configurable = []

for exec_item in executions:
    display_name = exec_item.get('displayName', '')
    provider_id = exec_item.get('providerId', '')
    exec_id = exec_item.get('id')
    requirement = exec_item.get('requirement', '')
    configurable = exec_item.get('configurable', False)
    
    should_disable = False
    reason = None
    
    # Check if it's a problematic provider
    if provider_id in problematic_providers:
        should_disable = True
        reason = f'Provider type: {provider_id}'
    elif any(keyword in display_name for keyword in problematic_keywords):
        should_disable = True
        reason = f'Contains problematic keyword'
    
    if should_disable:
        print(f'Found: {display_name} (Provider: {provider_id})')
        print(f'  Reason: {reason}')
        print(f'  Current Requirement: {requirement}')
        
        if requirement == 'DISABLED':
            print(f'  ✅ Already disabled\n')
            already_disabled.append(display_name)
        elif not configurable:
            print(f'  ⚠️  Not configurable - cannot disable\n')
            not_configurable.append(display_name)
        else:
            update_data = {
                'id': exec_id,
                'requirement': 'DISABLED'
            }
            
            update_resp = requests.put(executions_url, headers=headers, json=update_data, timeout=10, verify=False)
            if update_resp.status_code == 204:
                print(f'  ✅ Disabled successfully\n')
                fixed.append(display_name)
            else:
                print(f'  ❌ Failed: {update_resp.status_code} - {update_resp.text}\n')

print('\n=== Summary ===')
if fixed:
    print(f'✅ Fixed ({len(fixed)}): {", ".join(fixed)}')
if already_disabled:
    print(f'✓ Already disabled ({len(already_disabled)}): {", ".join(already_disabled)}')
if not_configurable:
    print(f'⚠️  Not configurable ({len(not_configurable)}): {", ".join(not_configurable)}')

if fixed:
    print('\n✅ Login should now work!')
elif not_configurable:
    print('\n⚠️  Some authenticators cannot be disabled - may need to check Keycloak logs')
