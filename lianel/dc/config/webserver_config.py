"""
Airflow Webserver Configuration for Keycloak OAuth SSO

This configuration enables Flask AppBuilder OAuth authentication with Keycloak.
Place this file in /opt/airflow/config/webserver_config.py
"""
import os
from flask_appbuilder.security.manager import AUTH_OAUTH

# Enable OAuth authentication
AUTH_TYPE = AUTH_OAUTH
AUTH_USER_REGISTRATION = True
AUTH_USER_REGISTRATION_ROLE = 'Viewer'
WTF_CSRF_ENABLED = False

# OAuth providers configuration
OAUTH_PROVIDERS = [
    {
        'name': 'keycloak',
        'icon': 'fa-key',
        'token_key': 'access_token',
        'remote_app': {
            'client_id': 'airflow',
            'client_secret': os.getenv('AIRFLOW_OAUTH_CLIENT_SECRET', ''),
            'api_base_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect',
            'client_kwargs': {
                'scope': 'openid profile email'
            },
            'access_token_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/token',
            'authorize_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth',
            'request_token_url': None,
        }
    }
]

# User info endpoint for retrieving user details
OAUTH_USER_INFO_ENDPOINT = 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/userinfo'

# Ensure correct callback is used: /oauth-authorized/keycloak
OAUTH_REDIRECT_URI = 'https://airflow.lianel.se/oauth-authorized/keycloak'

# Role mapping (optional - customize based on your Keycloak roles)
# OAUTH_ROLES_MAPPING = {
#     'admin': ['Admin'],
#     'user': ['User'],
#     'viewer': ['Viewer']
# }

