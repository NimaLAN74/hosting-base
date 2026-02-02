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

  async processRequest(prompt, framework = null, messages = null) {
    try {
      const body = { prompt };
      if (framework && String(framework).trim() !== '') {
        body.framework = String(framework).trim();
      }
      if (messages && Array.isArray(messages) && messages.length > 0) {
        body.messages = messages;
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
  },

  // Phase 4: Controls & Evidence
  async getControls() {
    const res = await authenticatedFetch('/api/v1/controls');
    if (!res.ok) throw new Error('Failed to fetch controls');
    return res.json();
  },

  async getControl(id) {
    const res = await authenticatedFetch(`/api/v1/controls/${id}`);
    if (!res.ok) {
      if (res.status === 404) throw new Error('Control not found');
      throw new Error('Failed to fetch control');
    }
    return res.json();
  },

  async getEvidence({ control_id, limit = 50, offset = 0 } = {}) {
    const res = await authenticatedFetch(
      `/api/v1/evidence${buildQuery({ control_id, limit, offset })}`
    );
    if (!res.ok) throw new Error('Failed to fetch evidence');
    return res.json();
  },

  async postEvidence(body) {
    const res = await authenticatedFetch('/api/v1/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error('Failed to add evidence');
    return res.json();
  },

  async postGitHubEvidence(body) {
    const res = await authenticatedFetch('/api/v1/integrations/github/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to collect GitHub evidence');
    }
    return res.json();
  },
};
