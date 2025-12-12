import { authenticatedFetch } from '../keycloak';

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

export const adminApi = {
  async check() {
    const res = await authenticatedFetch('/api/admin/check');
    if (!res.ok) throw new Error('Admin check failed');
    return res.json();
  },
  async listUsers({ page = 1, size = 20, search = '' } = {}) {
    const res = await authenticatedFetch(`/api/admin/users${buildQuery({ page, size, search })}`);
    if (!res.ok) throw new Error('Failed to fetch users');
    return res.json();
  },
  async getUser(id) {
    const res = await authenticatedFetch(`/api/admin/users/${encodeURIComponent(id)}`);
    if (!res.ok) throw new Error('Failed to fetch user');
    return res.json();
  },
  async createUser(payload) {
    const res = await authenticatedFetch('/api/admin/users', {
      method: 'POST',
      body: JSON.stringify(payload)
    });
    if (!res.ok) throw new Error('Failed to create user');
    return res.json();
  },
  async updateUser(id, payload) {
    const res = await authenticatedFetch(`/api/admin/users/${encodeURIComponent(id)}`, {
      method: 'PUT',
      body: JSON.stringify(payload)
    });
    if (!res.ok) throw new Error('Failed to update user');
    return res.json();
  },
  async deleteUser(id) {
    const res = await authenticatedFetch(`/api/admin/users/${encodeURIComponent(id)}`, {
      method: 'DELETE'
    });
    if (!res.ok) throw new Error('Failed to delete user');
    return res.json();
  },
  async changePassword(id, newPassword) {
    const res = await authenticatedFetch(`/api/admin/users/${encodeURIComponent(id)}/change-password`, {
      method: 'POST',
      body: JSON.stringify({ new_password: newPassword })
    });
    if (!res.ok) throw new Error('Failed to change password');
    return res.json();
  }
};
