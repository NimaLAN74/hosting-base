#!/usr/bin/env python3
"""Fix authentication flow via Keycloak Admin API - create new browser flow without Conditional 2FA"""

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

print('=== Creating New Browser Flow Without Conditional 2FA ===\n')

# Get current browser flow executions to see what we need
executions_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/browser/executions'
executions_resp = requests.get(executions_url, headers=headers, timeout=10, verify=False)
executions = executions_resp.json()

print('Current browser flow executions:')
for i, exec_item in enumerate(executions):
    display_name = exec_item.get('displayName', exec_item.get('providerId', 'unknown'))
    provider_id = exec_item.get('providerId', 'unknown')
    requirement = exec_item.get('requirement', 'unknown')
    print(f'  {i+1}. {display_name} ({provider_id}) - {requirement}')
print()

# Get flow details
flows_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows'
flows_resp = requests.get(flows_url, headers=headers, timeout=10, verify=False)
flows = flows_resp.json()

browser_flow = None
for flow in flows:
    if flow.get('alias') == 'browser':
        browser_flow = flow
        break

if not browser_flow:
    print('❌ Browser flow not found')
    exit(1)

print(f'Found browser flow: {browser_flow.get("alias")} (ID: {browser_flow.get("id")})\n')

# Create a new flow based on browser but without Conditional 2FA
new_flow_alias = 'browser-no-mfa'
print(f'Creating new flow: {new_flow_alias}...')

new_flow_data = {
    'alias': new_flow_alias,
    'description': 'Browser flow without Conditional 2FA to prevent NullPointerException',
    'providerId': 'basic-flow',
    'topLevel': True,
    'builtIn': False
}

create_flow_resp = requests.post(flows_url, headers=headers, json=new_flow_data, timeout=10, verify=False)
if create_flow_resp.status_code not in [201, 409]:
    print(f'❌ Failed to create flow: {create_flow_resp.status_code} - {create_flow_resp.text}')
    exit(1)

if create_flow_resp.status_code == 409:
    print('⚠️  Flow already exists, continuing...')
else:
    print('✅ Flow created')

# Get the new flow ID
flows_resp = requests.get(flows_url, headers=headers, timeout=10, verify=False)
flows = flows_resp.json()
new_flow = None
for flow in flows:
    if flow.get('alias') == new_flow_alias:
        new_flow = flow
        break

if not new_flow:
    print('❌ New flow not found after creation')
    exit(1)

new_flow_id = new_flow.get('id')
print(f'New flow ID: {new_flow_id}\n')

# Add executions to the new flow (excluding Conditional 2FA and problematic ones)
executions_to_add = []

# Authenticators we want to include (in order)
authenticators_to_include = [
    ('auth-cookie', 'Cookie', 'ALTERNATIVE'),
    ('identity-provider-redirector', 'Identity Provider Redirector', 'DISABLED'),
    ('auth-username-password-form', 'Username Password Form', 'REQUIRED'),
]

# Get all available authenticators
auth_executors_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/authenticator-providers'
auth_executors_resp = requests.get(auth_executors_url, headers=headers, timeout=10, verify=False)
available_auths = auth_executors_resp.json() if auth_executors_resp.status_code == 200 else []

print('Adding authenticators to new flow...\n')

for provider_id, display_name, requirement in authenticators_to_include:
    execution_data = {
        'provider': provider_id
    }
    
    add_exec_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/{new_flow_alias}/executions/execution'
    add_exec_resp = requests.post(add_exec_url, headers=headers, json=execution_data, timeout=10, verify=False)
    
    if add_exec_resp.status_code == 204:
        print(f'  ✅ Added: {display_name} ({provider_id})')
        
        # Set requirement
        # Get execution ID
        execs_resp = requests.get(f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/{new_flow_alias}/executions', 
                                  headers=headers, timeout=10, verify=False)
        execs = execs_resp.json()
        exec_id = None
        for e in execs:
            if e.get('providerId') == provider_id:
                exec_id = e.get('id')
                break
        
        if exec_id and requirement != 'ALTERNATIVE':
            update_data = {'id': exec_id, 'requirement': requirement}
            update_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/{new_flow_alias}/executions'
            update_resp = requests.put(update_url, headers=headers, json=update_data, timeout=10, verify=False)
            if update_resp.status_code == 204:
                print(f'    Set requirement: {requirement}')
    else:
        print(f'  ⚠️  Failed to add {display_name}: {add_exec_resp.status_code}')

print()

# Set the new flow as the default browser flow
print('Setting new flow as default browser flow...')
realm_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}'
realm_resp = requests.get(realm_url, headers=headers, timeout=10, verify=False)
realm_config = realm_resp.json()

realm_config['browserFlow'] = new_flow_alias

update_realm_resp = requests.put(realm_url, headers=headers, json=realm_config, timeout=10, verify=False)
if update_realm_resp.status_code == 204:
    print('✅ New flow set as default browser flow')
    print('   Login should now work without Conditional 2FA issues!')
else:
    print(f'❌ Failed to set default flow: {update_realm_resp.status_code} - {update_realm_resp.text}')
    exit(1)

print('\n✅ Authentication flow fixed via API!')
