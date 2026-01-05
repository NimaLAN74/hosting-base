# 🔧 SSH Key Fix Guide - "ssh-ed25519" Hostname Error

## 🔴 Error

```
ssh: Could not resolve hostname ssh-ed25519: Temporary failure in name resolution
```

## 🎯 Root Cause

This error means the `SSH_PRIVATE_KEY` secret in GitHub Actions contains **"ssh-ed25519"** as text instead of the actual private key content.

**"ssh-ed25519"** is the **key type identifier** (like "RSA" or "ECDSA"), not the key itself. SSH is trying to use it as a hostname, which fails.

## ✅ Solution

### Step 1: Get Your Private Key

On your local machine, get the **private key** (not the public key):

```bash
cat ~/.ssh/id_ed25519_host
```

**The private key should look like:**
```
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1lZDI1NTE5AAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACCZCYbSxgdaMlOoR6R9pchX7CyX8phSY8tjycdj5ysdHwAAAJjGGQDlxhkA
...
(many more lines of base64-encoded content)
...
-----END OPENSSH PRIVATE KEY-----
```

### Step 2: Update GitHub Secret

1. Go to: https://github.com/NimaLAN74/hosting-base/settings/secrets/actions
2. Find `SSH_PRIVATE_KEY` secret
3. Click **"Update"**
4. **Delete everything** in the secret value
5. **Copy the ENTIRE private key** from Step 1 (including `-----BEGIN` and `-----END` lines)
6. **Paste it** into the secret value
7. **Save**

### Step 3: Verify Secret Content

The secret should:
- ✅ Start with: `-----BEGIN OPENSSH PRIVATE KEY-----`
- ✅ End with: `-----END OPENSSH PRIVATE KEY-----`
- ✅ Contain many lines of base64-encoded content in between
- ✅ Be at least 100+ bytes

**It should NOT:**
- ❌ Contain just "ssh-ed25519" text
- ❌ Be the public key (starts with "ssh-ed25519 AAAAC3...")
- ❌ Be empty or very short

## 🔍 How to Tell Private vs Public Key

### Private Key (CORRECT - what you need)
```
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1lZDI1NTE5AAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
...
-----END OPENSSH PRIVATE KEY-----
```

### Public Key (WRONG - don't use this)
```
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKuvlzes6DIEapH3ULT2KuHDFM/cOGaz6WiGOXm74sS8 nima_ms1@msn.com
```

## 🧪 Test After Fixing

After updating the secret, the next workflow run will:
1. ✅ Validate the key format
2. ✅ Show clear error if still wrong
3. ✅ Connect successfully if correct

## 📝 Workflow Validation

I've updated the workflows to:
- ✅ Check for "ssh-ed25519" text (catches this exact error)
- ✅ Validate key starts with `-----BEGIN`
- ✅ Validate key ends with `-----END`
- ✅ Check minimum file size
- ✅ Show helpful error messages

If the secret is still wrong, the workflow will now show:
```
❌ Error: SSH key contains 'ssh-ed25519' as text
This looks like a public key, not a private key!
```

---

**Fix the secret, and the deployment will work!** ✅

