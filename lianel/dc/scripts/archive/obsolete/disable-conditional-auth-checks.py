#!/usr/bin/env python3
"""Disable conditional authenticators that check user before user exists"""

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

print('=== Disabling Conditional Authenticators That Check User Before User Exists ===\n')

# Find and disable problematic conditional authenticators
fixed = False

for exec_item in executions:
    display_name = exec_item.get('displayName', '')
    provider_id = exec_item.get('providerId', '')
    exec_id = exec_item.get('id')
    requirement = exec_item.get('requirement', '')
    configurable = exec_item.get('configurable', False)
    
    # Disable "Condition - user configured" and "Condition - credential"
    # These check user.credentialManager() before user exists
    if provider_id in ['conditional-user-configured', 'conditional-credential']:
        if requirement != 'DISABLED' and configurable:
            print(f'Disabling: {display_name} (ID: {exec_id}, Provider: {provider_id})')
            
            # Update to DISABLED
            update_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/browser/executions'
            update_data = {
                'id': exec_id,
                'requirement': 'DISABLED'
            }
            
            update_resp = requests.put(executions_url, headers=headers, json=update_data, timeout=10, verify=False)
            if update_resp.status_code == 204:
                print(f'  ✅ Disabled successfully\n')
                fixed = True
            else:
                print(f'  ❌ Failed: {update_resp.status_code} - {update_resp.text}\n')

if not fixed:
    print('⚠️  No fixable conditional authenticators found')
    print('   They may be non-configurable or already disabled')
else:
    print('✅ Conditional authenticators fixed!')
    print('   Login should now work without NullPointerException')
