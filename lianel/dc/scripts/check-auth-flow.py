#!/usr/bin/env python3
"""Check Keycloak authentication flow configuration"""

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

# Get realm config
realm_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}'
realm_resp = requests.get(realm_url, headers=headers, timeout=10, verify=False)
realm_config = realm_resp.json()

print('=== Realm Authentication Flow Configuration ===\n')
print(f'Browser Flow: {realm_config.get("browserFlow")}')
print(f'Direct Grant Flow: {realm_config.get("directGrantFlow")}\n')

# Get browser flow executions
browser_flow = realm_config.get('browserFlow', 'browser')
executions_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/{browser_flow}/executions'
executions_resp = requests.get(executions_url, headers=headers, timeout=10, verify=False)
executions = executions_resp.json()

print(f'=== Browser Flow Executions ({browser_flow}) ===\n')
for i, exec_item in enumerate(executions):
    display_name = exec_item.get('displayName', exec_item.get('providerId', 'unknown'))
    provider_id = exec_item.get('providerId', 'unknown')
    requirement = exec_item.get('requirement', 'unknown')
    configurable = exec_item.get('configurable', False)
    
    print(f'{i+1}. {display_name}')
    print(f'   Provider: {provider_id}')
    print(f'   Requirement: {requirement}')
    print(f'   Configurable: {configurable}')
    
    # Check for sub-executions (conditionals)
    if 'authenticationExecutions' in exec_item:
        for sub_exec in exec_item['authenticationExecutions']:
            sub_display = sub_exec.get('displayName', sub_exec.get('providerId', 'unknown'))
            sub_provider = sub_exec.get('providerId', 'unknown')
            sub_req = sub_exec.get('requirement', 'unknown')
            print(f'   └─ {sub_display} (Provider: {sub_provider}, Requirement: {sub_req})')
    print()
