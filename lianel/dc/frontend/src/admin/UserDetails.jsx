import React, { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { adminApi } from './adminApi';

const UserDetails = () => {
  const { id } = useParams();
  const [user, setUser] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const data = await adminApi.getUser(id);
        if (mounted) setUser(data);
      } catch (e) {
        if (mounted) setError(e.message || 'Failed to load user');
      } finally {
        if (mounted) setLoading(false);
      }
    })();
    return () => { mounted = false; };
  }, [id]);

  if (loading) return <div style={{ padding: 24 }}>Loading…</div>;
  if (error) return <div style={{ padding: 24, color: 'crimson' }}>{error}</div>;
  if (!user) return <div style={{ padding: 24 }}>User not found</div>;

  return (
    <div style={{ maxWidth: 700, margin: '24px auto' }}>
      <h2 style={{ marginTop: 0 }}>User Details</h2>
      <div style={{ border: '1px solid #eee', borderRadius: 8, padding: 16 }}>
        <div style={{ marginBottom: 8 }}><strong>Username:</strong> {user.username}</div>
        <div style={{ marginBottom: 8 }}><strong>Name:</strong> {user.name}</div>
        <div style={{ marginBottom: 8 }}><strong>Email:</strong> {user.email}</div>
        <div style={{ marginBottom: 8 }}><strong>Status:</strong> {user.enabled ? 'Enabled' : 'Disabled'}</div>
      </div>
    </div>
  );
};

export default UserDetails;
