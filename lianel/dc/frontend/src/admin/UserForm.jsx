import React, { useState } from 'react';
import { adminApi } from './adminApi';
import PageTemplate from '../PageTemplate';

const empty = { username: '', email: '', firstName: '', lastName: '', password: '', enabled: true };

const UserForm = ({ mode = 'create' }) => {
  const [form, setForm] = useState(empty);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState(null);

  const onChange = (e) => {
    const { name, value, type, checked } = e.target;
    setForm((f) => ({ ...f, [name]: type === 'checkbox' ? checked : value }));
  };

  const onSubmit = async (e) => {
    e.preventDefault();
    setSaving(true);
    setMessage(null);
    try {
      if (mode === 'create') {
        await adminApi.createUser({
          username: form.username.trim(),
          email: form.email.trim(),
          firstName: form.firstName.trim() || undefined,
          lastName: form.lastName.trim() || undefined,
          password: form.password,
          enabled: !!form.enabled
        });
        setMessage('User created successfully');
        setForm(empty);
      }
    } catch (e) {
      setMessage(e.message || 'Failed to save user');
    } finally {
      setSaving(false);
    }
  };

  return (
    <PageTemplate title={mode === 'create' ? 'Create User' : 'Edit User'}>
      <div style={{ maxWidth: 700, margin: '0 auto' }}>
      {message && <div style={{ marginBottom: 12 }}>{message}</div>}
      <form onSubmit={onSubmit}>
        <div style={{ display: 'grid', gap: 12 }}>
          <label>
            <div>Username</div>
            <input name="username" value={form.username} onChange={onChange} required style={{ width: '100%', padding: 8 }} />
          </label>
          <label>
            <div>Email</div>
            <input name="email" type="email" value={form.email} onChange={onChange} required style={{ width: '100%', padding: 8 }} />
          </label>
          <div style={{ display: 'flex', gap: 12 }}>
            <label style={{ flex: 1 }}>
              <div>First Name</div>
              <input name="firstName" value={form.firstName} onChange={onChange} style={{ width: '100%', padding: 8 }} />
            </label>
            <label style={{ flex: 1 }}>
              <div>Last Name</div>
              <input name="lastName" value={form.lastName} onChange={onChange} style={{ width: '100%', padding: 8 }} />
            </label>
          </div>
          <label>
            <div>Password</div>
            <input name="password" type="password" value={form.password} onChange={onChange} required minLength={8} style={{ width: '100%', padding: 8 }} />
          </label>
          <label style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <input name="enabled" type="checkbox" checked={form.enabled} onChange={onChange} /> Enabled
          </label>
        </div>
        <div style={{ marginTop: 16 }}>
          <button type="submit" disabled={saving} style={{ padding: '8px 12px' }}>{saving ? 'Saving…' : 'Save'}</button>
        </div>
      </form>
      </div>
    </PageTemplate>
  );
};

export default UserForm;
