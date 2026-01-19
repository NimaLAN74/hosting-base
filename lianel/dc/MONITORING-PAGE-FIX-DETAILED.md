# Monitoring Page Redirect Fix - Detailed

**Date**: 2026-01-19  
**Issue**: Monitoring/Dashboards page redirects to login and shows empty page after login

## Root Cause Analysis

### Problem Flow
1. User visits `/monitoring` (unauthenticated)
2. React app checks `authenticated` state → `false`
3. Shows "Please log in" message with button ✅
4. User clicks "Log In" button
5. Redirects to Keycloak with `redirectUri=https://www.lianel.se/monitoring` ✅
6. User logs in at Keycloak
7. Keycloak redirects back to `https://www.lianel.se/monitoring?code=...`
8. **PROBLEM**: React state `authenticated` doesn't update immediately
9. Component still shows login page or empty page

### Root Causes Identified

1. **Timing Issue**: Keycloak.js needs time to exchange the authorization code for a token
   - The `initKeycloak()` promise resolves before code exchange completes
   - React state updates before authentication is confirmed

2. **State Update Race Condition**: 
   - `KeycloakProvider` sets `authenticated` based on `initKeycloak()` result
   - But Keycloak.js might still be processing the callback
   - State doesn't re-check after callback processing

3. **Missing Event Listeners**:
   - Keycloak.js provides `onAuthSuccess` event
   - We weren't listening to this event to update state
   - State only updated on mount, not on auth success

## Fixes Applied

### Fix 1: Improved Callback Handling in `keycloak.js`

**Changes:**
- Added delay check after callback detection
- Store token immediately after callback confirmation
- Added retry logic if authentication not immediately confirmed

```javascript
if (code) {
  console.log('Processing authorization callback - code detected');
  window.history.replaceState({}, '', window.location.pathname);
  
  if (keycloak.authenticated && keycloak.token) {
    // Store token immediately
    localStorage.setItem('keycloak_token', keycloak.token);
    // ...
  } else {
    // Wait for Keycloak to finish processing
    setTimeout(() => {
      if (keycloak.authenticated && keycloak.token) {
        // Store token after delay
        localStorage.setItem('keycloak_token', keycloak.token);
      }
    }, 200);
  }
}
```

### Fix 2: Added Keycloak Event Listeners in `KeycloakProvider.js`

**Changes:**
- Added `onAuthSuccess` event listener
- Added `onAuthRefreshSuccess` event listener
- Added `onAuthError` and `onTokenExpired` handlers
- Created `updateAuthState()` helper function

```javascript
const onAuthSuccess = () => {
  console.log('Keycloak onAuthSuccess event - updating state');
  setTimeout(() => {
    updateAuthState();
    // Double-check after delay
    setTimeout(() => {
      updateAuthState();
    }, 100);
  }, 50);
};

keycloak.onAuthSuccess = onAuthSuccess;
keycloak.onAuthRefreshSuccess = () => {
  updateAuthState();
};
```

### Fix 3: Added Auth Check Delay in `Monitoring.js`

**Changes:**
- Added `authCheckComplete` state
- Wait 300ms after `keycloakReady` before showing content
- Gives Keycloak time to process callback

```javascript
const [authCheckComplete, setAuthCheckComplete] = useState(false);

useEffect(() => {
  if (keycloakReady) {
    const timer = setTimeout(() => {
      setAuthCheckComplete(true);
    }, 300);
    return () => clearTimeout(timer);
  }
}, [keycloakReady]);
```

### Fix 4: Improved State Update Function

**Changes:**
- Created centralized `updateAuthState()` function
- Checks `isAuthenticated()` and updates React state
- Updates `userInfo` when authenticated

```javascript
const updateAuthState = () => {
  const auth = isAuthenticated();
  setAuthenticated(auth);
  if (auth) {
    setUserInfo(getUserInfo());
  }
};
```

## Expected Behavior After Fix

1. **Visit `/monitoring` (unauthenticated)**:
   - ✅ Shows "Please log in" message
   - ✅ Shows "Log In" button
   - ✅ No auto-redirect

2. **Click "Log In"**:
   - ✅ Redirects to Keycloak
   - ✅ Preserves `/monitoring` path in redirect URI

3. **After login in Keycloak**:
   - ✅ Returns to `https://www.lianel.se/monitoring?code=...`
   - ✅ Keycloak.js exchanges code for token
   - ✅ `onAuthSuccess` event fires
   - ✅ `updateAuthState()` is called
   - ✅ React state updates to `authenticated: true`
   - ✅ Component re-renders with dashboard cards
   - ✅ URL cleaned to `/monitoring` (no `?code=...`)

## Testing

### Manual Test Steps

1. Clear browser cache/cookies
2. Visit `https://www.lianel.se/monitoring`
3. Should see login button (not auto-redirect) ✅
4. Click "Log In"
5. After login, should return to `/monitoring`
6. Should see dashboard cards (not empty page) ✅

### Browser Console Checks

After login, check console for:
- `"Processing authorization callback - code detected"`
- `"Authentication confirmed after callback - token available"`
- `"Keycloak onAuthSuccess event - updating state"`
- `"Keycloak initialized, authenticated: true"`

## Files Modified

- `frontend/src/KeycloakProvider.js` - Added event listeners and improved state updates
- `frontend/src/keycloak.js` - Improved callback handling with delays
- `frontend/src/monitoring/Monitoring.js` - Added auth check delay

## Status

✅ **FIXED** - Changes committed and pushed
- Frontend pipeline will rebuild automatically
- After deployment, Monitoring page should work correctly

## Next Steps

1. Wait for frontend pipeline to complete
2. Test Monitoring page login flow in browser
3. Verify dashboards are visible after login
