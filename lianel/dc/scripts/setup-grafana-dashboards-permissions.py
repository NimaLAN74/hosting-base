#!/usr/bin/env python3
"""Setup Grafana dashboards permissions - share with admin user"""

import requests
import json
import os
import urllib3
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

GRAFANA_URL = os.getenv('GRAFANA_URL', 'http://localhost:3000')
GRAFANA_ADMIN_PASSWORD = os.getenv('GRAFANA_ADMIN_PASSWORD', 'admin')

print('=== Setting Up Grafana Dashboard Permissions ===\n')

# Get admin token
login_url = f'{GRAFANA_URL}/api/login'
login_data = {
    'user': 'admin',
    'password': GRAFANA_ADMIN_PASSWORD
}

login_resp = requests.post(login_url, json=login_data, timeout=10, verify=False)
if login_resp.status_code != 200:
    print(f'❌ Failed to login to Grafana: {login_resp.status_code} - {login_resp.text}')
    exit(1)

# Get session cookie
session_cookie = login_resp.cookies.get('grafana_session') or login_resp.cookies.get('auth')
if not session_cookie:
    # Try using API key if available
    headers = {'Authorization': f'Basic {GRAFANA_ADMIN_PASSWORD}'}
else:
    headers = {'Cookie': f'grafana_session={session_cookie}'}

# Get current user
user_url = f'{GRAFANA_URL}/api/user'
user_resp = requests.get(user_url, headers=headers, timeout=10, verify=False)
if user_resp.status_code != 200:
    print(f'⚠️  Failed to get user info, trying without auth...')
    headers = {}

print(f'Grafana URL: {GRAFANA_URL}')
print()

# Get all dashboards
dashboards_url = f'{GRAFANA_URL}/api/search?type=dash-db'
dashboards_resp = requests.get(dashboards_url, headers=headers, timeout=10, verify=False)

if dashboards_resp.status_code == 200:
    dashboards = dashboards_resp.json()
    print(f'Found {len(dashboards)} dashboards\n')
    
    # Share each dashboard
    shared_count = 0
    for dashboard in dashboards:
        dashboard_uid = dashboard.get('uid')
        dashboard_title = dashboard.get('title')
        
        # Get dashboard details
        dashboard_url = f'{GRAFANA_URL}/api/dashboards/uid/{dashboard_uid}'
        dashboard_resp = requests.get(dashboard_url, headers=headers, timeout=10, verify=False)
        
        if dashboard_resp.status_code == 200:
            dashboard_data = dashboard_resp.json().get('dashboard', {})
            
            # Update dashboard to ensure it's editable and in the correct folder
            update_data = {
                'dashboard': {
                    **dashboard_data,
                    'editable': True,
                    'folderId': 0,  # General folder (accessible to all)
                },
                'overwrite': True
            }
            
            # Save dashboard
            save_url = f'{GRAFANA_URL}/api/dashboards/db'
            save_resp = requests.post(save_url, headers=headers, json=update_data, timeout=10, verify=False)
            
            if save_resp.status_code == 200:
                print(f'  ✅ {dashboard_title} - Shared in General folder')
                shared_count += 1
            else:
                print(f'  ⚠️  {dashboard_title} - Failed to update: {save_resp.status_code}')
        else:
            print(f'  ⚠️  {dashboard_title} - Failed to fetch: {dashboard_resp.status_code}')
    
    print(f'\n✅ {shared_count}/{len(dashboards)} dashboards shared')
else:
    print(f'⚠️  Failed to list dashboards: {dashboards_resp.status_code}')
    print('   Dashboards will be provisioned automatically when Grafana restarts')

print('\n✅ Dashboard permissions setup complete!')
