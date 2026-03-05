/**
 * Stock monitoring API client – uses main app Keycloak (same pattern as compAiApi).
 * All requests go to /api/v1/stock-monitoring with the main app's token.
 */
import { authenticatedFetch } from '../keycloak';

const STOCK_API_BASE = '/api/v1/stock-monitoring';

function withBasePath(path) {
  if (!path) return STOCK_API_BASE;
  return `${STOCK_API_BASE}${path.startsWith('/') ? path : `/${path}`}`;
}

async function parseErrorBody(response) {
  try {
    const payload = await response.json();
    if (payload && typeof payload.error === 'string' && payload.error.trim()) {
      const detail = typeof payload.detail === 'string' && payload.detail.trim() ? `: ${payload.detail}` : '';
      return `${payload.error}${detail}`;
    }
  } catch {
    // ignore
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
  const url = withBasePath(path);
  const headers = {
    Accept: 'application/json',
    ...(options.headers || {}),
  };
  return authenticatedFetch(url, { ...options, headers, credentials: 'include' });
}

export async function apiJson(path, options = {}) {
  const response = await apiFetch(path, options);
  if (!response.ok) {
    throw new ApiError(await parseErrorBody(response), response.status);
  }
  if (response.status === 204) return null;
  const contentType = response.headers.get('content-type') || '';
  if (contentType.includes('application/json')) return response.json();
  return response.text();
}

/** Return path for post-login redirect: current path if under /stock, else /stock. */
export function getLoginReturnPath() {
  if (typeof window === 'undefined') return '/stock';
  const path = window.location.pathname || '';
  return path.startsWith('/stock') ? path : '/stock';
}
