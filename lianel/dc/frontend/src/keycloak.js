// Keycloak initialization and configuration
import Keycloak from 'keycloak-js';

// Get configuration from environment variables (REACT_APP_ prefix required)
const keycloakConfig = {
  url: process.env.REACT_APP_KEYCLOAK_URL || 'https://auth.lianel.se',
  realm: process.env.REACT_APP_KEYCLOAK_REALM || 'lianel',
  clientId: process.env.REACT_APP_KEYCLOAK_CLIENT_ID || 'frontend-client'
};

// Initialize Keycloak instance
const keycloak = new Keycloak(keycloakConfig);

// Export keycloak instance for direct access when needed
export { keycloak };

// Keycloak initialization options
// Note: check-sso with iframe won't work cross-domain (auth.lianel.se -> lianel.se)
// X-Frame-Options blocks the iframe, so we disable it and use token-based persistence
const initOptions = {
  checkLoginIframe: false,  // DISABLED: X-Frame-Options blocks cross-domain iframe
  enableLogging: true,
  pkceMethod: 'S256',  // REQUIRED: Keycloak 26.4.6 requires PKCE for public clients
  flow: 'standard'
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

        // Keycloak auto-loaded token from sessionStorage/cookie if available
        if (authenticated && keycloak.token) {
          console.log('User authenticated, token available');
          // Store token in localStorage for persistence across sessions
          try {
            localStorage.setItem('keycloak_token', keycloak.token);
            localStorage.setItem('keycloak_refresh_token', keycloak.refreshToken);
            localStorage.setItem('keycloak_token_timestamp', Date.now().toString());
          } catch (e) {
            console.warn('Could not store token in localStorage:', e);
          }
          
          // Set up token refresh every minute
          const refreshInterval = setInterval(() => {
            keycloak.updateToken(70)
              .then((refreshed) => {
                if (refreshed) {
                  console.log('Token refreshed');
                  // Update stored token
                  try {
                    localStorage.setItem('keycloak_token', keycloak.token);
                    localStorage.setItem('keycloak_refresh_token', keycloak.refreshToken);
                    localStorage.setItem('keycloak_token_timestamp', Date.now().toString());
                  } catch (e) {
                    console.warn('Could not update token in localStorage:', e);
                  }
                }
              })
              .catch((error) => {
                console.error('Token refresh failed:', error);
                clearInterval(refreshInterval);
              });
          }, 60000);
          
          resolve(true);
        } else {
          // Not authenticated - check if we have a stored token
          const storedToken = localStorage.getItem('keycloak_token');
          const tokenTimestamp = localStorage.getItem('keycloak_token_timestamp');
          const now = Date.now();
          const tokenAge = tokenTimestamp ? (now - parseInt(tokenTimestamp)) / 1000 : Infinity;
          
          // Token valid if less than 5 minutes old (typically expires in 5 min)
          if (storedToken && tokenAge < 300) {
            console.log('Restoring token from localStorage (age: ' + Math.round(tokenAge) + 's)');
            // Manually restore the token
            keycloak.token = storedToken;
            keycloak.refreshToken = localStorage.getItem('keycloak_refresh_token') || storedToken;
            keycloak.authenticated = true;
            
            // Verify token is still valid by checking expiry
            try {
              const parts = storedToken.split('.');
              if (parts.length === 3) {
                const payload = JSON.parse(atob(parts[1]));
                const exp = payload.exp * 1000; // Convert to ms
                if (exp > now) {
                  console.log('Restored token is valid, expires in', Math.round((exp - now) / 1000), 'seconds');
                  // Set up token refresh
                  setInterval(() => {
                    keycloak.updateToken(70)
                      .then((refreshed) => {
                        if (refreshed) {
                          console.log('Token refreshed');
                          try {
                            localStorage.setItem('keycloak_token', keycloak.token);
                          } catch (e) {
                            console.warn('Could not update token:', e);
                          }
                        }
                      })
                      .catch((error) => console.error('Token refresh failed:', error));
                  }, 60000);
                  resolve(true);
                  return;
                }
              }
            } catch (e) {
              console.warn('Could not verify stored token:', e);
            }
          }
          
          console.log('Not authenticated - no valid token available');
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
 * Login function - redirects to Keycloak login
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
  
  // Preserve current path so user returns to the same page after login
  const redirectUri = redirectToCurrentPath 
    ? window.location.origin + window.location.pathname + window.location.search
    : window.location.origin + '/';
  
  console.log('Login: redirecting to Keycloak with redirectUri:', redirectUri);
  
  try {
    // Use Keycloak's createLoginUrl to build the URL explicitly
    // This ensures we always redirect to Keycloak, not /login
    const loginUrl = keycloak.createLoginUrl({
      redirectUri: redirectUri,
      prompt: 'login'  // Force re-authentication
    });
    
    console.log('Login: Keycloak login URL:', loginUrl);
    
    // Explicitly redirect to Keycloak
    window.location.href = loginUrl;
  } catch (error) {
    console.error('Login error:', error);
    // Fallback: try keycloak.login() if createLoginUrl fails
    try {
      keycloak.login({
        redirectUri: redirectUri,
        prompt: 'login'
      });
    } catch (fallbackError) {
      console.error('Login fallback error:', fallbackError);
      // Last resort: redirect to Keycloak manually
      const keycloakUrl = `${keycloakConfig.url}/realms/${keycloakConfig.realm}/protocol/openid-connect/auth`;
      const params = new URLSearchParams({
        client_id: keycloakConfig.clientId,
        redirect_uri: redirectUri,
        response_type: 'code',
        scope: 'openid profile email',
        prompt: 'login'
      });
      window.location.href = `${keycloakUrl}?${params.toString()}`;
    }
  }
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
 * Get authenticated fetch function that includes access token
 * @param {string} url - URL to fetch
 * @param {object} options - Fetch options
 * @returns {Promise<Response>} Fetch response
 */
export const authenticatedFetch = async (url, options = {}) => {
  const token = getToken();
  if (!token) {
    throw new Error('Not authenticated');
  }

  const headers = {
    ...options.headers,
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  };

  return fetch(url, {
    ...options,
    headers
  });
};

export default keycloak;

