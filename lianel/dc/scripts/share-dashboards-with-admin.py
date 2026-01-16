#!/usr/bin/env python3
"""Share all provisioned dashboards with admin user via Grafana API"""

import requests
import json
import os
import sys
import urllib3
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

# Get environment variables
GRAFANA_URL = os.getenv('GRAFANA_URL', 'http://localhost:3000')
ADMIN_USER = os.getenv('GRAFANA_ADMIN_USER', 'admin')
ADMIN_PASSWORD = os.getenv('GRAFANA_ADMIN_PASSWORD', 'admin')

print('=== Sharing Dashboards with Admin User ===\n')

# Step 1: Login as admin to get session
login_url = f'{GRAFANA_URL}/api/login'
login_data = {
    'user': ADMIN_USER,
    'password': ADMIN_PASSWORD
}

print(f'Logging in as {ADMIN_USER}...')
login_resp = requests.post(login_url, json=login_data, timeout=10, verify=False)

if login_resp.status_code != 200:
    print(f'❌ Failed to login: {login_resp.status_code} - {login_resp.text}')
    sys.exit(1)

# Get session cookie
session_cookie = login_resp.cookies.get('grafana_session')
if not session_cookie:
    print('❌ No session cookie received')
    sys.exit(1)

headers = {
    'Cookie': f'grafana_session={session_cookie}',
    'Content-Type': 'application/json'
}

print('✅ Logged in successfully\n')

# Step 2: Get current user info
user_url = f'{GRAFANA_URL}/api/user'
user_resp = requests.get(user_url, headers=headers, timeout=10, verify=False)
if user_resp.status_code == 200:
    user_info = user_resp.json()
    user_id = user_info.get('id')
    user_login = user_info.get('login')
    print(f'Current user: {user_login} (ID: {user_id})\n')

# Step 3: Get all dashboards
dashboards_url = f'{GRAFANA_URL}/api/search?type=dash-db'
print('Fetching dashboards...')
dashboards_resp = requests.get(dashboards_url, headers=headers, timeout=10, verify=False)

if dashboards_resp.status_code != 200:
    print(f'❌ Failed to fetch dashboards: {dashboards_resp.status_code} - {dashboards_resp.text}')
    sys.exit(1)

dashboards = dashboards_resp.json()
print(f'Found {len(dashboards)} dashboards\n')

if len(dashboards) == 0:
    print('⚠️  No dashboards found. They should be provisioned automatically.')
    print('   Waiting 5 seconds for provisioning to complete...')
    import time
    time.sleep(5)
    
    # Try again
    dashboards_resp = requests.get(dashboards_url, headers=headers, timeout=10, verify=False)
    if dashboards_resp.status_code == 200:
        dashboards = dashboards_resp.json()
        print(f'Found {len(dashboards)} dashboards after wait\n')

# Step 4: Share each dashboard with admin user
shared_count = 0
for dashboard in dashboards:
    dashboard_uid = dashboard.get('uid')
    dashboard_title = dashboard.get('title')
    dashboard_id = dashboard.get('id')
    
    if not dashboard_uid:
        print(f'  ⚠️  {dashboard_title} - No UID, skipping')
        continue
    
    print(f'Processing: {dashboard_title} (UID: {dashboard_uid})...')
    
    # Get dashboard full definition
    dashboard_url = f'{GRAFANA_URL}/api/dashboards/uid/{dashboard_uid}'
    dashboard_resp = requests.get(dashboard_url, headers=headers, timeout=10, verify=False)
    
    if dashboard_resp.status_code != 200:
        print(f'  ⚠️  Failed to fetch dashboard: {dashboard_resp.status_code}')
        continue
    
    dashboard_data = dashboard_resp.json().get('dashboard', {})
    
    # Ensure dashboard is in General folder (folderId: 0) and editable
    dashboard_data['editable'] = True
    dashboard_data['folderId'] = 0
    
    # Save dashboard (this makes it accessible)
    save_data = {
        'dashboard': dashboard_data,
        'overwrite': True
    }
    
    save_url = f'{GRAFANA_URL}/api/dashboards/db'
    save_resp = requests.post(save_url, headers=headers, json=save_data, timeout=10, verify=False)
    
    if save_resp.status_code == 200:
        print(f'  ✅ Shared successfully')
        shared_count += 1
    else:
        print(f'  ⚠️  Failed to share: {save_resp.status_code} - {save_resp.text[:100]}')

print(f'\n✅ {shared_count}/{len(dashboards)} dashboards shared with admin user')
print('\n✅ Complete! Dashboards should now be visible in Grafana.')
