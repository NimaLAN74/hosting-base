# User Profile and Dropdown Features

## Overview
This document describes the user dropdown and profile page features added to the frontend.

## Features

### 1. User Dropdown (Top Right)
- **Location**: Top-right corner of the header
- **Icon**: User avatar with initials
- **Features**:
  - Shows user avatar with initials
  - Dropdown menu with:
    - User name and email (in header)
    - Profile link
    - Logout button

### 2. Profile Page
- **Route**: `/profile`
- **Features**:
  - Displays Keycloak user information:
    - Username
    - Email
    - Full Name (if available)
    - User ID
  - Actions:
    - Back to Home button
    - Logout button

## Technical Implementation

### Components
- `UserDropdown.js` - Dropdown component with user menu
- `Profile.js` - Profile page component
- `UserDropdown.css` - Styles for dropdown
- `Profile.css` - Styles for profile page

### API Endpoint
- `/api/user-info` - Returns user information from OAuth2-proxy headers
  - Reads `X-User` and `X-Email` headers set by nginx
  - Returns JSON with user details

### Routing
- Uses `react-router-dom` for client-side routing
- Routes:
  - `/` - Home page with dropdown
  - `/profile` - Profile page

### Backend Changes
- Updated Dockerfile to use Node.js/Express instead of nginx
- Express server serves React app and provides API endpoint
- Server reads user headers from nginx proxy

## Deployment Notes

1. **Build the frontend**:
   ```bash
   cd lianel/dc/frontend
   npm install
   npm run build
   ```

2. **Build Docker image**:
   ```bash
   docker buildx build --platform linux/amd64 -t lianel-frontend:latest -f frontend/Dockerfile frontend/
   ```

3. **Deploy**:
   - Transfer image to remote host
   - Load and restart container

## User Information Source

User information comes from OAuth2-proxy via nginx headers:
- `X-User` / `X-Auth-Request-User` - Username
- `X-Email` / `X-Auth-Request-Email` - Email address

These headers are set by nginx after OAuth2-proxy authentication.

