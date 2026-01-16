#!/usr/bin/env python3
"""Disable Conditional 2FA subflow - it checks user before user exists"""

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

print('=== Disabling Conditional 2FA Subflow ===\n')

# Find "Browser - Conditional 2FA" subflow
conditional_2fa = None
for exec_item in executions:
    display_name = exec_item.get('displayName', '')
    if 'Conditional 2FA' in display_name:
        conditional_2fa = exec_item
        break

if conditional_2fa:
    exec_id = conditional_2fa.get('id')
    requirement = conditional_2fa.get('requirement', '')
    configurable = conditional_2fa.get('configurable', False)
    
    print(f'Found: {conditional_2fa.get("displayName")} (ID: {exec_id})')
    print(f'  Current Requirement: {requirement}')
    print(f'  Configurable: {configurable}\n')
    
    if requirement != 'DISABLED':
        if configurable:
            print('Disabling Conditional 2FA subflow...')
            update_data = {
                'id': exec_id,
                'requirement': 'DISABLED'
            }
            
            update_resp = requests.put(executions_url, headers=headers, json=update_data, timeout=10, verify=False)
            if update_resp.status_code == 204:
                print('✅ Conditional 2FA subflow disabled successfully')
                print('   This should fix the NullPointerException (user.credentialManager())')
                print('   Login should now work!')
                exit(0)
            else:
                print(f'❌ Failed: {update_resp.status_code} - {update_resp.text}')
                exit(1)
        else:
            print('⚠️  Conditional 2FA is not configurable')
            print('   Cannot disable - may need to remove MFA requirement or create custom flow')
            exit(1)
    else:
        print('✅ Conditional 2FA is already disabled')
        print('   But error persists - checking other causes...')
else:
    print('⚠️  Conditional 2FA subflow not found')
    print('   Error may be from a different authenticator')

print('\n✅ Analysis complete')
