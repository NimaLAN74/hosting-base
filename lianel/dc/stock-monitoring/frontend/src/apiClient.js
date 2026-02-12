const STOCK_API_BASE = '/api/v1/stock-monitoring';

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

export function getLoginUrl(returnToPath = '/stock') {
  const rd = encodeURIComponent(returnToPath || '/stock');
  return `/oauth2/start?rd=${rd}`;
}

export function getLogoutUrl(returnToPath = '/stock') {
  const rd = encodeURIComponent(returnToPath || '/stock');
  return `/oauth2/sign_out?rd=${rd}`;
}

