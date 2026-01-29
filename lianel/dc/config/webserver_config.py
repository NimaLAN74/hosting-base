"""
Airflow Webserver Configuration for Keycloak OAuth SSO

This configuration enables Flask AppBuilder OAuth authentication with Keycloak.
Place this file in /opt/airflow/config/webserver_config.py
"""
import os
from flask_appbuilder.security.manager import AUTH_OAUTH

# Force HTTPS for url_for(..., _external=True) so redirect_uri sent to Keycloak is https (avoids 400 when behind nginx)
PREFERRED_URL_SCHEME = "https"

# Enable OAuth authentication
AUTH_TYPE = AUTH_OAUTH
AUTH_USER_REGISTRATION = True
# User role allows triggering DAGs (can_edit on DAG, can create on DAG Run); Viewer is read-only and returns 403 on trigger.
# Existing users already registered as Viewer must be updated in Airflow UI: Admin → List Users → edit → Role = User.
AUTH_USER_REGISTRATION_ROLE = 'User'
WTF_CSRF_ENABLED = False

# OAuth providers configuration
# redirect_uri must be https and match Keycloak client; FAB/authlib may build from request (http behind proxy).
OAUTH_PROVIDERS = [
    {
        'name': 'keycloak',
        'icon': 'fa-key',
        'token_key': 'access_token',
        'remote_app': {
            'client_id': 'airflow',
            'client_secret': os.getenv('AIRFLOW_OAUTH_CLIENT_SECRET', ''),
            'api_base_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect',
            # Authlib needs jwks_uri for id_token validation; fetch from OIDC discovery (fixes "Missing jwks_uri in metadata")
            'server_metadata_url': 'https://auth.lianel.se/realms/lianel/.well-known/openid-configuration',
            'client_kwargs': {
                'scope': 'openid profile email',
                # Keycloak accepts both; client_secret_post avoids proxy/header issues with Basic auth
                'token_endpoint_auth_method': 'client_secret_post',
            },
            'access_token_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/token',
            'authorize_url': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/auth',
            'request_token_url': None,
            # Force https callback so Keycloak accepts (avoids 400 when FAB builds redirect_uri from request as http)
            'redirect_uri': 'https://airflow.lianel.se/auth/oauth-authorized/keycloak',
            # authlib/FAB: explicit userinfo for Keycloak (OIDC userinfo returns preferred_username, email, etc.)
            'userinfo_endpoint': 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/userinfo',
        }
    }
]

# User info endpoint for retrieving user details
OAUTH_USER_INFO_ENDPOINT = 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/userinfo'

# FAB auth blueprint uses /auth/ prefix; callback is /auth/oauth-authorized/keycloak (see ROOT-CAUSE-LOGIN-AIRFLOW-KEYCLOAK.md).
OAUTH_REDIRECT_URI = 'https://airflow.lianel.se/auth/oauth-authorized/keycloak'


# Force HTTPS for url_for(..., _external=True): set wsgi.url_scheme before FAB builds redirect_uri.
# Airflow loads this file with app context (see airflow.apache.org FAB "Configuring Flask Application").
# Register before_request on current_app so it always runs; FLASK_APP_MUTATOR is not guaranteed to be called.
def _force_https_scheme():
    from flask import request
    # Force HTTPS so url_for(..., _external=True) yields https (Keycloak redirect_uri).
    # Apply when proxy sent X-Forwarded-Proto or when handling /auth/ (OAuth).
    if request.path.startswith("/auth/") or request.environ.get("HTTP_X_FORWARDED_PROTO") == "https":
        request.environ["wsgi.url_scheme"] = "https"
        request.environ["HTTP_X_FORWARDED_PROTO"] = "https"


def _register_https_scheme_fix(app):
    """Register before_request to force HTTPS scheme for /auth/ (so redirect_uri is https)."""
    app.before_request(_force_https_scheme)


# Register when config is loaded with app context (Airflow FAB pushes app context when loading this file).
# ProxyFix must run before any view so url_for(..., _external=True) sees https from X-Forwarded-Proto.
try:
    from flask import current_app
    try:
        from werkzeug.middleware.proxy_fix import ProxyFix
        current_app.wsgi_app = ProxyFix(current_app.wsgi_app, x_proto=1, x_host=1)
    except Exception:
        pass
    _register_https_scheme_fix(current_app)
except RuntimeError:
    # No app context yet (e.g. config loaded before app created); FAB may call FLASK_APP_MUTATOR later.
    pass


def FLASK_APP_MUTATOR(app):
    """Apply ProxyFix so X-Forwarded-Proto is used (redirect_uri=https). Then register before_request as backup."""
    try:
        from werkzeug.middleware.proxy_fix import ProxyFix
        # x_proto=1: trust X-Forwarded-Proto from one proxy (nginx)
        app.wsgi_app = ProxyFix(app.wsgi_app, x_proto=1, x_host=1)
    except Exception:
        pass
    _register_https_scheme_fix(app)


# Role mapping (optional - customize based on your Keycloak roles)
# OAUTH_ROLES_MAPPING = {
#     'admin': ['Admin'],
#     'user': ['User'],
#     'viewer': ['Viewer']
# }

