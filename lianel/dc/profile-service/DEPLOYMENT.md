# Profile Service Deployment Guide

## Overview

The Profile Service is a Rust-based backend API that handles user profile management, acting as an intermediary between the frontend and Keycloak.

## Features Implemented

✅ **OpenAPI/Swagger UI**: Interactive API documentation at `/swagger-ui`
✅ **HTTPS Only**: All API endpoints enforce HTTPS
✅ **CI/CD Pipeline**: Automated deployment via GitHub Actions
✅ **Profile Management**: Get, update profile, change password
✅ **Frontend Integration**: React frontend uses backend APIs

## API Endpoints

- `GET /api/profile` - Get user profile
- `PUT /api/profile` - Update profile (firstName, lastName, email)
- `POST /api/profile/change-password` - Change password
- `GET /health` - Health check
- `GET /swagger-ui` - Swagger UI documentation
- `GET /api-doc/openapi.json` - OpenAPI specification

## Deployment Steps

### 1. Manual Deployment

```bash
# Build image
cd lianel/dc/profile-service
docker buildx build --platform linux/amd64 -t lianel-profile-service:latest -f Dockerfile .

# Save and transfer
docker save lianel-profile-service:latest | gzip > profile-service-image.tar.gz
scp profile-service-image.tar.gz root@72.60.80.84:/root/lianel/dc/

# On remote host
cd /root/lianel/dc
docker load < profile-service-image.tar.gz
docker-compose -f docker-compose.backend.yaml up -d profile-service
```

### 2. Automated Deployment (CI/CD)

The GitHub Actions workflow automatically:
- Builds Rust service for linux/amd64
- Pushes to GHCR
- Transfers to remote host
- Deploys and verifies

Triggered on changes to:
- `lianel/dc/profile-service/**`
- `lianel/dc/docker-compose.backend.yaml`

### 3. Verify Deployment

```bash
# Check container status
docker ps | grep lianel-profile-service

# Check logs
docker logs lianel-profile-service --tail 50

# Test health endpoint
curl http://localhost:3000/health

# Test via nginx (requires authentication)
curl -k https://lianel.se/api/profile
curl -k https://lianel.se/swagger-ui/
```

## Configuration

### Environment Variables

Required in `.env` file:
```bash
KEYCLOAK_ADMIN_USER=admin
KEYCLOAK_ADMIN_PASSWORD=your-password
KEYCLOAK_REALM=lianel  # Optional
KEYCLOAK_URL=http://keycloak:8080  # Optional
PORT=3000  # Optional
RUST_LOG=info  # Optional
```

### Nginx Configuration

The nginx config routes:
- `/api/*` → Profile service (HTTPS enforced)
- `/swagger-ui/*` → Swagger UI (OAuth protected)
- `/api-doc/*` → OpenAPI JSON (OAuth protected)

## Testing

### Local Testing

```bash
# Run locally
cd lianel/dc/profile-service
cargo run

# Test endpoints (requires headers from OAuth2-proxy)
curl -H "X-User: testuser" -H "X-Email: test@lianel.se" \
  http://localhost:3000/api/profile
```

### E2E Testing

1. **Login**: Navigate to https://lianel.se
2. **View Profile**: Click user dropdown → Profile
3. **Edit Profile**: Click "Edit Profile", update fields, save
4. **Change Password**: Click "Change Password", enter current/new passwords
5. **Swagger UI**: Navigate to https://lianel.se/swagger-ui/

## Troubleshooting

### Build Issues

- **Rust version**: Ensure Dockerfile uses compatible Rust version
- **Dependencies**: Check Cargo.toml for version conflicts
- **Platform**: Build for linux/amd64

### Runtime Issues

- **Keycloak connection**: Verify KEYCLOAK_URL and network connectivity
- **Admin credentials**: Check KEYCLOAK_ADMIN_USER and KEYCLOAK_ADMIN_PASSWORD
- **Logs**: Check `docker logs lianel-profile-service`

### API Issues

- **401 Unauthorized**: Check OAuth2-proxy headers are passed
- **404 Not Found**: Verify user exists in Keycloak
- **500 Internal Server Error**: Check Keycloak admin API access

## Security

- ✅ HTTPS enforced for all API endpoints
- ✅ OAuth2 authentication required
- ✅ User identification via headers (set by nginx/OAuth2-proxy)
- ✅ Password verification before change
- ✅ Admin credentials never exposed to frontend

## Performance

- Rust provides excellent performance
- Small binary size (~10-15MB)
- Low memory footprint
- Fast startup time

