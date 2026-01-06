# 🔐 Admin Credentials & Login Guide

## 📋 Admin Login Credentials

### Keycloak Admin (Master Realm)
- **Username**: `admin`
- **Password**: `D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA`
- **Realm**: `master`
- **URL**: https://auth.lianel.se/admin

### Application Admin User (Lianel Realm)
- **Username**: `admin`
- **Password**: `D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA`
- **Realm**: `lianel`
- **Login URL**: https://lianel.se (redirects to Keycloak)

---

## 🚀 How to Log In as Admin

### Step 1: Visit the Application
1. Go to: **https://lianel.se**
2. Click **"Login"** button
3. You'll be redirected to: **https://auth.lianel.se**

### Step 2: Enter Credentials
- **Username**: `admin`
- **Password**: `D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA`
- **Realm**: `lianel` (should be selected automatically)

### Step 3: Access Admin Features
After login, you'll be redirected back to https://lianel.se with admin access.

**Admin features available:**
- User management at `/admin/users`
- Admin console card on dashboard
- Access to all admin API endpoints

---

## 🔧 Environment Variables

### Keycloak Configuration
These are set in `docker-compose.infra.yaml`:

```yaml
KEYCLOAK_ADMIN_USER=admin
KEYCLOAK_ADMIN_PASSWORD=D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA
```

### Profile Service Configuration
If you need to configure the profile service, create a `.env` file in `/root/lianel/dc/profile-service/`:

```bash
KEYCLOAK_ADMIN_USER=admin
KEYCLOAK_ADMIN_PASSWORD=D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA
KEYCLOAK_REALM=lianel
KEYCLOAK_URL=http://keycloak:8080
PORT=3000
RUST_LOG=info
```

---

## 📁 Where Credentials Are Stored

### On Remote Host
Credentials are configured in:
- **Docker Compose files**: `/root/lianel/dc/docker-compose.infra.yaml`
- **Keycloak container**: Environment variables passed to container
- **No `.env` file needed** (credentials are in docker-compose files)

### To Check Current Configuration
```bash
ssh root@72.60.80.84 "cd /root/lianel/dc && docker-compose -f docker-compose.infra.yaml config | grep KEYCLOAK_ADMIN"
```

---

## 🛠️ Create/Update Admin User

If the admin user doesn't exist, create it:

```bash
ssh root@72.60.80.84 "cd /root/lianel/dc && ./scripts/create-admin-user.sh admin admin@lianel.se D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA Admin User"
```

---

## 🔒 Security Notes

⚠️ **Important**: These are default credentials. For production:
1. Change the Keycloak admin password
2. Change the application admin user password
3. Use strong, unique passwords
4. Consider using secrets management (e.g., Docker secrets, Kubernetes secrets)

---

## 📝 Quick Reference

| Service | Username | Password | Realm |
|---------|----------|----------|-------|
| Keycloak Admin | `admin` | `D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA` | `master` |
| Application Admin | `admin` | `D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA` | `lianel` |

**Login URL**: https://lianel.se

---

**Status**: Ready to log in! 🎉

