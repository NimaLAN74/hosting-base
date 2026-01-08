# Quick Guide: Set Up SSH Key for GitHub Actions

## ⚠️ Important Security Note
**Never share private keys!** Private keys must remain secret. This guide will help you generate a new key pair properly.

## Step 1: Generate New SSH Key Pair

On your **local machine** (where you have access), run:

```bash
# Generate a new SSH key specifically for GitHub Actions
ssh-keygen -t ed25519 -C "github-actions-deploy" -f ~/.ssh/github_actions_deploy

# When prompted:
# - Press Enter to accept default location
# - Enter a passphrase (optional but recommended) or press Enter for no passphrase
```

This creates:
- **Private key**: `~/.ssh/github_actions_deploy` (keep this secret!)
- **Public key**: `~/.ssh/github_actions_deploy.pub` (safe to share)

## Step 2: Add Public Key to Remote Server

```bash
# Copy public key to remote server (replace with your actual server IP if different)
ssh-copy-id -i ~/.ssh/github_actions_deploy.pub root@72.60.80.84

# If ssh-copy-id doesn't work, use this:
cat ~/.ssh/github_actions_deploy.pub | ssh root@72.60.80.84 "mkdir -p ~/.ssh && chmod 700 ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"
```

## Step 3: Test SSH Connection

```bash
# Test that the key works
ssh -i ~/.ssh/github_actions_deploy root@72.60.80.84 "echo '✅ SSH connection successful!'"
```

If this works, you're ready for the next step!

## Step 4: Get Private Key Content

```bash
# Display the private key (copy the ENTIRE output)
cat ~/.ssh/github_actions_deploy
```

**Important:** Copy everything, including:
```
-----BEGIN OPENSSH PRIVATE KEY-----
[all the lines in between]
-----END OPENSSH PRIVATE KEY-----
```

## Step 5: Add Private Key to GitHub Secrets

1. Go to: https://github.com/NimaLAN74/hosting-base/settings/secrets/actions

2. Find `SSH_PRIVATE_KEY` secret (or create new if it doesn't exist)

3. Click **Update** (or **New repository secret**)

4. Paste the **entire private key** you copied in Step 4

5. Click **Update secret**

## Step 6: Verify Other Secrets

Make sure these secrets are also set correctly:

- **REMOTE_HOST**: `72.60.80.84` (or your actual server IP)
- **REMOTE_USER**: `root`
- **REMOTE_PORT**: `22` (or your SSH port)

## Step 7: Test the Workflow

1. Go to: https://github.com/NimaLAN74/hosting-base/actions

2. Find "Deploy Energy Service to Production" workflow

3. Click **Run workflow** → **Run workflow**

4. Watch the logs - it should now connect successfully!

## Troubleshooting

### "Permission denied" error
- Verify the public key is in `/root/.ssh/authorized_keys` on the server
- Check file permissions: `chmod 600 ~/.ssh/authorized_keys` on server
- Verify private key in GitHub secrets is complete (includes BEGIN and END lines)

### "error in libcrypto" error
- The private key might be corrupted in GitHub secrets
- Re-copy the private key from Step 4
- Make sure you copied the PRIVATE key, not the public key

### "Connection timed out" error
- Verify REMOTE_HOST is correct
- Check if server firewall allows SSH
- Verify REMOTE_PORT is correct

## Security Reminders

✅ **DO:**
- Keep private keys secret
- Use a dedicated key for GitHub Actions
- Use a passphrase for extra security
- Rotate keys periodically

❌ **DON'T:**
- Share private keys
- Commit private keys to git
- Use your personal SSH key
- Post private keys anywhere
