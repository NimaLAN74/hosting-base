# 🔓 Make Packages Public - Manual Steps

## ⚠️ Issue

Making the **repository** public does **NOT** automatically make **container packages** public. They must be made public separately.

## ✅ Solution: Make Packages Public via GitHub UI

### Step 1: Go to Your Packages

1. Go to: https://github.com/NimaLAN74?tab=packages
2. Or navigate: GitHub → Your Profile → Packages

### Step 2: Find Each Package

Look for these packages:
- `hosting-base/lianel-frontend`
- `hosting-base/lianel-profile-service`

### Step 3: Make Each Package Public

For **each package**:

1. Click on the package name
2. Click **Package settings** (on the right sidebar)
3. Scroll down to **Danger Zone**
4. Click **Change visibility**
5. Select **Public**
6. Type the package name to confirm
7. Click **I understand, change visibility**

### Step 4: Verify

After making packages public, test on remote host:

```bash
ssh root@72.60.80.84 "docker pull ghcr.io/nimalan74/hosting-base/lianel-frontend:latest"
```

Should work without authentication!

---

## 🔧 Alternative: Fix Workflow to Make Packages Public Automatically

The workflow has a step to make packages public, but it might need fixing. Let me check and update it.

