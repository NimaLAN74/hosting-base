#!/usr/bin/env python3
"""Check Keycloak organizations configuration"""

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

print('=== Checking Organizations Configuration ===\n')

# Check if organizations are enabled
realm_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}'
realm_resp = requests.get(realm_url, headers=headers, timeout=10, verify=False)
realm_config = realm_resp.json()

orgs_enabled = realm_config.get('organizationsEnabled', False)
print(f'Organizations Enabled: {orgs_enabled}\n')

if orgs_enabled:
    # Check if there are any organizations
    orgs_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/organizations'
    try:
        orgs_resp = requests.get(orgs_url, headers=headers, timeout=10, verify=False)
        if orgs_resp.status_code == 200:
            orgs = orgs_resp.json()
            if 'organizations' in orgs:
                org_list = orgs['organizations']
                print(f'Number of Organizations: {len(org_list)}')
                if org_list:
                    for org in org_list:
                        print(f'  - {org.get("name")} (ID: {org.get("id")})')
                else:
                    print('  ⚠️  No organizations configured but organizations are enabled')
                    print('     This may be causing authentication flow issues')
            else:
                print('  ⚠️  Organizations API returned unexpected format')
        else:
            print(f'  ⚠️  Failed to fetch organizations: {orgs_resp.status_code}')
    except Exception as e:
        print(f'  ⚠️  Error checking organizations: {e}')
else:
    print('✅ Organizations are disabled')
    print('   However, organization authenticators are still in the flow')
    print('   This may be causing the authentication error')

print('\n=== Recommendation ===')
if orgs_enabled and not orgs_list:
    print('Disable organizations or configure at least one organization')
else:
    print('If organizations are not needed, disable them in realm settings')
