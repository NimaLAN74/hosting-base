# GitHub Secrets Setup for Pipeline

**SSH Key**: `id_ed25519_host`  
**Status**: ✅ SSH connection verified  
**Remote Host**: 72.60.80.84

---

## 🔐 Required GitHub Secrets

Add these secrets in: https://github.com/NimaLAN74/hosting-base/settings/secrets/actions

### 1. SSH_PRIVATE_KEY

**Name**: `SSH_PRIVATE_KEY`  
**Value**: Copy the entire private key content from `~/.ssh/id_ed25519_host`

```bash
# Get the private key content:
cat ~/.ssh/id_ed25519_host
```

Copy everything including:
```
-----BEGIN OPENSSH PRIVATE KEY-----
...
-----END OPENSSH PRIVATE KEY-----
```

### 2. REMOTE_HOST

**Name**: `REMOTE_HOST`  
**Value**: `72.60.80.84`

### 3. REMOTE_USER

**Name**: `REMOTE_USER`  
**Value**: `root`

### 4. REMOTE_PORT (Optional)

**Name**: `REMOTE_PORT`  
**Value**: `22` (default, can be omitted)

---

## ✅ Verification

### Remote Host Status:
- ✅ SSH connection: Working
- ✅ Docker: Version 29.0.4
- ✅ Docker Compose: Version v2.40.3
- ✅ Network: `lianel-network` exists
- ✅ Compose files: All present in `/root/lianel/dc`

### Compose Files Found:
- `docker-compose.yaml` ✅
- `docker-compose.frontend.yaml` ✅
- `docker-compose.backend.yaml` ✅
- `docker-compose.infra.yaml` ✅
- `docker-compose.oauth2-proxy.yaml` ✅
- `docker-compose.airflow.yaml` ✅
- `docker-compose.monitoring.yaml` ✅

---

## 🚀 Next Steps

1. **Add SSH_PRIVATE_KEY to GitHub Secrets**:
   - Go to: https://github.com/NimaLAN74/hosting-base/settings/secrets/actions
   - Click **New repository secret**
   - Name: `SSH_PRIVATE_KEY`
   - Value: Paste private key content
   - Click **Add secret**

2. **Verify other secrets exist**:
   - `REMOTE_HOST` = `72.60.80.84`
   - `REMOTE_USER` = `root`
   - `REMOTE_PORT` = `22` (optional)

3. **Test the pipeline**:
   - Push a change or manually trigger workflow
   - Check: https://github.com/NimaLAN74/hosting-base/actions

---

## 📝 Notes

- The workflow uses `~/.ssh/deploy_key` (created from `SSH_PRIVATE_KEY` secret)
- SSH key is already added to remote host's `~/.ssh/authorized_keys`
- Network `lianel-network` already exists on remote host
- All compose files are in place

**Status**: ✅ Ready - just add `SSH_PRIVATE_KEY` secret to GitHub

