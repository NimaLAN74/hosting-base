# Admin User Management – Setup & Deployment Guide

## Quick Start: Create Your First Admin User

### 1. Start the Keycloak Infrastructure

```bash
cd /root/lianel/dc
docker-compose -f docker-compose.infra.yaml up -d
```

Verify Keycloak is running:
```bash
docker-compose -f docker-compose.infra.yaml logs -f keycloak | grep "Listening on"
```

### 2. Create an Admin User

```bash
chmod +x ./scripts/create-admin-user.sh
./scripts/create-admin-user.sh admin admin@lianel.se MySecurePassword123 Admin User
```

**Script usage**:
```
./scripts/create-admin-user.sh <username> <email> <password> [firstName] [lastName]
```

The script:
- Creates the user in the 'lianel' realm
- Sets the password (non-temporary)
- Assigns the 'admin' realm role
- Outputs success/failure messages

### 3. Build & Deploy Backend

```bash
cd /root/lianel/dc/profile-service
cargo build --release
docker build -t lianel-profile-service:latest .
```

Deploy to remote (replace with your server):
```bash
# Save image
docker save lianel-profile-service:latest | gzip > profile-service-latest.tar.gz

# Transfer and load
scp profile-service-latest.tar.gz user@server:/tmp/
ssh user@server 'cd /root/lianel/dc && docker load < /tmp/profile-service-latest.tar.gz'

# Restart backend
ssh user@server 'cd /root/lianel/dc && docker-compose -f docker-compose.backend.yaml restart profile-service'
```

### 4. Build & Deploy Frontend

```bash
cd /root/lianel/dc/frontend
npm run build
```

Deploy to remote:
```bash
# Build Docker image locally
docker build -t lianel-frontend:latest .

# Save and transfer
docker save lianel-frontend:latest | gzip > frontend-latest.tar.gz
scp frontend-latest.tar.gz user@server:/tmp/

# Load and restart on remote
ssh user@server 'cd /root/lianel/dc && docker load < /tmp/frontend-latest.tar.gz && docker-compose -f docker-compose.frontend.yaml restart frontend'
```

## Features

### Backend Admin APIs

All endpoints require Bearer token with 'admin' realm role and return 403 if unauthorized.

- **GET /api/admin/users** – List all users with pagination & search
  - Query params: `page` (1-based), `size` (1-100), `search` (username/email/name)
  - Response: `{ users: [...], total: N }`

- **GET /api/admin/users/{id}** – Fetch user details

- **POST /api/admin/users** – Create user
  - Body: `{ username, email, password, firstName?, lastName?, enabled? }`

- **PUT /api/admin/users/{id}** – Update user (firstName, lastName, email)

- **DELETE /api/admin/users/{id}** – Delete user

- **POST /api/admin/users/{id}/change-password** – Reset user password
  - Body: `{ new_password }`

- **GET /api/admin/check** – Check if current user is admin
  - Response: `{ isAdmin: boolean }`

### Frontend Admin UI

Accessible at `/admin/users` (requires admin role).

- **Users List** – Paginated, searchable table with status
- **Create User** – Form to add new users
- **User Details** – View individual user info
- **Admin Console Card** – Links from dashboard for admins

### CORS Configuration

CORS is now restricted to production origins via env var `ALLOWED_ORIGINS`:
```bash
ALLOWED_ORIGINS=https://lianel.se,https://www.lianel.se
```

Requests from other origins will be rejected. Credentials allowed only for specified origins.

## Troubleshooting

**Admin endpoints return 403**:
- Verify user has 'admin' realm role in Keycloak
- Check token is valid and not expired

**Admin UI shows "403 – Admin Access Required"**:
- Ensure user is logged in
- Verify user's realm roles include 'admin'
- Check browser console for API errors

**Script fails with "keycloak container not running"**:
- Start infra: `docker-compose -f docker-compose.infra.yaml up -d`
- Verify with: `docker ps | grep keycloak`

**Password change fails**:
- Ensure new password is at least 8 characters
- Verify admin user has change-password permission in Keycloak

## Next Steps

- Set up audit logging for user management actions
- Implement role assignment UI (admin/manager/viewer roles)
- Add user enable/disable endpoint
- Configure production monitoring and alerting
