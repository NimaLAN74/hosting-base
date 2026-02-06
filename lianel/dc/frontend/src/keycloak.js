// Keycloak initialization and configuration
// Use https://www.lianel.se/auth (same-origin proxy) so login hits www's /auth/ -> Keycloak.
// auth.lianel.se/auth/login/ currently serves Airflow's login page (routing bug); using www.lianel.se/auth avoids that.
import Keycloak from 'keycloak-js';

const getKeycloakUrl = () => {
  const fromEnv = process.env.REACT_APP_KEYCLOAK_URL;
  return fromEnv || 'https://www.lianel.se/auth';
};

const keycloakConfig = {
  url: getKeycloakUrl(),
  realm: process.env.REACT_APP_KEYCLOAK_REALM || 'lianel',
  clientId: process.env.REACT_APP_KEYCLOAK_CLIENT_ID || 'frontend-client'
};

// Initialize Keycloak instance
const keycloak = new Keycloak(keycloakConfig);

// Export keycloak instance for direct access when needed
export { keycloak };

// Keycloak initialization options
// Note: check-sso with iframe won't work cross-domain (auth.lianel.se -> lianel.se).
// X-Frame-Options blocks the iframe, so we disable it and use token-based persistence.
// Include offline_access so Keycloak returns a refresh_token; otherwise updateToken() fails with "no refresh token available"
const initOptions = {
  checkLoginIframe: false,  // DISABLED: X-Frame-Options blocks cross-domain iframe
  enableLogging: true,
  pkceMethod: 'S256',  // REQUIRED: Keycloak 26.4.6 requires PKCE for public clients
  flow: 'standard',
  scope: 'openid profile email offline_access'
};

/**
 * Initialize Keycloak and return a promise
 * Uses localStorage to persist token across page refreshes
 * @returns {Promise<boolean>} True if authenticated, false otherwise
 */
export const initKeycloak = () => {
  return new Promise((resolve, reject) => {
    // Check for callback from Keycloak (authorization code)
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get('code');

    // Initialize Keycloak
    keycloak.init(initOptions)
      .then((authenticated) => {
        // If we have a code, we went through the callback flow
        if (code) {
          console.log('Processing authorization callback - code detected');
          // Clean up the callback URL immediately to prevent issues
          window.history.replaceState({}, '', window.location.pathname);
          
          // Keycloak.js should have already exchanged the code for a token
          // But we need to verify authentication state
          if (keycloak.authenticated && keycloak.token) {
            console.log('Authentication confirmed after callback - token available');
            // Ensure token is stored
            try {
              localStorage.setItem('keycloak_token', keycloak.token);
              localStorage.setItem('keycloak_refresh_token', keycloak.refreshToken);
              localStorage.setItem('keycloak_token_timestamp', Date.now().toString());
            } catch (e) {
              console.warn('Could not store token after callback:', e);
            }
          } else {
            console.warn('Callback detected but authentication not confirmed yet');
            // Wait a bit for Keycloak to finish processing
            setTimeout(() => {
              if (keycloak.authenticated && keycloak.token) {
                console.log('Authentication confirmed after delay');
                try {
                  localStorage.setItem('keycloak_token', keycloak.token);
                  localStorage.setItem('keycloak_refresh_token', keycloak.refreshToken);
                  localStorage.setItem('keycloak_token_timestamp', Date.now().toString());
                } catch (e) {
                  console.warn('Could not store token:', e);
                }
              }
            }, 200);
          }
        }

        const persistTokens = () => {
          try {
            localStorage.setItem('keycloak_token', keycloak.token);
            if (keycloak.refreshToken) {
              localStorage.setItem('keycloak_refresh_token', keycloak.refreshToken);
            }
            localStorage.setItem('keycloak_token_timestamp', Date.now().toString());
          } catch (e) {
            console.warn('Could not persist token in localStorage:', e);
          }
        };

        const refreshIntervalRef = { current: null };
        const startRefreshInterval = () => {
          refreshIntervalRef.current = setInterval(() => {
            keycloak.updateToken(70)
              .then((refreshed) => {
                if (refreshed) {
                  console.log('Token refreshed');
                  persistTokens();
                }
              })
              .catch((error) => {
                console.error('Token refresh failed:', error);
                if (refreshIntervalRef.current) {
                  clearInterval(refreshIntervalRef.current);
                  refreshIntervalRef.current = null;
                }
              });
          }, 60000); // Every 60s so we refresh before 5min expiry
        };

        // Keycloak auto-loaded token from sessionStorage/cookie if available
        if (authenticated && keycloak.token) {
          console.log('User authenticated, token available');
          persistTokens();
          // Refresh immediately so we have at least 70s validity, then start interval
          keycloak.updateToken(70)
            .then((refreshed) => {
              if (refreshed) persistTokens();
            })
            .catch((e) => console.warn('Initial token refresh check:', e));
          startRefreshInterval();
          resolve(true);
        } else {
          // Not authenticated - try restore from localStorage if we have both tokens
          const storedToken = localStorage.getItem('keycloak_token');
          const storedRefresh = localStorage.getItem('keycloak_refresh_token');
          if (storedToken && storedRefresh) {
            console.log('Attempting to restore session from localStorage');
            keycloak.token = storedToken;
            keycloak.refreshToken = storedRefresh;
            keycloak.authenticated = true;
            // Immediately refresh to get a valid access token (handles expired access token)
            keycloak.updateToken(70)
              .then((refreshed) => {
                persistTokens();
                startRefreshInterval();
                console.log('Session restored', refreshed ? '(token refreshed)' : '(token still valid)');
                resolve(true);
              })
              .catch((error) => {
                console.warn('Session restore failed (refresh failed):', error);
                try {
                  localStorage.removeItem('keycloak_token');
                  localStorage.removeItem('keycloak_refresh_token');
                  localStorage.removeItem('keycloak_token_timestamp');
                } catch (e) {}
                keycloak.authenticated = false;
                keycloak.token = null;
                keycloak.refreshToken = null;
                resolve(false);
              });
            return;
          }
          console.log('Not authenticated - no stored session');
          resolve(false);
        }
      })
      .catch((error) => {
        console.error('Keycloak init error:', error);
        resolve(false);
      });
  });
};

/**
 * Login function - redirects to Keycloak login.
 * Uses createLoginUrl() and awaits it so it works when keycloak-js returns a string or a Promise (keycloak-js 26.x).
 * Avoids keycloak.login() which may pass a Promise to location.href when createLoginUrl is async.
 */
export const login = (redirectToCurrentPath = true) => {
  // Clear any stored token to avoid using a stale session when switching users
  try {
    keycloak.clearToken();
    localStorage.removeItem('keycloak_token');
    localStorage.removeItem('keycloak_refresh_token');
    localStorage.removeItem('keycloak_token_timestamp');
  } catch (e) {
    console.warn('Could not clear stored token before login:', e);
  }

  const origin = window.location.origin;
  const pathname = window.location.pathname || '/';
  const search = window.location.search || '';
  const isKeycloakPath = pathname.startsWith('/auth');
  const appPath = isKeycloakPath ? '/' : (pathname + search);
  const redirectUri = redirectToCurrentPath ? (origin + appPath) : (origin + '/');

  const doRedirect = (url) => {
    if (typeof url === 'string' && url) {
      window.location.href = url;
      return;
    }
    const keycloakUrl = `${keycloakConfig.url}/realms/${keycloakConfig.realm}/protocol/openid-connect/auth`;
    const params = new URLSearchParams({
      client_id: keycloakConfig.clientId,
      redirect_uri: redirectUri,
      response_type: 'code',
      scope: 'openid profile email offline_access',
      prompt: 'login'
    });
    window.location.href = `${keycloakUrl}?${params.toString()}`;
  };

  (async () => {
    try {
      const loginUrlOrPromise = keycloak.createLoginUrl({ redirectUri, prompt: 'login', scope: 'openid profile email offline_access' });
      let loginUrl = await Promise.resolve(loginUrlOrPromise);
      // Keep same-origin: if URL points to auth.lianel.se, use www.lianel.se/auth so login runs via www's proxy.
      if (typeof loginUrl === 'string' && loginUrl.includes('auth.lianel.se') && loginUrl.includes('/realms/')) {
        loginUrl = loginUrl.replace(/https:\/\/auth\.lianel\.se/g, 'https://www.lianel.se/auth');
      }
      // Ensure offline_access is in scope so we get a refresh_token (fixes "Unable to update token, no refresh token available")
      if (typeof loginUrl === 'string' && loginUrl.includes('scope=') && !loginUrl.includes('offline_access')) {
        loginUrl = loginUrl.replace(/scope=([^&]+)/, (_, s) => 'scope=' + encodeURIComponent(decodeURIComponent(s).replace(/\s+/g, ' ') + ' offline_access'));
      } else if (typeof loginUrl === 'string' && !loginUrl.includes('scope=')) {
        loginUrl += (loginUrl.includes('?') ? '&' : '?') + 'scope=' + encodeURIComponent('openid profile email offline_access');
      }
      doRedirect(loginUrl);
    } catch (error) {
      console.error('Login error:', error);
      doRedirect(null);
    }
  })();
};

/**
 * Logout function - use Keycloak.js built-in logout for better redirect handling
 */
export const logout = () => {
  const redirectUri = `${window.location.origin}/`;

  // Clear only Keycloak-specific tokens from localStorage
  // Do NOT use localStorage.clear() or sessionStorage.clear() as they wipe all data
  try {
    localStorage.removeItem('keycloak_token');
    localStorage.removeItem('keycloak_refresh_token');
    localStorage.removeItem('keycloak_token_timestamp');
  } catch (e) {
    console.error('Error clearing keycloak tokens:', e);
  }

  // Build explicit logout URL to ensure redirect, avoiding the Keycloak info page
  const logoutUrl = keycloak.createLogoutUrl({ redirectUri });
  window.location.href = logoutUrl;
};

/**
 * Get access token
 * @returns {string|null} Access token or null
 */
export const getToken = () => {
  return keycloak.token;
};

/**
 * Get user info from token
 * @returns {object|null} User info or null
 */
export const getUserInfo = () => {
  if (keycloak.tokenParsed) {
    return {
      id: keycloak.tokenParsed.sub,
      username: keycloak.tokenParsed.preferred_username,
      email: keycloak.tokenParsed.email,
      firstName: keycloak.tokenParsed.given_name,
      lastName: keycloak.tokenParsed.family_name,
      name: keycloak.tokenParsed.name || 
            `${keycloak.tokenParsed.given_name || ''} ${keycloak.tokenParsed.family_name || ''}`.trim() ||
            keycloak.tokenParsed.preferred_username,
      emailVerified: keycloak.tokenParsed.email_verified,
      roles: keycloak.tokenParsed.realm_access?.roles || []
    };
  }
  return null;
};

/**
 * Check if user is authenticated
 * @returns {boolean} True if authenticated
 */
export const isAuthenticated = () => {
  return keycloak.authenticated;
};

/**
 * Check if user has a specific role
 * @param {string} role - Role name
 * @returns {boolean} True if user has the role
 */
export const hasRole = (role) => {
  return keycloak.hasRealmRole(role);
};

/**
 * Get authenticated fetch function that includes access token.
 * On 401, tries to refresh the token once and retries the request.
 * @param {string} url - URL to fetch
 * @param {object} options - Fetch options (headers merged; Content-Type default application/json)
 * @param {boolean} isRetry - Internal: true when this is the retry after refresh
 * @returns {Promise<Response>} Fetch response
 */
export const authenticatedFetch = async (url, options = {}, isRetry = false) => {
  const token = getToken();
  if (!token) {
    throw new Error('Not authenticated');
  }

  const headers = {
    ...options.headers,
    'Authorization': `Bearer ${token}`
  };
  if (!headers['Content-Type'] && options.body instanceof FormData === false) {
    headers['Content-Type'] = 'application/json';
  }

  const res = await fetch(url, { ...options, headers });

  // On 401, try refresh once and retry (avoids re-login when refresh was slightly late)
  if (res.status === 401 && !isRetry && keycloak.refreshToken) {
    try {
      const refreshed = await keycloak.updateToken(70);
      if (refreshed && keycloak.token) {
        try {
          localStorage.setItem('keycloak_token', keycloak.token);
          if (keycloak.refreshToken) localStorage.setItem('keycloak_refresh_token', keycloak.refreshToken);
          localStorage.setItem('keycloak_token_timestamp', Date.now().toString());
        } catch (e) {}
        return authenticatedFetch(url, options, true);
      }
    } catch (e) {
      console.warn('authenticatedFetch: refresh on 401 failed', e);
    }
  }

  return res;
};

export default keycloak;

