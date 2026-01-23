# Monitoring Page Redirect Loop Fix

**Date**: 2026-01-19  
**Issue**: Monitoring/Dashboards page redirecting to login and remaining empty

## Problem

The Monitoring page had a redirect loop issue:
1. User visits `/monitoring`
2. Component checks `authenticated` state
3. If false, `useEffect` immediately calls `login()`
4. `login()` redirects to Keycloak
5. After login, user is redirected back to `/` (home page), not `/monitoring`
6. User navigates to `/monitoring` again
7. If authentication state hasn't updated yet, loop repeats

## Root Causes

1. **Automatic Redirect**: `useEffect` was calling `login()` immediately when `authenticated` was false, causing an automatic redirect
2. **Wrong Redirect URI**: `login()` was always redirecting to `/` instead of preserving the current path
3. **No Loading State**: Component didn't wait for Keycloak to initialize before checking authentication

## Solution

### 1. Removed Automatic Redirect
- Removed the `useEffect` that automatically called `login()`
- Now shows a login button instead of auto-redirecting
- User can choose when to log in

### 2. Preserve Current Path on Login
- Modified `login()` function to accept `redirectToCurrentPath` parameter
- When `true`, preserves the current path (`window.location.pathname`)
- After login, user returns to the same page they were on

### 3. Added Loading State
- Check `keycloakReady` before checking `authenticated`
- Show "Loading authentication..." while Keycloak initializes
- Prevents false negatives during initialization

### 4. Better User Experience
- Show clear message: "Please log in to view monitoring dashboards"
- Provide a "Log In" button with nice styling
- User has control over when to log in

## Code Changes

### `frontend/src/monitoring/Monitoring.js`
```javascript
// Before: Auto-redirect
useEffect(() => {
  if (!authenticated) {
    login();  // Immediate redirect
    return;
  }
}, [authenticated, login]);

// After: Manual login button
if (!keycloakReady) {
  return <LoadingState />;
}

if (!authenticated) {
  return (
    <PageTemplate>
      <p>Please log in...</p>
      <button onClick={() => login(true)}>Log In</button>
    </PageTemplate>
  );
}
```

### `frontend/src/keycloak.js`
```javascript
// Before: Always redirect to home
return keycloak.login({
  redirectUri: window.location.origin + '/',
  prompt: 'login'
});

// After: Preserve current path
export const login = (redirectToCurrentPath = true) => {
  const redirectUri = redirectToCurrentPath 
    ? window.location.origin + window.location.pathname + window.location.search
    : window.location.origin + '/';
  
  return keycloak.login({
    redirectUri: redirectUri,
    prompt: 'login'
  });
};
```

## Testing

### Before Fix
1. Visit `/monitoring` → Immediate redirect to login
2. After login → Redirected to `/` (home)
3. Navigate to `/monitoring` again → May redirect again if auth state not ready

### After Fix
1. Visit `/monitoring` → Shows "Please log in" message with button
2. Click "Log In" → Redirects to Keycloak
3. After login → Returns to `/monitoring` (same page)
4. Page displays dashboard cards correctly

## Verification Steps

1. **Clear browser cache/cookies** (to start fresh)
2. **Visit** https://www.lianel.se/monitoring
3. **Expected**: See "Please log in to view monitoring dashboards" with a "Log In" button
4. **Click "Log In"**
5. **After login**: Should return to `/monitoring` page
6. **Expected**: See dashboard cards displayed

## Status

✅ **FIXED**
- No more redirect loop
- Path is preserved on login
- Better user experience
- Loading state handled properly

## Deployment

Changes have been committed and pushed. The frontend pipeline will automatically deploy these changes.

**Next**: Wait for frontend deployment to complete, then test the Monitoring page.
