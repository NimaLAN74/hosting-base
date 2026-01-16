#!/usr/bin/env python3
"""Restore frontend-client to original working configuration"""

import requests
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

# Get frontend-client
clients_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients'
params = {'clientId': 'frontend-client'}
clients_resp = requests.get(clients_url, headers=headers, params=params, timeout=10, verify=False)
client_id = clients_resp.json()[0]['id']

client_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients/{client_id}'
client_resp = requests.get(client_url, headers=headers, timeout=10, verify=False)
client_config = client_resp.json()

print('=== Restoring Frontend Client to Original Working Configuration ===\n')

# Original working config from RESOLUTION-SUMMARY.md
original_config = {
    'clientId': 'frontend-client',
    'enabled': True,
    'publicClient': True,
    'standardFlowEnabled': True,
    'implicitFlowEnabled': False,
    'directAccessGrantsEnabled': True,
    'serviceAccountsEnabled': False,
    'redirectUris': [
        'https://lianel.se',
        'https://lianel.se/',
        'https://www.lianel.se',  # Keep www variants too
        'https://www.lianel.se/'
    ],
    'webOrigins': ['*'],  # Original was ['*']
    'frontchannelLogout': True,
    'protocol': 'openid-connect',
    'fullScopeAllowed': True,
    'attributes': {
        'post.logout.redirect.uris': 'https://lianel.se\nhttps://lianel.se/',
        # Remove all PKCE attributes - original config didn't have them
    }
}

# Update only the changed fields, keep rest as is
client_config['redirectUris'] = original_config['redirectUris']
client_config['webOrigins'] = original_config['webOrigins']
client_config['frontchannelLogout'] = original_config['frontchannelLogout']
client_config['attributes'] = original_config['attributes']

# Update client
update_resp = requests.put(client_url, headers=headers, json=client_config, timeout=10, verify=False)
if update_resp.status_code == 204:
    print('✅ Frontend client restored to original working configuration')
    print(f'   Redirect URIs: {client_config["redirectUris"]}')
    print(f'   Web Origins: {client_config["webOrigins"]}')
    print(f'   PKCE Attributes: Removed')
    exit(0)
else:
    print(f'❌ Failed: {update_resp.status_code} - {update_resp.text}')
    exit(1)
