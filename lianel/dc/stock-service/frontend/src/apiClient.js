import { getToken } from './keycloak';

const STOCK_API_BASE = '/api/v1/stock-service';

function withBasePath(path) {
  if (!path) {
    return STOCK_API_BASE;
  }
  return `${STOCK_API_BASE}${path.startsWith('/') ? path : `/${path}`}`;
}

async function parseErrorBody(response) {
  try {
    const payload = await response.json();
    if (payload && typeof payload.error === 'string' && payload.error.trim()) {
      const detail = typeof payload.detail === 'string' && payload.detail.trim()
        ? `: ${payload.detail}`
        : '';
      return `${payload.error}${detail}`;
    }
  } catch {
    // Ignore parse failures and fall back to status text.
  }
  return `${response.status} ${response.statusText || 'Request failed'}`;
}

export class ApiError extends Error {
  constructor(message, status) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

export async function apiFetch(path, options = {}) {
  const headers = {
    Accept: 'application/json',
    ...(options.headers || {}),
  };
  const token = getToken();
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
  return fetch(withBasePath(path), {
    credentials: 'include',
    ...options,
    headers,
  });
}

export async function apiJson(path, options = {}) {
  const response = await apiFetch(path, options);
  if (!response.ok) {
    throw new ApiError(await parseErrorBody(response), response.status);
  }
  if (response.status === 204) {
    return null;
  }
  const contentType = response.headers.get('content-type') || '';
  if (contentType.includes('application/json')) {
    return response.json();
  }
  return response.text();
}

/** Return the current path for use as returnToPath after login (e.g. /stock or /stock/watchlists). */
export function getLoginReturnPath() {
  const path = typeof window !== 'undefined' ? window.location.pathname : '/stock';
  return path && path.startsWith('/stock') ? path : '/stock';
}

/** Use Keycloak login (same SSO as main app). Call login(returnToPath) to redirect; no URL string. */
export { login as getLoginRedirect, logout as getLogoutRedirect } from './keycloak';

