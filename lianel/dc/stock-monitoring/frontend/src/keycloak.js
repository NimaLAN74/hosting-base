// Keycloak for stock-monitoring UI – same realm as main app (www.lianel.se) so SSO works.
// Use https://www.lianel.se/auth so login hits the same proxy as the main frontend.
import Keycloak from 'keycloak-js';

/** Base64-decode string (browser atob or Node Buffer). */
function base64Decode(str) {
  if (typeof atob !== 'undefined') return atob(str);
  if (typeof Buffer !== 'undefined') return Buffer.from(str, 'base64').toString('utf8');
  return null;
}

/** Return JWT exp (seconds since epoch) or null if unreadable. */
function getTokenExp(token) {
  if (!token || typeof token !== 'string') return null;
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const payload = parts[1].replace(/-/g, '+').replace(/_/g, '/');
    const decoded = base64Decode(payload);
    if (!decoded) return null;
    const obj = JSON.parse(decoded);
    return typeof obj.exp === 'number' ? obj.exp : null;
  } catch {
    return null;
  }
}

/** True if token exists and expires in more than minSeconds. */
function isTokenValidFor(token, minSeconds = 60) {
  const exp = getTokenExp(token);
  return exp != null && exp > Math.floor(Date.now() / 1000) + minSeconds;
}

const getKeycloakUrl = () => {
  const fromEnv = process.env.REACT_APP_KEYCLOAK_URL;
  return fromEnv || 'https://www.lianel.se/auth';
};

const keycloakConfig = {
  url: getKeycloakUrl(),
  realm: process.env.REACT_APP_KEYCLOAK_REALM || 'lianel',
  clientId: process.env.REACT_APP_KEYCLOAK_CLIENT_ID || 'frontend-client',
};

const keycloak = new Keycloak(keycloakConfig);

const initOptions = {
  checkLoginIframe: false,
  enableLogging: process.env.NODE_ENV === 'development',
  pkceMethod: 'S256',
  flow: 'standard',
  scope: 'openid profile email offline_access',
};

/** Apply token from localStorage to keycloak instance so getToken() works. Returns true if a valid token was applied. */
function restoreFromLocalStorage() {
  try {
    const storedToken = localStorage.getItem('keycloak_token');
    const storedRefresh = localStorage.getItem('keycloak_refresh_token');
    if (!storedToken || !isTokenValidFor(storedToken, 0)) return false;
    keycloak.token = storedToken;
    keycloak.refreshToken = storedRefresh || null;
    keycloak.authenticated = true;
    return true;
  } catch {
    return false;
  }
}

export const initKeycloak = () => {
  return new Promise((resolve) => {
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get('code');

    // Use token from main app immediately if present (same origin = same localStorage)
    if (restoreFromLocalStorage()) {
      resolve(true);
      return;
    }

    keycloak.init(initOptions)
      .then((authenticated) => {
        const persistTokens = () => {
          try {
            if (keycloak.token) {
              localStorage.setItem('keycloak_token', keycloak.token);
              if (keycloak.refreshToken) {
                localStorage.setItem('keycloak_refresh_token', keycloak.refreshToken);
              }
              localStorage.setItem('keycloak_token_timestamp', Date.now().toString());
            }
          } catch (e) {
            console.warn('Could not persist token:', e);
          }
        };

        if (code) {
          window.history.replaceState({}, '', window.location.pathname);
        }

        if (authenticated && keycloak.token) {
          persistTokens();
          keycloak.updateToken(70).then(() => persistTokens()).catch(() => {});
          resolve(true);
        } else {
          const storedToken = localStorage.getItem('keycloak_token');
          const storedRefresh = localStorage.getItem('keycloak_refresh_token');
          if (storedToken) {
            keycloak.token = storedToken;
            keycloak.refreshToken = storedRefresh || null;
            keycloak.authenticated = true;
            if (storedRefresh) {
              keycloak.updateToken(70)
                .then((refreshed) => {
                  persistTokens();
                  resolve(true);
                })
                .catch(() => {
                  if (isTokenValidFor(storedToken, 60)) {
                    persistTokens();
                    resolve(true);
                  } else {
                    try {
                      localStorage.removeItem('keycloak_token');
                      localStorage.removeItem('keycloak_refresh_token');
                      localStorage.removeItem('keycloak_token_timestamp');
                    } catch (e) {}
                    keycloak.authenticated = false;
                    keycloak.token = null;
                    keycloak.refreshToken = null;
                    resolve(false);
                  }
                });
            } else {
              if (isTokenValidFor(storedToken, 60)) {
                persistTokens();
                resolve(true);
              } else {
                try { localStorage.removeItem('keycloak_token'); localStorage.removeItem('keycloak_token_timestamp'); } catch (e) {}
                keycloak.authenticated = false;
                keycloak.token = null;
                resolve(false);
              }
            }
            return;
          }
          resolve(false);
        }
      })
      .catch((err) => {
        console.warn('Keycloak init error (trying localStorage):', err);
        if (restoreFromLocalStorage()) resolve(true);
        else resolve(false);
      });
  });
};

/** Redirect to Keycloak login; returnToPath is the path to return to after login (e.g. /stock or /stock/watchlists). */
export const login = (returnToPath = '/stock') => {
  try {
    keycloak.clearToken();
    localStorage.removeItem('keycloak_token');
    localStorage.removeItem('keycloak_refresh_token');
    localStorage.removeItem('keycloak_token_timestamp');
  } catch (e) {
    console.warn('Could not clear token before login:', e);
  }

  const origin = typeof window !== 'undefined' ? window.location.origin : '';
  const path = (returnToPath && String(returnToPath).startsWith('/')) ? returnToPath : `/stock${returnToPath ? `/${returnToPath}` : ''}`;
  const redirectUri = `${origin}${path}`;

  (async () => {
    try {
      const loginUrlOrPromise = keycloak.createLoginUrl?.({ redirectUri, prompt: 'login', scope: 'openid profile email offline_access' });
      let url = await Promise.resolve(loginUrlOrPromise);
      if (typeof url === 'string' && url.includes('auth.lianel.se') && url.includes('/realms/')) {
        url = url.replace(/https:\/\/auth\.lianel\.se/g, 'https://www.lianel.se/auth');
      }
      if (typeof url === 'string' && url) {
        window.location.href = url;
        return;
      }
    } catch (e) {
      console.warn('createLoginUrl failed, using manual URL', e);
    }
    const authUrl = `${keycloakConfig.url}/realms/${keycloakConfig.realm}/protocol/openid-connect/auth`;
    const params = new URLSearchParams({
      client_id: keycloakConfig.clientId,
      redirect_uri: redirectUri,
      response_type: 'code',
      scope: 'openid profile email offline_access',
      prompt: 'login',
    });
    window.location.href = `${authUrl}?${params.toString()}`;
  })();
};

export const logout = (returnToPath = '/stock') => {
  try {
    localStorage.removeItem('keycloak_token');
    localStorage.removeItem('keycloak_refresh_token');
    localStorage.removeItem('keycloak_token_timestamp');
  } catch (e) {
    console.error('Error clearing keycloak tokens:', e);
  }
  const origin = window.location.origin;
  const path = (returnToPath && returnToPath.startsWith('/')) ? returnToPath : '/stock';
  const redirectUri = `${origin}${path}`;
  const logoutUrl = keycloak.createLogoutUrl({ redirectUri });
  window.location.href = logoutUrl;
};

export const getToken = () => keycloak.token || null;

export const getUserInfo = () => {
  if (keycloak.tokenParsed) {
    const p = keycloak.tokenParsed;
    return {
      id: p.sub,
      username: p.preferred_username,
      email: p.email,
      name: p.name || [p.given_name, p.family_name].filter(Boolean).join(' ') || p.preferred_username,
    };
  }
  return null;
};

export const isAuthenticated = () => !!keycloak.authenticated;

/** Fetch with Bearer token; on 401 tries refresh once and retries. */
export const authenticatedFetch = async (url, options = {}, isRetry = false) => {
  const token = getToken();
  if (!token) {
    throw new Error('Not authenticated');
  }
  const headers = {
    ...options.headers,
    Authorization: `Bearer ${token}`,
  };
  if (!headers['Content-Type'] && !(options.body instanceof FormData)) {
    headers['Content-Type'] = 'application/json';
  }
  const res = await fetch(url, { ...options, headers });
  if (res.status === 401 && !isRetry && keycloak.refreshToken) {
    try {
      const refreshed = await keycloak.updateToken(70);
      if (refreshed && keycloak.token) {
        try {
          localStorage.setItem('keycloak_token', keycloak.token);
          if (keycloak.refreshToken) localStorage.setItem('keycloak_refresh_token', keycloak.refreshToken);
        } catch (e) {}
        return authenticatedFetch(url, options, true);
      }
    } catch (e) {
      console.warn('authenticatedFetch: refresh on 401 failed', e);
    }
  }
  return res;
};

export { keycloak };
export default keycloak;
