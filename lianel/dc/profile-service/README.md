# Lianel Profile Service

Backend service written in Rust that acts as an intermediary between the frontend and Keycloak for user profile management.

## Technology Stack

- **Language**: Rust
- **Web Framework**: Axum
- **HTTP Client**: Reqwest
- **Serialization**: Serde

## Features

- **Get User Profile**: Retrieve current user's profile information
- **Update Profile**: Update firstName, lastName, and email
- **Change Password**: Change user password with current password verification

## API Endpoints

### GET `/api/profile`
Get current user's profile information.

**Headers:**
- `X-User` or `X-Auth-Request-User`: Username (set by OAuth2-proxy via nginx)
- `X-Email` or `X-Auth-Request-Email`: Email (set by OAuth2-proxy via nginx)

**Response:**
```json
{
  "id": "user-uuid",
  "username": "testuser",
  "email": "test@lianel.se",
  "firstName": "Test",
  "lastName": "User",
  "name": "Test User",
  "email_verified": true,
  "enabled": true
}
```

### PUT `/api/profile`
Update user profile information.

**Headers:** Same as GET

**Body:**
```json
{
  "firstName": "New First Name",
  "lastName": "New Last Name",
  "email": "newemail@lianel.se"
}
```

**Response:** Updated user profile (same format as GET)

### POST `/api/profile/change-password`
Change user password.

**Headers:** Same as GET

**Body:**
```json
{
  "current_password": "oldpassword",
  "new_password": "newpassword123"
}
```

**Response:**
```json
{
  "message": "Password changed successfully"
}
```

### GET `/health`
Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "service": "lianel-profile-service"
}
```

## Environment Variables

- `PORT`: Server port (default: 3000)
- `KEYCLOAK_URL`: Keycloak server URL (default: http://keycloak:8080)
- `KEYCLOAK_REALM`: Keycloak realm name (default: lianel)
- `KEYCLOAK_ADMIN_USER`: Keycloak admin username (required)
- `KEYCLOAK_ADMIN_PASSWORD`: Keycloak admin password (required)
- `RUST_LOG`: Logging level (default: info)

## Building

### Local Development
```bash
cargo build --release
cargo run
```

### Docker Build
```bash
docker buildx build --platform linux/amd64 -t lianel-profile-service:latest -f profile-service/Dockerfile profile-service/
```

## Docker Deployment

```bash
# Build image
docker buildx build --platform linux/amd64 -t lianel-profile-service:latest -f profile-service/Dockerfile profile-service/

# Or use docker-compose
docker-compose -f docker-compose.backend.yaml build
docker-compose -f docker-compose.backend.yaml up -d
```

## Architecture

The profile service:
1. Receives authenticated requests from nginx (with user headers from OAuth2-proxy)
2. Uses Keycloak Admin API to perform user operations
3. Returns user data to the frontend
4. Hides Keycloak implementation details from the frontend

This provides a clean separation between frontend and Keycloak, allowing for easier maintenance and future enhancements.

## Performance

Rust provides:
- **Memory Safety**: No garbage collection overhead
- **Performance**: Near C/C++ performance
- **Concurrency**: Excellent async/await support with Tokio
- **Small Binary**: Alpine-based Docker image is minimal

## Error Handling

The service uses Rust's Result type for comprehensive error handling:
- Proper HTTP status codes
- Detailed error messages
- Structured error responses
