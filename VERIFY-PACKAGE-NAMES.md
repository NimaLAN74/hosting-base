# Verify Package Names and Format

## 🔍 Issue

Packages are still returning 401 (UNAUTHORIZED) even though you say they're public. This suggests either:
1. Packages aren't actually public
2. Package names don't match what we're trying to pull

## 📋 Current Format We're Using

**Workflow generates:**
- `nimalan74/hosting-base/lianel-frontend`
- `nimalan74/hosting-base/lianel-profile-service`

**Full image tags:**
- `ghcr.io/nimalan74/hosting-base/lianel-frontend:latest`
- `ghcr.io/nimalan74/hosting-base/lianel-profile-service:latest`

## ✅ How to Verify Package Names

1. Go to: https://github.com/NimaLAN74?tab=packages
2. Click on each package
3. Check the **exact package name** shown in the URL or package settings
4. Compare with what we're using above

## 🔧 Common Package Name Formats

GitHub Container Registry can have different formats:

### Format 1: Repository-based (what we're using)
```
ghcr.io/owner/repo/image-name:tag
Example: ghcr.io/nimalan74/hosting-base/lianel-frontend:latest
```

### Format 2: Simple name
```
ghcr.io/owner/image-name:tag
Example: ghcr.io/nimalan74/lianel-frontend:latest
```

### Format 3: With underscores/hyphens
```
ghcr.io/owner/repo-image-name:tag
Example: ghcr.io/nimalan74/hosting-base-lianel-frontend:latest
```

## 🧪 Test Different Formats

Try these commands on the remote host to see which format works:

```bash
# Format 1 (current)
docker pull ghcr.io/nimalan74/hosting-base/lianel-frontend:latest

# Format 2 (simple)
docker pull ghcr.io/nimalan74/lianel-frontend:latest

# Format 3 (hyphenated)
docker pull ghcr.io/nimalan74/hosting-base-lianel-frontend:latest
```

## 📝 Next Steps

1. **Check the actual package names** in GitHub Packages
2. **Verify they're actually public** (should show "Public" badge)
3. **Share the exact package names** you see
4. **I'll update the workflow** to match the correct format

