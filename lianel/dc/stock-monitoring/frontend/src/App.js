import React, { useCallback, useEffect, useMemo, useState } from 'react';

const API_HEALTH = '/api/v1/stock-monitoring/health';
const API_STATUS = '/api/v1/stock-monitoring/status';

const pageStyle = {
  padding: '2rem',
  fontFamily: 'Inter, system-ui, -apple-system, Segoe UI, Roboto, sans-serif',
  maxWidth: '960px',
  margin: '0 auto',
  color: '#1f2937',
};

const rowStyle = {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
  gap: '1rem',
  marginTop: '1rem',
};

const cardStyle = {
  border: '1px solid #e5e7eb',
  borderRadius: '10px',
  padding: '1rem',
  background: '#fff',
  boxShadow: '0 1px 2px rgba(0, 0, 0, 0.04)',
};

const labelStyle = {
  margin: 0,
  fontSize: '0.85rem',
  color: '#6b7280',
};

const valueStyle = {
  margin: '0.35rem 0 0',
  fontSize: '1.1rem',
  fontWeight: 600,
};

const statusPillStyle = (isOk) => ({
  display: 'inline-block',
  padding: '0.3rem 0.6rem',
  borderRadius: '999px',
  fontSize: '0.8rem',
  fontWeight: 700,
  background: isOk ? '#dcfce7' : '#fee2e2',
  color: isOk ? '#166534' : '#991b1b',
});

function App() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [health, setHealth] = useState('');
  const [status, setStatus] = useState(null);
  const [lastUpdated, setLastUpdated] = useState('');

  const loadData = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const [healthRes, statusRes] = await Promise.all([
        fetch(API_HEALTH, { credentials: 'include' }),
        fetch(API_STATUS, { credentials: 'include' }),
      ]);

      if (!healthRes.ok) {
        throw new Error(`Health request failed (${healthRes.status})`);
      }
      if (!statusRes.ok) {
        throw new Error(`Status request failed (${statusRes.status})`);
      }

      const [healthText, statusJson] = await Promise.all([
        healthRes.text(),
        statusRes.json(),
      ]);

      setHealth(healthText.trim());
      setStatus(statusJson);
      setLastUpdated(new Date().toLocaleString());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const isHealthy = useMemo(() => health.toLowerCase() === 'ok', [health]);
  const dbConnected = useMemo(
    () => (status?.database || '').toLowerCase() === 'connected',
    [status]
  );

  return (
    <main style={pageStyle}>
      <h1 style={{ marginBottom: '0.5rem' }}>Stock Exchange Monitoring</h1>
      <p style={{ marginTop: 0, color: '#4b5563' }}>
        Live operational status for the EU markets MVP backend.
      </p>

      <div style={{ marginTop: '1rem', display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
        <button
          type="button"
          onClick={loadData}
          disabled={loading}
          style={{
            padding: '0.5rem 0.9rem',
            borderRadius: '8px',
            border: '1px solid #d1d5db',
            cursor: loading ? 'not-allowed' : 'pointer',
            background: loading ? '#f3f4f6' : '#ffffff',
          }}
        >
          {loading ? 'Refreshing...' : 'Refresh status'}
        </button>
        {lastUpdated && (
          <span style={{ color: '#6b7280', fontSize: '0.9rem' }}>
            Last updated: {lastUpdated}
          </span>
        )}
      </div>

      {error && (
        <div style={{ ...cardStyle, marginTop: '1rem', borderColor: '#fecaca', background: '#fef2f2' }}>
          <p style={{ margin: 0, color: '#991b1b', fontWeight: 600 }}>Unable to load stock service status</p>
          <p style={{ margin: '0.35rem 0 0', color: '#7f1d1d' }}>{error}</p>
        </div>
      )}

      <section style={rowStyle}>
        <article style={cardStyle}>
          <p style={labelStyle}>Backend health</p>
          <p style={valueStyle}>
            <span style={statusPillStyle(isHealthy)}>
              {loading && !health ? 'Checking...' : (health || 'unknown').toUpperCase()}
            </span>
          </p>
        </article>

        <article style={cardStyle}>
          <p style={labelStyle}>Database status</p>
          <p style={valueStyle}>
            <span style={statusPillStyle(dbConnected)}>
              {loading && !status ? 'Checking...' : (status?.database || 'unknown').toUpperCase()}
            </span>
          </p>
        </article>

        <article style={cardStyle}>
          <p style={labelStyle}>Service name</p>
          <p style={valueStyle}>{status?.service || (loading ? 'Loading...' : 'unknown')}</p>
        </article>

        <article style={cardStyle}>
          <p style={labelStyle}>Version</p>
          <p style={valueStyle}>{status?.version || (loading ? 'Loading...' : 'unknown')}</p>
        </article>
      </section>

      <section style={{ ...cardStyle, marginTop: '1rem' }}>
        <p style={labelStyle}>Scope</p>
        <p style={valueStyle}>{status?.scope || (loading ? 'Loading...' : 'unknown')}</p>
      </section>
    </main>
  );
}

export default App;
