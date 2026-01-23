#!/usr/bin/env python3
"""Fix Identity Provider Redirector - disable if no providers or misconfigured"""

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

print('=== Checking Identity Provider Configuration ===\n')

# Check identity providers
idps_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/identity-provider/instances'
idps_resp = requests.get(idps_url, headers=headers, timeout=10, verify=False)
idps = idps_resp.json()

print('Identity Providers:')
if idps:
    for idp in idps:
        alias = idp.get('alias', 'unknown')
        provider_id = idp.get('providerId', 'unknown')
        enabled = idp.get('enabled', False)
        print(f'  - {alias} (Type: {provider_id}, Enabled: {enabled})')
    print(f'\nTotal: {len(idps)} provider(s) configured')
else:
    print('  No identity providers configured')

# Get browser flow executions
executions_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/authentication/flows/browser/executions'
executions_resp = requests.get(executions_url, headers=headers, timeout=10, verify=False)
executions = executions_resp.json()

print('\n=== Checking Identity Provider Redirector Authenticator ===\n')

# Find Identity Provider Redirector
idp_redirector = None
for exec_item in executions:
    provider_id = exec_item.get('providerId', '')
    display_name = exec_item.get('displayName', '')
    
    if provider_id == 'identity-provider-redirector':
        idp_redirector = exec_item
        break

if idp_redirector:
    exec_id = idp_redirector.get('id')
    requirement = idp_redirector.get('requirement', '')
    configurable = idp_redirector.get('configurable', False)
    
    print(f'Found: Identity Provider Redirector (ID: {exec_id})')
    print(f'  Current Requirement: {requirement}')
    print(f'  Configurable: {configurable}\n')
    
    # Decision: Disable if no providers or if misconfigured
    if not idps or len([idp for idp in idps if idp.get('enabled')]) == 0:
        print('⚠️  No enabled identity providers found')
        print('   Disabling Identity Provider Redirector to prevent authentication errors\n')
        
        if configurable and requirement != 'DISABLED':
            update_data = {
                'id': exec_id,
                'requirement': 'DISABLED'
            }
            
            update_resp = requests.put(executions_url, headers=headers, json=update_data, timeout=10, verify=False)
            if update_resp.status_code == 204:
                print('✅ Identity Provider Redirector disabled successfully')
                print('   Login should now work without identity provider errors')
                exit(0)
            else:
                print(f'❌ Failed to disable: {update_resp.status_code} - {update_resp.text}')
                exit(1)
        else:
            print(f'⚠️  Cannot disable - Configurable: {configurable}, Requirement: {requirement}')
            print('   This authenticator may be built-in and non-configurable')
            if requirement == 'ALTERNATIVE':
                print('   ⚠️  As ALTERNATIVE, it should not block login, but may be causing issues')
    else:
        enabled_idps = [idp for idp in idps if idp.get('enabled')]
        print(f'✅ {len(enabled_idps)} enabled identity provider(s) found')
        print('   Identity Provider Redirector should work correctly')
        print('   The error may be caused by misconfigured provider(s)')
        print('\nChecking provider configurations...')
        for idp in enabled_idps:
            alias = idp.get('alias')
            config = idp.get('config', {})
            print(f'\n  Provider: {alias}')
            print(f'    Config keys: {list(config.keys())[:5]}...')
            # Check for common misconfigurations
            if 'clientId' in config and not config.get('clientId'):
                print(f'    ⚠️  clientId is empty')
            if 'clientSecret' in config and not config.get('clientSecret'):
                print(f'    ⚠️  clientSecret is empty')
else:
    print('⚠️  Identity Provider Redirector authenticator not found')
    print('   This is unusual - it should be in the browser flow')
    print('   The error may be coming from a different authenticator')

print('\n✅ Analysis complete')
