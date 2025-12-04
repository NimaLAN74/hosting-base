# Setting Up GitHub Actions for Frontend Deployment

This guide will help you configure GitHub Actions to automatically deploy the frontend to your remote host.

## Prerequisites

- GitHub repository: https://github.com/NimaLAN74/hosting-base
- Remote host access: `root@72.60.80.84`
- SSH key pair for authentication

## Step 1: Generate SSH Key (if you don't have one)

```bash
# Generate a new SSH key specifically for GitHub Actions
ssh-keygen -t ed25519 -C "github-actions-deploy" -f ~/.ssh/github_actions_deploy

# This creates:
# - ~/.ssh/github_actions_deploy (private key - keep secret!)
# - ~/.ssh/github_actions_deploy.pub (public key - add to server)
```

## Step 2: Add Public Key to Remote Host

```bash
# Copy public key to remote host
ssh-copy-id -i ~/.ssh/github_actions_deploy.pub root@72.60.80.84

# Or manually add to authorized_keys
cat ~/.ssh/github_actions_deploy.pub | ssh root@72.60.80.84 "mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys"
```

## Step 3: Test SSH Connection

```bash
# Test the connection
ssh -i ~/.ssh/github_actions_deploy root@72.60.80.84 "echo 'Connection successful!'"
```

## Step 4: Add Secrets to GitHub Repository

1. Go to your repository: https://github.com/NimaLAN74/hosting-base
2. Click **Settings** (top menu)
3. Click **Secrets and variables** → **Actions** (left sidebar)
4. Click **New repository secret**

### Add These Secrets:

#### 1. REMOTE_HOST
- **Name**: `REMOTE_HOST`
- **Value**: `72.60.80.84`

#### 2. REMOTE_USER
- **Name**: `REMOTE_USER`
- **Value**: `root`

#### 3. SSH_PRIVATE_KEY
- **Name**: `SSH_PRIVATE_KEY`
- **Value**: Copy the entire private key content:
  ```bash
  cat ~/.ssh/github_actions_deploy
  ```
  Copy everything including:
  ```
  -----BEGIN OPENSSH PRIVATE KEY-----
  ...
  -----END OPENSSH PRIVATE KEY-----
  ```

#### 4. REMOTE_PORT (Optional)
- **Name**: `REMOTE_PORT`
- **Value**: `22` (default, can be omitted)

## Step 5: Verify Workflow File

The workflow file is already created at:
`.github/workflows/deploy-frontend.yml`

It will:
- Build frontend Docker image for `linux/amd64`
- Push to GitHub Container Registry
- Deploy to your remote host automatically

## Step 6: Test the Workflow

### Option A: Manual Trigger
1. Go to **Actions** tab in GitHub
2. Select **Deploy Frontend to Production**
3. Click **Run workflow**
4. Select branch: `master`
5. Click **Run workflow**

### Option B: Push a Change
Make a small change to trigger automatic deployment:
```bash
# Make a small change to frontend
echo "# Test" >> lianel/dc/frontend/README.md
git add .
git commit -m "Test GitHub Actions deployment"
git push origin master
```

## Step 7: Monitor Deployment

1. Go to **Actions** tab
2. Click on the running workflow
3. Watch the logs in real-time
4. Check for any errors

## Step 8: Verify Deployment

After workflow completes, verify on remote host:

```bash
ssh root@72.60.80.84

# Check container is running
docker ps | grep lianel-frontend

# Check container logs
docker logs lianel-frontend --tail 50

# Check image
docker images | grep lianel-frontend
```

## Troubleshooting

### Workflow Fails at SSH Step
- Verify SSH key is correct in secrets
- Check remote host firewall allows SSH
- Test SSH manually: `ssh -i <key> root@72.60.80.84`

### Workflow Fails at Docker Step
- Check Docker is running on remote: `systemctl status docker`
- Verify docker-compose.yaml exists: `ls /root/lianel/dc/docker-compose.yaml`

### Image Not Loading
- Check disk space: `df -h`
- Verify image file transferred: `ls -lh /root/lianel/dc/frontend-image.tar.gz`

### Container Not Starting
- Check logs: `docker logs lianel-frontend`
- Verify network: `docker network ls | grep lianel-network`
- Check compose file: `docker-compose -f docker-compose.yaml config`

## Security Best Practices

1. **Never commit SSH private keys** - Always use GitHub Secrets
2. **Use dedicated SSH key** - Don't reuse your personal SSH key
3. **Restrict SSH key** - Consider using `command=` restriction in authorized_keys
4. **Rotate keys regularly** - Update SSH keys periodically
5. **Monitor access** - Check SSH logs: `grep sshd /var/log/auth.log`

## Advanced: Restrict SSH Key to Deployment Only

Edit `/root/.ssh/authorized_keys` on remote host:

```
command="cd /root/lianel/dc && docker load < frontend-image.tar.gz && docker-compose -f docker-compose.yaml up -d frontend",no-port-forwarding,no-X11-forwarding,no-pty ssh-ed25519 AAAAC3NzaC1lZDI1NTE5... github-actions-deploy
```

## Next Steps

- Set up notifications (Slack, email) for deployment status
- Add staging environment deployment
- Add automated testing before deployment
- Add rollback capability

