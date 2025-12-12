import React, { useEffect, useState } from 'react';
import { adminApi } from './adminApi';
import { Link, useSearchParams, useNavigate } from 'react-router-dom';

const UsersList = () => {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();

  const [users, setUsers] = useState([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [page, setPage] = useState(parseInt(searchParams.get('page') || '1', 10));
  const [size, setSize] = useState(parseInt(searchParams.get('size') || '20', 10));
  const [search, setSearch] = useState(searchParams.get('search') || '');

  const fetchUsers = async (p = page, s = size, q = search) => {
    setLoading(true);
    setError(null);
    try {
      const data = await adminApi.listUsers({ page: p, size: s, search: q });
      setUsers(data.users || []);
      setTotal(data.total || 0);
    } catch (e) {
      setError(e.message || 'Failed to load users');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchUsers(page, size, search);
    // eslint-disable-next-line
  }, []);

  const totalPages = Math.max(1, Math.ceil(total / size));

  const applyFilters = (newPage = page) => {
    const params = new URLSearchParams();
    params.set('page', String(newPage));
    params.set('size', String(size));
    if (search.trim()) params.set('search', search.trim());
    navigate({ pathname: '/admin/users', search: params.toString() });
    fetchUsers(newPage, size, search);
  };

  return (
    <div style={{ maxWidth: 1100, margin: '24px auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <h2 style={{ margin: 0 }}>Users</h2>
        <Link to="/admin/users/new" className="btn-primary" style={{ textDecoration: 'none', padding: '8px 12px', background: '#111', color: '#fff', borderRadius: 6 }}>+ New User</Link>
      </div>

      {/* Filters */}
      <div style={{ display: 'flex', gap: 12, marginBottom: 16 }}>
        <input
          type="text"
          placeholder="Search username, email, name…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') applyFilters(1); }}
          style={{ flex: 1, padding: 8, border: '1px solid #ddd', borderRadius: 6 }}
        />
        <select value={size} onChange={(e) => { const s = parseInt(e.target.value, 10); setSize(s); applyFilters(1); }} style={{ padding: 8, border: '1px solid #ddd', borderRadius: 6 }}>
          <option value={10}>10</option>
          <option value={20}>20</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
        </select>
        <button onClick={() => applyFilters(1)} style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #ddd' }}>Search</button>
      </div>

      {/* Table */}
      <div style={{ border: '1px solid #eee', borderRadius: 8, overflow: 'hidden' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead style={{ background: '#fafafa' }}>
            <tr>
              <th style={{ textAlign: 'left', padding: 12, borderBottom: '1px solid #eee' }}>Username</th>
              <th style={{ textAlign: 'left', padding: 12, borderBottom: '1px solid #eee' }}>Name</th>
              <th style={{ textAlign: 'left', padding: 12, borderBottom: '1px solid #eee' }}>Email</th>
              <th style={{ textAlign: 'left', padding: 12, borderBottom: '1px solid #eee' }}>Status</th>
              <th style={{ textAlign: 'right', padding: 12, borderBottom: '1px solid #eee' }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {loading && (
              <tr><td colSpan={5} style={{ padding: 16 }}>Loading…</td></tr>
            )}
            {error && !loading && (
              <tr><td colSpan={5} style={{ padding: 16, color: 'crimson' }}>{error}</td></tr>
            )}
            {!loading && !error && users.length === 0 && (
              <tr><td colSpan={5} style={{ padding: 16 }}>No users found</td></tr>
            )}
            {!loading && !error && users.map((u) => (
              <tr key={u.id}>
                <td style={{ padding: 12, borderTop: '1px solid #f2f2f2' }}>{u.username}</td>
                <td style={{ padding: 12, borderTop: '1px solid #f2f2f2' }}>{u.name}</td>
                <td style={{ padding: 12, borderTop: '1px solid #f2f2f2' }}>{u.email}</td>
                <td style={{ padding: 12, borderTop: '1px solid #f2f2f2' }}>
                  <span style={{ padding: '2px 6px', borderRadius: 16, background: u.enabled ? '#e6ffed' : '#fff5f5', color: u.enabled ? '#037847' : '#b42318', border: `1px solid ${u.enabled ? '#b7f0c6' : '#f5c2c7'}` }}>
                    {u.enabled ? 'Enabled' : 'Disabled'}
                  </span>
                </td>
                <td style={{ padding: 12, borderTop: '1px solid #f2f2f2', textAlign: 'right' }}>
                  <Link to={`/admin/users/${u.id}`} style={{ marginRight: 8 }}>View</Link>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: 12 }}>
        <div style={{ color: '#666' }}>Total: {total}</div>
        <div style={{ display: 'flex', gap: 8 }}>
          <button disabled={page <= 1} onClick={() => { const np = Math.max(1, page - 1); setPage(np); applyFilters(np); }}>Prev</button>
          <span style={{ alignSelf: 'center' }}>Page {page} / {totalPages}</span>
          <button disabled={page >= totalPages} onClick={() => { const np = Math.min(totalPages, page + 1); setPage(np); applyFilters(np); }}>Next</button>
        </div>
      </div>
    </div>
  );
};

export default UsersList;
