# GitHub Actions Workflows

This directory contains CI/CD workflows for automated deployment.

## Frontend Deployment Workflow

**File**: `deploy-frontend.yml`

### Overview
Automatically builds and deploys the React frontend to the production server when changes are pushed to the `master` or `main` branch.

### Trigger Conditions
- Push to `master` or `main` branch with changes in:
  - `lianel/dc/frontend/**`
  - `lianel/dc/docker-compose.frontend.yaml`
  - `lianel/dc/docker-compose.yaml`
- Manual trigger via GitHub Actions UI (`workflow_dispatch`)

### Workflow Steps
1. **Checkout**: Gets the latest code
2. **Docker Buildx Setup**: Sets up multi-platform builds
3. **Build Image**: Builds frontend Docker image for `linux/amd64`
4. **Push to Registry**: Pushes image to GitHub Container Registry (ghcr.io)
5. **Save Image**: Saves image as tar.gz for deployment
6. **Upload Artifact**: Stores image as GitHub Actions artifact
7. **SCP Transfer**: Transfers image to remote host
8. **Deploy**: Loads image and restarts container on remote host
9. **Cleanup**: Removes old images and temporary files

### Required Secrets
Configure these secrets in your GitHub repository settings (Settings → Secrets and variables → Actions):

| Secret Name | Description | Example |
|------------|-------------|---------|
| `REMOTE_HOST` | Remote server IP or hostname | `72.60.80.84` |
| `REMOTE_USER` | SSH username | `root` |
| `SSH_PRIVATE_KEY` | Private SSH key for authentication | `-----BEGIN OPENSSH PRIVATE KEY-----...` |
| `REMOTE_PORT` | SSH port (optional, defaults to 22) | `22` |
| `KEYCLOAK_ADMIN_PASSWORD` | Keycloak admin password (optional). If set, the deploy runs `fix-keycloak-https.sh` and `update-keycloak-frontend-client.sh` on the remote so realm frontendUrl and frontend-client rootUrl point to https://www.lianel.se. | *(your Keycloak admin password)* |

### How to Set Up Secrets

1. Go to your repository: https://github.com/NimaLAN74/hosting-base
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Add each secret listed above

#### Generating SSH Key (if needed)
```bash
# Generate SSH key pair
ssh-keygen -t ed25519 -C "github-actions" -f ~/.ssh/github_actions_deploy

# Copy public key to remote host
ssh-copy-id -i ~/.ssh/github_actions_deploy.pub root@72.60.80.84

# Copy private key content (to add as secret)
cat ~/.ssh/github_actions_deploy
```

### Image Registry
Images are pushed to GitHub Container Registry:
- **Registry**: `ghcr.io`
- **Image**: `ghcr.io/nimalan74/hosting-base/lianel-frontend:latest`
- **Tags**: `latest`, `master-<sha>`, `<branch>-<sha>`

### Manual Deployment
You can manually trigger deployment:
1. Go to **Actions** tab in GitHub
2. Select **Deploy Frontend to Production**
3. Click **Run workflow**
4. Select branch and click **Run workflow**

### Monitoring the pipe and process
The workflow is instrumented so you can follow both the **pipeline** and the **deployed process**:

| What | Where |
|------|--------|
| **Pipeline context** | First step prints Run ID, branch, workflow, and a direct link to the run. Check the step summary for the run URL. |
| **SSH preflight** | Before copying the deploy script, a step tests SSH to the host. If this fails, fix `REMOTE_HOST`/`REMOTE_USER`/`REMOTE_PORT`, SSH key, or firewall before the Copy step. |
| **Copy step** | Logs `[Monitor] Copy step — REMOTE_HOST=... REMOTE_USER=... REMOTE_PORT=...`, checks the local script exists, then runs `scp` and `ssh chmod` with clear pass/fail and exit-code messages. |
| **Live process** | After deploy and nginx sync, a step curls the frontend URL (default `https://www.lianel.se`) and fails the job if the response is not 2xx/3xx. Override with repository variable `FRONTEND_URL` if needed. |

- **Actions**: https://github.com/NimaLAN74/hosting-base/actions  
- Use the run URL from the “Pipeline context” step to jump to that run’s logs.

### Troubleshooting

#### SSH Connection Failed
- Verify SSH key is correctly added as secret
- Check remote host firewall allows SSH
- Test SSH connection manually: `ssh -i <key> root@72.60.80.84`

#### Docker Build Failed
- Check Dockerfile syntax
- Verify frontend dependencies are correct
- Review build logs in GitHub Actions

#### Deployment Failed
- Check remote host Docker is running: `docker ps`
- Verify docker-compose.yaml is correct
- Check container logs: `docker logs lianel-frontend`

### Fix Keycloak 502 on Remote

**File**: `fix-keycloak-502.yml`

When **auth.lianel.se** returns **502 Bad Gateway** (e.g. after a deploy that recreated Keycloak), this workflow SSHs to the remote host and:

1. Pulls the latest repo
2. Runs `set-keycloak-db-password-on-server.sh` (syncs Postgres `keycloak` user password from `.env`)
3. Brings up Keycloak (`docker compose -f docker-compose.infra.yaml up -d keycloak`)
4. Reloads nginx
5. Verifies auth.lianel.se responds (not 502)

- **Trigger**: Manual only — **Actions → "Fix Keycloak 502 on Remote" → Run workflow**
- **Secrets**: Same as other deploy workflows (`REMOTE_HOST`, `REMOTE_USER`, `SSH_PRIVATE_KEY`, optional `REMOTE_PORT`)
- **Server**: Expects repo at `/root/hosting-base` and `.env` at `/root/hosting-base/lianel/dc/.env` with `POSTGRES_PASSWORD` and `KEYCLOAK_DB_PASSWORD`

### Fix Keycloak Redirect (Remote)

**File**: `fix-keycloak-redirect.yml`

Applies the Keycloak redirect fix on the remote host via pipeline SSH: copies `docker-compose.infra.yaml` and `nginx.conf`, restarts Keycloak, reloads nginx.

**Run with correct GH profile:**

1. Log in to the GitHub account that has access to this repo (e.g. NimaLAN74):
   ```bash
   gh auth login --web --hostname github.com
   ```
   Complete the browser flow. If you have multiple accounts: `gh auth switch` and choose the correct one.

2. Trigger the workflow:
   ```bash
   GH_TOKEN="$(gh auth token)" gh workflow run fix-keycloak-redirect.yml
   ```
   Or set `GH_TOKEN` to a Personal Access Token (repo + workflow scope) and run:
   ```bash
   export GH_TOKEN='your_token_here'
   gh workflow run fix-keycloak-redirect.yml
   ```

3. Or run from the UI: **Actions** → **Fix Keycloak Redirect (Remote)** → **Run workflow**.

### Airflow Webserver Config Deployment

**File**: `deploy-airflow-webserver-config.yml`

Deploys `lianel/dc/config/webserver_config.py` to the remote host and restarts the Airflow apiserver. Uses the pipeline as much as possible; only the copy and restart run on the remote via SSH.

- **Trigger**: Push to `master`/`main` when `lianel/dc/config/webserver_config.py` or this workflow changes; or `workflow_dispatch`.
- **Steps**: Checkout → validate secrets → setup SSH → scp config to `/root/lianel/dc/config/` → ssh restart `airflow-apiserver` → optionally sync to `/root/hosting-base/lianel/dc/config` if present.
- **Secrets**: Same as frontend (`REMOTE_HOST`, `REMOTE_USER`, `SSH_PRIVATE_KEY`, optional `REMOTE_PORT`).

For manual deploy from your machine (when pipeline is not used): run `bash lianel/dc/scripts/deployment/deploy-airflow-webserver-config.sh` from repo root (uses `scripts/SSH-CONFIG.md`).

### Future Enhancements
- [ ] Add rollback capability
- [x] Add health checks after deployment (live curl of frontend URL)
- [ ] Add notification (Slack, email) on deployment
- [ ] Add deployment to staging environment
- [ ] Add automated testing before deployment

