# How to Make GitHub Container Registry Packages Public

## What are "Packages"?

When your GitHub Actions workflow builds and pushes Docker images, they are stored as **"Packages"** in GitHub Container Registry (ghcr.io).

**Your packages are**:
- `ghcr.io/nimalan74/hosting-base/lianel-frontend:latest`
- `ghcr.io/nimalan74/hosting-base/lianel-profile-service:latest`

These packages are currently **PRIVATE**, which means only authenticated users can pull them. That's why the remote host gets "denied" error.

---

## Step-by-Step: Make Packages Public

### Step 1: Go to Your Packages

1. Open your browser
2. Go to: **https://github.com/NimaLAN74?tab=packages**
   - Or: Go to your profile → Click **"Packages"** tab (next to Repositories, Stars, etc.)

### Step 2: Find Your Container Packages

You should see packages like:
- `hosting-base/lianel-frontend`
- `hosting-base/lianel-profile-service`

(They might be listed as container images)

### Step 3: Open Package Settings

For **EACH** package:

1. **Click on the package name** (e.g., `hosting-base/lianel-frontend`)
2. On the package page, look for **"Package settings"** button (usually on the right sidebar)
3. Click **"Package settings"**

### Step 4: Change Visibility to Public

1. Scroll down to the bottom of the settings page
2. Look for **"Danger Zone"** section
3. Find **"Change visibility"** button
4. Click **"Change visibility"**
5. Select **"Public"** (instead of Private)
6. Type the package name to confirm
7. Click **"I understand, change visibility"**

### Step 5: Repeat for Other Package

Do the same for:
- `hosting-base/lianel-profile-service`

---

## Alternative: Direct URL Method

If you can't find the packages tab, try these direct URLs:

**For Frontend Package**:
```
https://github.com/orgs/NimaLAN74/packages/container/hosting-base%2Flianel-frontend/settings
```

**For Backend Package**:
```
https://github.com/orgs/NimaLAN74/packages/container/hosting-base%2Flianel-profile-service/settings
```

Then:
1. Scroll to "Danger Zone"
2. Click "Change visibility"
3. Select "Public"
4. Confirm

---

## What Happens After Making Public?

✅ **Anyone can pull the images** (no authentication needed)
✅ **Remote host can pull without login**
✅ **Pipeline will work automatically**
✅ **No security risk** - images are just Docker layers, not source code

---

## Verify It Worked

After making packages public, test on remote host:

```bash
ssh root@72.60.80.84
docker pull ghcr.io/nimalan74/hosting-base/lianel-frontend:latest
```

If it works without authentication, you're done! ✅

---

## Still Having Issues?

If you can't find the packages:

1. **Check if packages exist**: Go to https://github.com/NimaLAN74/hosting-base/actions
2. **Look for successful workflow runs** that built images
3. **Check the workflow logs** - they should show the package URL

Or check via GitHub CLI:
```bash
gh api user/packages?package_type=container
```

---

**Need Help?** If you still can't find the packages, let me know and I'll help you locate them!

