# SSH Key Troubleshooting - "ssh-ed25519" Hostname Error

## 🔴 Error Analysis

**Error**: `ssh: Could not resolve hostname ssh-ed25519: Temporary failure in name resolution`

This error suggests that **"ssh-ed25519"** is being interpreted as a hostname instead of a key type identifier.

### Possible Causes:

1. **SSH_PRIVATE_KEY secret is malformed**:
   - Contains "ssh-ed25519" as text instead of actual key
   - Missing `-----BEGIN` header
   - Incomplete key content

2. **SSH command construction issue**:
   - Key file path is wrong
   - SSH is reading wrong file
   - Key file contains wrong content

3. **Key format issue**:
   - Key type identifier in wrong place
   - Extra text before/after key

---

## ✅ Solution: Verify SSH_PRIVATE_KEY Secret

### Step 1: Get the Correct Private Key

The private key should be from `~/.ssh/id_ed25519_host`:

```bash
cat ~/.ssh/id_ed25519_host
```

### Step 2: Verify Key Format

The key should:
- ✅ Start with: `-----BEGIN OPENSSH PRIVATE KEY-----`
- ✅ End with: `-----END OPENSSH PRIVATE KEY-----`
- ✅ Contain base64-encoded content in between
- ✅ Be complete (all lines included)

### Step 3: Update GitHub Secret

1. Go to: https://github.com/NimaLAN74/hosting-base/settings/secrets/actions
2. Find `SSH_PRIVATE_KEY` secret
3. Click "Update"
4. Copy the **ENTIRE** key content from `~/.ssh/id_ed25519_host`
5. Paste it (make sure no extra text before/after)
6. Save

### Step 4: Verify Secret Content

The secret should look like:
```
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1lZDI1NTE5AAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACCZCYbSxgdaMlOoR6R9pchX7CyX8phSY8tjycdj5ysdHwAAAJjGGQDlxhkA
...
-----END OPENSSH PRIVATE KEY-----
```

**NOT**:
- ❌ Just "ssh-ed25519" text
- ❌ Public key content
- ❌ Incomplete key
- ❌ Extra text before/after

---

## 🔧 Workflow Fixes Applied

I've updated the workflows to:
1. ✅ Use `printf` for better multiline handling
2. ✅ Strict validation (must start with `-----BEGIN`)
3. ✅ Check key ends with `-----END`
4. ✅ Show key content preview on error
5. ✅ Better error messages

---

## 🧪 Test on Remote Host

The remote host is ready:
- ✅ SSH works
- ✅ Docker ready
- ✅ Network exists
- ✅ Compose files valid
- ✅ Images available

The issue is **only** in the SSH_PRIVATE_KEY secret format in GitHub.

---

## 📝 Next Steps

1. **Verify SSH_PRIVATE_KEY secret** contains complete private key
2. **Check workflow logs** - new validation will show key content if wrong
3. **Pipeline should work** once key is correct

---

**Status**: Workflow validation improved - will show exact key content if malformed

