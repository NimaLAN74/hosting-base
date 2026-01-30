import { authenticatedFetch } from '../keycloak.js';

const buildQuery = (params = {}) => {
  const q = new URLSearchParams();
  Object.entries(params).forEach(([k, v]) => {
    if (v !== undefined && v !== null && String(v).trim() !== '') {
      q.set(k, String(v));
    }
  });
  const qs = q.toString();
  return qs ? `?${qs}` : '';
};

export const compAiApi = {
  async getHealth() {
    // Health endpoint is public, no auth required
    const res = await fetch('/api/v1/comp-ai/health');
    if (!res.ok) throw new Error('Health check failed');
    return res.json();
  },

  async getFrameworks() {
    const res = await fetch('/api/v1/comp-ai/frameworks');
    if (!res.ok) throw new Error('Failed to fetch frameworks');
    return res.json();
  },

  async processRequest(prompt, framework = null) {
    try {
      const body = { prompt };
      if (framework && String(framework).trim() !== '') {
        body.framework = String(framework).trim();
      }
      const res = await authenticatedFetch('/api/v1/comp-ai/process', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(body),
      });
      if (!res.ok) {
        if (res.status === 429) {
          const data = await res.json().catch(() => ({}));
          const secs = data.retry_after_secs || 60;
          throw new Error(`Too many requests. Please try again in ${secs} seconds.`);
        }
        const errorText = await res.text().catch(() => 'Unknown error');
        throw new Error(`Failed to process request: ${res.status} ${res.statusText} - ${errorText}`);
      }
      return res.json();
    } catch (err) {
      if (err.message.includes('Not authenticated') || err.message.includes('Unauthorized')) {
        throw new Error('Authentication required. Please log in again.');
      }
      throw err;
    }
  },

  async getRequestHistory({ limit = 50, offset = 0 } = {}) {
    try {
      const res = await authenticatedFetch(
        `/api/v1/comp-ai/history${buildQuery({ limit, offset })}`
      );
      if (!res.ok) {
        if (res.status === 429) {
          const data = await res.json().catch(() => ({}));
          const secs = data.retry_after_secs || 60;
          throw new Error(`Too many requests. Please try again in ${secs} seconds.`);
        }
        const errorText = await res.text().catch(() => 'Unknown error');
        throw new Error(`Failed to fetch request history: ${res.status} ${res.statusText} - ${errorText}`);
      }
      return res.json();
    } catch (err) {
      if (err.message.includes('Not authenticated') || err.message.includes('Unauthorized')) {
        throw new Error('Authentication required. Please log in again.');
      }
      throw err;
    }
  }
};
