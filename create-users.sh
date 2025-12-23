#!/bin/bash
set -e

TOKEN=$(curl -sk -X POST "https://auth.lianel.se/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=admin" \
  -d "password=D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | jq -r ".access_token")

echo "=== Creating test user ==="
curl -sk -X POST "https://auth.lianel.se/admin/realms/lianel/users" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","email":"test@lianel.se","enabled":true,"emailVerified":true,"credentials":[{"type":"password","value":"Test123!","temporary":false}]}' && echo "✓ testuser created"

echo
echo "=== Creating admin user ==="
curl -sk -X POST "https://auth.lianel.se/admin/realms/lianel/users" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","email":"admin@lianel.se","enabled":true,"emailVerified":true,"credentials":[{"type":"password","value":"D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA","temporary":false}]}' && echo "✓ admin user created"

echo
echo "Done!"
