# SSH Key Setup for GitHub Actions

**Generated**: $(date)  
**Purpose**: GitHub Actions deployment to remote host

---

## 📋 Public Key (Add to Remote Host)

Copy this public key and add it to the remote host's `~/.ssh/authorized_keys`:

```bash
# On remote host (72.60.80.84)
ssh root@72.60.80.84

# Add the public key
echo "PUBLIC_KEY_HERE" >> ~/.ssh/authorized_keys
chmod 600 ~/.ssh/authorized_keys
chmod 700 ~/.ssh
```

**Or use ssh-copy-id**:
```bash
ssh-copy-id -i ~/.ssh/github_actions_deploy.pub root@72.60.80.84
```

---

## 🔐 Private Key (Add to GitHub Secrets)

Copy the entire private key (including BEGIN/END lines) and add it to GitHub:

1. Go to: https://github.com/NimaLAN74/hosting-base/settings/secrets/actions
2. Click **New repository secret**
3. Name: `SSH_PRIVATE_KEY`
4. Value: Paste the entire private key content
5. Click **Add secret**

---

## ✅ Verification

After adding the keys:

1. **Test SSH connection**:
   ```bash
   ssh -i ~/.ssh/github_actions_deploy root@72.60.80.84 "echo 'Connection successful'"
   ```

2. **Verify in GitHub Actions**:
   - The workflow should now be able to SSH without password
   - Check the "Test SSH Connection" step in workflow logs

---

## 🔒 Security Notes

- ✅ Private key is stored in GitHub Secrets (encrypted)
- ✅ Public key is on remote host (read-only access)
- ✅ Key is dedicated for GitHub Actions (not your personal key)
- ⚠️  Keep private key secure - never commit to repository

---

**Status**: Key generated - follow instructions above to complete setup

