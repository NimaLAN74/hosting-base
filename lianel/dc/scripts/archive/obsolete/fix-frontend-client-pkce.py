#!/usr/bin/env python3
"""
Fix Frontend Client PKCE Configuration
This script ensures the frontend-client is properly configured for PKCE
"""

import requests
import json
import os
import urllib3
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

KEYCLOAK_URL = os.getenv('KEYCLOAK_URL', 'https://auth.lianel.se')
REALM_NAME = os.getenv('REALM_NAME', 'lianel')
ADMIN_USER = os.getenv('KEYCLOAK_ADMIN_USER', 'admin')
ADMIN_PASSWORD = os.getenv('KEYCLOAK_ADMIN_PASSWORD', 'D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA')

def get_admin_token():
    """Get admin token from Keycloak"""
    url = f"{KEYCLOAK_URL}/realms/master/protocol/openid-connect/token"
    data = {
        'username': ADMIN_USER,
        'password': ADMIN_PASSWORD,
        'grant_type': 'password',
        'client_id': 'admin-cli'
    }
    
    try:
        response = requests.post(url, data=data, timeout=10, verify=False)
        response.raise_for_status()
        token_data = response.json()
        return token_data.get('access_token')
    except Exception as e:
        print(f"❌ Failed to get admin token: {e}")
        return None

def main():
    print("=== Fixing Frontend Client PKCE Configuration ===\n")
    
    # Get admin token
    print("1. Getting admin token...")
    token = get_admin_token()
    if not token:
        return 1
    print("✅ Admin token obtained\n")
    
    headers = {'Authorization': f'Bearer {token}'}
    
    # Get frontend-client
    print("2. Getting frontend-client configuration...")
    clients_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients'
    params = {'clientId': 'frontend-client'}
    
    try:
        response = requests.get(clients_url, headers=headers, params=params, timeout=10, verify=False)
        response.raise_for_status()
        clients = response.json()
        
        if not clients:
            print("❌ frontend-client not found")
            return 1
        
        client_id = clients[0]['id']
        print(f"✅ Found frontend-client (ID: {client_id})\n")
    except Exception as e:
        print(f"❌ Failed to get client: {e}")
        return 1
    
    # Get full client configuration
    client_url = f'{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients/{client_id}'
    
    try:
        response = requests.get(client_url, headers=headers, timeout=10, verify=False)
        response.raise_for_status()
        client_config = response.json()
    except Exception as e:
        print(f"❌ Failed to get client config: {e}")
        return 1
    
    print("3. Current configuration:")
    print(f"   PKCE Code Challenge Method: {client_config.get('attributes', {}).get('pkce.code.challenge.method', 'NOT SET')}")
    print(f"   Standard Flow: {client_config.get('standardFlowEnabled')}")
    print(f"   Public Client: {client_config.get('publicClient')}")
    print(f"   Direct Access Grants: {client_config.get('directAccessGrantsEnabled')}\n")
    
    # Ensure attributes exist
    if 'attributes' not in client_config:
        client_config['attributes'] = {}
    
    # Set PKCE configuration
    client_config['attributes']['pkce.code.challenge.method'] = 'S256'
    
    # Ensure all required settings
    client_config['standardFlowEnabled'] = True
    client_config['publicClient'] = True
    client_config['directAccessGrantsEnabled'] = True
    client_config['enabled'] = True
    
    # Ensure redirect URIs include exact matches
    redirect_uris = list(set(client_config.get('redirectUris', [])))
    required_uris = [
        'https://www.lianel.se/',
        'https://www.lianel.se/*',
        'https://www.lianel.se',
        'https://lianel.se/',
        'https://lianel.se/*',
        'http://localhost:3000/*'
    ]
    
    for uri in required_uris:
        if uri not in redirect_uris:
            redirect_uris.append(uri)
    
    client_config['redirectUris'] = redirect_uris
    
    # Ensure web origins
    web_origins = list(set(client_config.get('webOrigins', [])))
    required_origins = [
        'https://www.lianel.se',
        'https://lianel.se',
        '+'
    ]
    
    for origin in required_origins:
        if origin not in web_origins:
            web_origins.append(origin)
    
    client_config['webOrigins'] = web_origins
    
    print("4. Updating client configuration...")
    
    try:
        response = requests.put(client_url, headers=headers, json=client_config, timeout=10, verify=False)
        response.raise_for_status()
        
        if response.status_code == 204:
            print("✅ Client updated successfully!\n")
            print("5. Updated configuration:")
            print(f"   PKCE Code Challenge Method: {client_config['attributes']['pkce.code.challenge.method']}")
            print(f"   Redirect URIs: {len(redirect_uris)} configured")
            print(f"   Web Origins: {len(web_origins)} configured")
            print("\n✅ Frontend client PKCE configuration fixed!")
            return 0
        else:
            print(f"❌ Unexpected status code: {response.status_code}")
            print(f"   Response: {response.text}")
            return 1
    except Exception as e:
        print(f"❌ Failed to update client: {e}")
        if hasattr(e, 'response') and e.response is not None:
            try:
                error_data = e.response.json()
                print(f"   Error: {error_data.get('errorMessage', error_data.get('error', 'Unknown error'))}")
            except:
                print(f"   Response: {e.response.text}")
        return 1

if __name__ == '__main__':
    exit(main())
