# Profile Service Implementation

## Overview

A complete profile service written in Rust has been implemented to handle user profile management, acting as an intermediary between the frontend and Keycloak. This provides a clean API layer that hides Keycloak implementation details from the frontend.

## Architecture

```
Frontend (React) → Nginx → Profile Service (Rust) → Keycloak Admin API
                    ↓
                OAuth2-proxy (authentication)
```

## Components

### 1. Profile Service (`lianel/dc/profile-service/`)
- **Technology**: Rust + Axum
- **Port**: 3000
- **Features**:
  - Get user profile
  - Update user profile (firstName, lastName, email)
  - Change password
  - Health check endpoint

### 2. Frontend Updates (`lianel/dc/frontend/src/Profile.js`)
- Enhanced Profile page with:
  - Edit mode for profile fields
  - Change password form
  - Error and success messaging
  - Form validation

### 3. Nginx Configuration
- Routes `/api/*` requests to backend service
- Passes OAuth2-proxy user headers to backend
- Maintains authentication for API endpoints

### 4. Docker Configuration
- `docker-compose.backend.yaml`: Standalone backend service
- `docker-compose.yaml`: Updated to include backend service

## API Endpoints

### GET `/api/profile`
Returns current user's profile information.

### PUT `/api/profile`
Updates user profile (firstName, lastName, email).

**Request Body:**
```json
{
  "firstName": "John",
  "lastName": "Doe",
  "email": "john.doe@lianel.se"
}
```

### POST `/api/profile/change-password`
Changes user password.

**Request Body:**
```json
{
  "currentPassword": "oldpassword",
  "newPassword": "newpassword123"
}
```

## Deployment

### 1. Build Profile Service Image
```bash
cd lianel/dc/profile-service
docker buildx build --platform linux/amd64 -t lianel-profile-service:latest -f Dockerfile .
```

### 2. Deploy Profile Service
```bash
# On remote host
cd /root/lianel/dc
docker-compose -f docker-compose.backend.yaml up -d
# OR if using main docker-compose.yaml
docker-compose -f docker-compose.yaml up -d profile-service
```

### 3. Update Frontend
```bash
# Build and deploy frontend with updated Profile component
cd lianel/dc/frontend
docker buildx build --platform linux/amd64 -t lianel-frontend:latest -f Dockerfile .
```

### 4. Update Nginx
```bash
# Copy updated nginx.conf to remote host
scp lianel/dc/nginx/config/nginx.conf root@72.60.80.84:/root/lianel/dc/nginx/config/nginx.conf
ssh root@72.60.80.84 "docker restart nginx-proxy"
```

## Environment Variables

The profile service requires these environment variables (from `.env` file):

```bash
KEYCLOAK_ADMIN_USER=admin
KEYCLOAK_ADMIN_PASSWORD=your-admin-password
KEYCLOAK_REALM=lianel  # Optional, defaults to 'lianel'
RUST_LOG=info  # Optional, logging level
```

## Security

- All API endpoints require OAuth2 authentication via nginx
- User identification comes from OAuth2-proxy headers (set by nginx)
- Profile service uses Keycloak Admin API with admin credentials
- Password changes verify current password before allowing change
- Email changes reset email verification status
- Rust's memory safety prevents common security vulnerabilities

## Frontend Features

### Profile Page
- **View Mode**: Displays user information (read-only)
- **Edit Mode**: Allows editing firstName, lastName, and email
- **Change Password**: Separate form for password changes
- **Validation**: 
  - Password must be at least 8 characters
  - New password confirmation must match
  - Current password verification

### User Experience
- Success/error messages for all operations
- Loading states during API calls
- Form validation feedback
- Responsive design matching main site

## Benefits

1. **Separation of Concerns**: Frontend doesn't need to know about Keycloak
2. **Maintainability**: Changes to Keycloak don't affect frontend code
3. **Security**: Keycloak admin credentials never exposed to frontend
4. **Extensibility**: Easy to add more profile features without frontend changes
5. **API Consistency**: Clean REST API for frontend to consume
6. **Performance**: Rust provides excellent performance and memory efficiency
7. **Reliability**: Rust's type system and memory safety prevent many bugs

## Testing

After deployment, test the endpoints:

```bash
# Get profile (requires authentication)
curl -H "X-User: testuser" -H "X-Email: test@lianel.se" \
  http://localhost:3000/api/profile

# Update profile
curl -X PUT -H "Content-Type: application/json" \
  -H "X-User: testuser" -H "X-Email: test@lianel.se" \
  -d '{"firstName":"Test","lastName":"User"}' \
  http://localhost:3000/api/profile

# Health check
curl http://localhost:3000/health
```

## Next Steps

Potential future enhancements:
- Email verification resend
- Profile picture upload
- Two-factor authentication settings
- Account deletion
- Activity history
- Notification preferences

