#!/usr/bin/env python3
"""Make PKCE optional (not required) for frontend-client"""

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

# Get full config
client_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients/{client_id}'
client_resp = requests.get(client_url, headers=headers, timeout=10, verify=False)
client_config = client_resp.json()

# Make PKCE optional (not required)
if 'attributes' not in client_config:
    client_config['attributes'] = {}

# Remove PKCE requirement - allow clients to use PKCE if they want, but don't require it
if 'pkce.code.challenge.required' in client_config['attributes']:
    del client_config['attributes']['pkce.code.challenge.required']

# Keep the method set to S256 in case PKCE is used
client_config['attributes']['pkce.code.challenge.method'] = 'S256'

# Update client
update_resp = requests.put(client_url, headers=headers, json=client_config, timeout=10, verify=False)
if update_resp.status_code == 204:
    print('✅ Made PKCE optional for frontend-client')
    print('   PKCE is now supported but not required')
    print('   Clients can use PKCE if they want, but non-PKCE requests will also work')
else:
    print(f'❌ Failed: {update_resp.status_code} - {update_resp.text}')
