#!/usr/bin/env python3
"""
Create Grafana Client in Keycloak
This script creates the grafana-client needed for Grafana OAuth authentication
"""

import sys
import json
import requests
import os

KEYCLOAK_URL = os.getenv('KEYCLOAK_URL', 'http://localhost:8080')
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
        response = requests.post(url, data=data, timeout=10)
        response.raise_for_status()
        token_data = response.json()
        return token_data.get('access_token')
    except Exception as e:
        print(f"❌ Failed to get admin token: {e}")
        if hasattr(e, 'response') and e.response is not None:
            print("   Response: (redacted)")
        return None

def check_client_exists(token, client_id):
    """Check if client already exists"""
    url = f"{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients"
    headers = {'Authorization': f'Bearer {token}'}
    params = {'clientId': client_id}
    
    try:
        response = requests.get(url, headers=headers, params=params, timeout=10)
        response.raise_for_status()
        clients = response.json()
        return clients[0]['id'] if clients else None
    except Exception as e:
        print(f"⚠️  Error checking client: {e}")
        return None

def create_client(token):
    """Create or update Grafana client"""
    client_id = 'grafana-client'
    
    # Check if client exists
    existing_client_id = check_client_exists(token, client_id)
    
    client_data = {
        "clientId": client_id,
        "enabled": True,
        "publicClient": False,
        "standardFlowEnabled": True,
        "implicitFlowEnabled": False,
        "directAccessGrantsEnabled": False,
        "serviceAccountsEnabled": False,
        "redirectUris": [
            "https://monitoring.lianel.se/login/generic_oauth"
        ],
        "webOrigins": [
            "https://monitoring.lianel.se"
        ],
        "protocol": "openid-connect",
        "attributes": {
            "pkce.code.challenge.method": "S256"
        },
        "fullScopeAllowed": True
    }
    
    if existing_client_id:
        print(f"⚠️  Client {client_id} already exists (ID: {existing_client_id})")
        print("   Updating client configuration...")
        
        # Update existing client
        url = f"{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients/{existing_client_id}"
        headers = {'Authorization': f'Bearer {token}', 'Content-Type': 'application/json'}
        
        try:
            response = requests.put(url, headers=headers, json=client_data, timeout=10)
            response.raise_for_status()
            print(f"✅ Client {client_id} updated successfully")
            client_uuid = existing_client_id
        except Exception as e:
            print(f"❌ Failed to update client: {e}")
            if hasattr(e, 'response') and e.response is not None:
                print(f"   Response: {e.response.text}")
            return None
    else:
        print(f"Creating client {client_id}...")
        
        # Create new client
        url = f"{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients"
        headers = {'Authorization': f'Bearer {token}', 'Content-Type': 'application/json'}
        
        try:
            response = requests.post(url, headers=headers, json=client_data, timeout=10)
            response.raise_for_status()
            
            # Get the created client ID
            created_client_id = check_client_exists(token, client_id)
            if created_client_id:
                print(f"✅ Client {client_id} created successfully (ID: {created_client_id})")
                client_uuid = created_client_id
            else:
                print(f"⚠️  Client may have been created but couldn't retrieve ID")
                return None
        except Exception as e:
            print(f"❌ Failed to create client: {e}")
            if hasattr(e, 'response') and e.response is not None:
                try:
                    error_data = e.response.json()
                    error_msg = error_data.get('errorMessage', error_data.get('error', 'Unknown error'))
                    print(f"   Error: {error_msg}")
                except:
                    print(f"   Response: {e.response.text}")
            return None
    
    # Get client secret
    print("\n3. Getting client secret...")
    secret_url = f"{KEYCLOAK_URL}/admin/realms/{REALM_NAME}/clients/{client_uuid}/client-secret"
    secret_headers = {'Authorization': f'Bearer {token}'}
    
    try:
        secret_response = requests.get(secret_url, headers=secret_headers, timeout=10)
        secret_response.raise_for_status()
        secret_data = secret_response.json()
        client_secret = secret_data.get('value')
        
        if client_secret:
            print(f"✅ Client Secret: {client_secret}")
            print("\n⚠️  IMPORTANT: Update GRAFANA_OAUTH_CLIENT_SECRET in your .env file:")
            print(f"   GRAFANA_OAUTH_CLIENT_SECRET={client_secret}")
        else:
            print("⚠️  Could not retrieve client secret")
    except Exception as e:
        print(f"⚠️  Could not retrieve client secret: {e}")
    
    return client_uuid

def main():
    print("=== Creating Grafana Client in Keycloak ===")
    print(f"Keycloak URL: {KEYCLOAK_URL}")
    print(f"Realm: {REALM_NAME}\n")
    
    # Get admin token
    print("1. Getting admin token...")
    token = get_admin_token()
    if not token:
        sys.exit(1)
    print("✅ Admin token obtained\n")
    
    # Create client
    print("2. Creating/updating grafana-client...")
    client_uuid = create_client(token)
    
    if client_uuid:
        print("\n=== Grafana Client Configuration Complete ===")
        print("\nNext steps:")
        print("1. Update .env file with GRAFANA_OAUTH_CLIENT_SECRET if needed")
        print("2. Restart Grafana: docker restart grafana")
        print("3. Access Grafana at: https://monitoring.lianel.se")
    else:
        print("\n❌ Failed to create/update Grafana client")
        sys.exit(1)

if __name__ == '__main__':
    main()
