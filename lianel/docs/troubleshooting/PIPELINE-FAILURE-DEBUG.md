# Pipeline Failure Debug Guide

## If Pipeline Failed

### 1. Check GitHub Actions Logs
1. Go to: https://github.com/NimaLAN74/hosting-base/actions
2. Click on the failed workflow run
3. Click on the failed job
4. Check the error message in the logs

### 2. Common Build Errors

#### Rust Compilation Errors
- **Missing imports**: Check if all `use` statements are present
- **Syntax errors**: Missing braces `{}`, semicolons, etc.
- **Type mismatches**: Check function signatures

#### Docker Build Errors
- **Dockerfile issues**: Check if Dockerfile syntax is correct
- **Build context**: Ensure all required files are in the build context
- **Platform issues**: Check if `linux/amd64` platform is specified

### 3. Recent Changes That Might Cause Issues

#### Profile Service Token Validation Fix
- Changed `validate_token()` to use userinfo endpoint first
- Added fallback to introspection
- Uses `base64::engine::general_purpose::URL_SAFE_NO_PAD.decode()`

**If build fails with base64 error:**
- Check if `base64 = "0.22"` is in `Cargo.toml`
- Verify `use base64::Engine;` is imported
- Check if base64 0.22 API matches the usage

### 4. Quick Fixes

#### If base64 API changed:
```rust
// Old (if it doesn't work):
base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1])

// Try:
use base64::engine::general_purpose;
let engine = general_purpose::URL_SAFE_NO_PAD;
engine.decode(parts[1])
```

#### If compilation fails:
1. Check the exact error message
2. Look for line numbers in the error
3. Verify syntax around those lines
4. Check if all required fields are present in struct initialization

### 5. Manual Build Test
To test locally (if you have Rust installed):
```bash
cd lianel/dc/profile-service
cargo check
cargo build --release --target x86_64-unknown-linux-musl
```

### 6. Check Workflow File
Verify `.github/workflows/deploy-profile-service.yml`:
- Build context is correct: `./lianel/dc/profile-service`
- Dockerfile path is correct: `./lianel/dc/profile-service/Dockerfile`
- Platform is specified: `linux/amd64`

## Next Steps
1. **Get the actual error message** from GitHub Actions
2. **Share the error** so we can fix it
3. **Check if it's a base64 API issue** or something else

