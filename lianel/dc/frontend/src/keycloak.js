// Keycloak initialization and configuration
import Keycloak from 'keycloak-js';

const keycloakConfig = {
  url: 'https://auth.lianel.se',
  realm: 'lianel',
  clientId: 'frontend-client',
  redirectUri: window.location.origin + window.location.pathname
};

// Initialize Keycloak instance
const keycloak = new Keycloak(keycloakConfig);

// Keycloak initialization options
// Only used when we have an authorization code (callback)
// For initial page load, we skip check-sso entirely and redirect directly to login
const initOptions = {
  // Don't set onLoad - this means no auto-login
  // Valid values are: 'login-required' or 'check-sso', but we want neither
  checkLoginIframe: false,
  enableLogging: true,
  pkceMethod: 'S256',
  flow: 'standard',
  redirectUri: window.location.origin + window.location.pathname
};

/**
 * Initialize Keycloak and return a promise
 * @returns {Promise<boolean>} True if authenticated, false otherwise
 */
export const initKeycloak = () => {
  return new Promise((resolve, reject) => {
    // Check if we're coming from Keycloak callback (has code in URL)
    // Keycloak returns code as query parameter (?code=...) not hash (#code=...)
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get('code');
    
    // Clear any stale session data
    sessionStorage.removeItem('KEYCLOAK_AUTH_DATA');
    sessionStorage.removeItem('KEYCLOAK_TOKEN');
    localStorage.removeItem('KEYCLOAK_AUTH_DATA');
    
    if (code) {
      // We have an authorization code from Keycloak callback - initialize normally
      console.log('Found authorization code, initializing Keycloak...');
      keycloak.init(initOptions)
        .then((authenticated) => {
          if (authenticated) {
            console.log('User authenticated successfully');
            // Clear the code from URL to prevent re-processing
            const newUrl = window.location.pathname;
            window.history.replaceState({}, '', newUrl);
            // Set up token refresh
            setInterval(() => {
              keycloak.updateToken(70)
                .then((refreshed) => {
                  if (refreshed) console.log('Token refreshed');
                })
                .catch(() => console.error('Failed to refresh token'));
            }, 60000);
            resolve(true);
          } else {
            console.log('Not authenticated after callback');
            resolve(false);
          }
        })
        .catch((error) => {
          console.error('Keycloak init error:', error);
          resolve(false);
        });
    } else {
      // No authorization code - just initialize without auto-login
      keycloak.init(initOptions)
        .then((authenticated) => {
          if (authenticated) {
            console.log('User already authenticated');
            // Set up token refresh
            setInterval(() => {
              keycloak.updateToken(70)
                .then((refreshed) => {
                  if (refreshed) console.log('Token refreshed');
                })
                .catch(() => console.error('Failed to refresh token'));
            }, 60000);
            resolve(true);
          } else {
            // Not authenticated - just resolve false (no auto-redirect)
            console.log('Not authenticated, waiting for user action...');
            resolve(false);
          }
        })
        .catch((error) => {
          console.error('Keycloak init error:', error);
          // On error, just resolve false - user can click login button
          resolve(false);
        });
    }
  });
};

/**
 * Login function - redirects to Keycloak login
 */
export const login = () => {
  // Use Keycloak's built-in login method which handles PKCE automatically
  return keycloak.login({
    redirectUri: window.location.origin + window.location.pathname,
    prompt: 'login'  // Force re-authentication
  });
};

/**
 * Logout function - clears Keycloak session and redirects to landing page
 * ROOT CAUSE FIX: keycloak-js sends "post_logout_redirect_uri" parameter,
 * but Keycloak 26.4.6 expects "redirect_uri". We construct the URL manually.
 */
export const logout = () => {
  const redirectUri = window.location.origin === 'https://www.lianel.se' 
    ? 'https://www.lianel.se/' 
    : 'https://lianel.se/';
  
  // Clear frontend state immediately
  try {
    keycloak.clearToken();
    sessionStorage.clear();
    localStorage.clear();
  } catch (e) {
    console.error('Error clearing storage:', e);
  }
  
  // Construct logout URL with correct parameter name (redirect_uri, not post_logout_redirect_uri)
  const logoutUrl = `${keycloakConfig.url}/realms/${keycloakConfig.realm}/protocol/openid-connect/logout?redirect_uri=${encodeURIComponent(redirectUri)}`;
  
  console.log('Logging out with correct parameter: redirect_uri');
  
  // Redirect to logout endpoint - Keycloak will invalidate the session
  // and redirect back to our landing page
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

