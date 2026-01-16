#!/usr/bin/env python3
"""Check and fix Grafana client configuration for OAuth"""

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

print('=== Checking Grafana Client Configuration ===\n')

# Get grafana-client
clients_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients'
params = {'clientId': 'grafana-client'}
clients_resp = requests.get(clients_url, headers=headers, params=params, timeout=10, verify=False)

if not clients_resp.json():
    print('❌ Grafana client not found!')
    exit(1)

client_id = clients_resp.json()[0]['id']
client_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients/{client_id}'
client_resp = requests.get(client_url, headers=headers, timeout=10, verify=False)
client_config = client_resp.json()

print('Current Configuration:')
print(f'  Client ID: {client_config.get("clientId")}')
print(f'  Enabled: {client_config.get("enabled")}')
print(f'  Public Client: {client_config.get("publicClient")}')
print(f'  Standard Flow: {client_config.get("standardFlowEnabled")}')
print(f'  Direct Access Grants: {client_config.get("directAccessGrantsEnabled")}')
print(f'  Redirect URIs: {client_config.get("redirectUris")}')
print(f'  Web Origins: {client_config.get("webOrigins")}')

# Get client secret
secret_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients/{client_id}/client-secret'
secret_resp = requests.get(secret_url, headers=headers, timeout=10, verify=False)
secret_data = secret_resp.json()
secret_value = secret_data.get('value')

print(f'\nClient Secret: {secret_value}')

# Check attributes
attrs = client_config.get('attributes', {})
print(f'\nAttributes:')
print(f'  PKCE Code Challenge Method: {attrs.get("pkce.code.challenge.method", "NOT SET")}')

# The issue: Grafana is using PKCE but client is confidential (needs secret)
# For PKCE with confidential clients, we need to ensure PKCE is optional or properly configured
# But actually, the error "Invalid client credentials" suggests the secret is missing or wrong

print('\n=== Issue Analysis ===')
print('Error: "unauthorized_client" "Invalid client or Invalid client credentials"')
print('This means Grafana is trying to exchange code for token but Keycloak rejects it.')
print()

if client_config.get('publicClient'):
    print('⚠️  Client is PUBLIC - but Grafana config might be sending secret')
    print('   Solution: Either make client confidential and set client_secret in Grafana,')
    print('             OR make client public and remove client_secret from Grafana config')
elif not secret_value:
    print('❌ Client is CONFIDENTIAL but has no secret!')
    print('   Solution: Generate client secret or make client public')
else:
    print('✅ Client is CONFIDENTIAL with secret')
    print('   Issue: Grafana may not have client_secret configured in grafana.ini')
    print('   Solution: Add client_secret to Grafana config')

# Check if PKCE is properly configured
if attrs.get('pkce.code.challenge.method') == 'S256':
    print('\n⚠️  PKCE is REQUIRED on client')
    print('   But Grafana is using use_pkce = true, which should work')
    print('   However, for confidential clients with PKCE, secret may still be needed')

print('\n=== Recommendation ===')
print('For confidential client with PKCE:')
print('1. Ensure client_secret is set in Grafana grafana.ini')
print('2. OR make client public (if PKCE is sufficient)')
print('3. OR remove PKCE requirement from client and use secret only')
