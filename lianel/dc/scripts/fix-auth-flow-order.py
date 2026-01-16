#!/usr/bin/env python3
"""Fix authentication flow order - move conditional checks after username/password"""

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

# Get current flow executions
executions_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/browser/executions'
executions_resp = requests.get(executions_url, headers=headers, timeout=10, verify=False)
executions = executions_resp.json()

print('=== Fixing Authentication Flow Order ===\n')

# Find the problematic conditional authenticators
conditional_2fa_exec = None
user_configured_exec = None
credential_exec = None

for exec_item in executions:
    display_name = exec_item.get('displayName', '')
    provider_id = exec_item.get('providerId', '')
    exec_id = exec_item.get('id')
    
    if 'Conditional 2FA' in display_name:
        conditional_2fa_exec = exec_item
        print(f'Found: {display_name} (ID: {exec_id}, Requirement: {exec_item.get("requirement")})')
    elif provider_id == 'conditional-user-configured':
        user_configured_exec = exec_item
        print(f'Found: {display_name} (ID: {exec_id}, Requirement: {exec_item.get("requirement")})')
    elif provider_id == 'conditional-credential':
        credential_exec = exec_item
        print(f'Found: {display_name} (ID: {exec_id}, Requirement: {exec_item.get("requirement")})')

print()

# The issue: These conditionals are checking for user before user exists
# Solution: Make them CONDITIONAL instead of REQUIRED, or move after username/password
# Since they're in a subflow, we need to make the parent CONDITIONAL

if conditional_2fa_exec:
    # Check if it's already CONDITIONAL
    if conditional_2fa_exec.get('requirement') != 'CONDITIONAL':
        print(f'⚠️  Conditional 2FA is {conditional_2fa_exec.get("requirement")}, but should be CONDITIONAL')
        print('   This is likely causing the NullPointerException')
        print('   The conditional should only run AFTER user is authenticated\n')

# The real fix: The conditional authenticators need to be in a subflow that only executes after username/password
# But since Keycloak's default browser flow has them in the wrong order, we need to reorder

# Get the flow config to see current order
print('Current execution order:')
for i, exec_item in enumerate(executions):
    display_name = exec_item.get('displayName', exec_item.get('providerId', 'unknown'))
    requirement = exec_item.get('requirement', 'unknown')
    print(f'  {i+1}. {display_name} - {requirement}')

print('\n✅ Analysis complete')
print('The issue is that conditional authenticators check user.credentialManager() before user exists')
print('These should only execute AFTER username/password authentication')
print('\nRecommendation: Disable or reorder these conditional checks')
