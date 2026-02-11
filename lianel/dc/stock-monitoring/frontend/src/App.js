import React, { useCallback, useEffect, useMemo, useState } from 'react';
import './App.css';

const API_HEALTH = '/api/v1/stock-monitoring/health';
const API_STATUS = '/api/v1/stock-monitoring/status';
const MAX_HISTORY_ITEMS = 20;

function App() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [health, setHealth] = useState('');
  const [status, setStatus] = useState(null);
  const [metrics, setMetrics] = useState({
    healthMs: null,
    statusMs: null,
    healthCode: null,
    statusCode: null,
  });
  const [lastUpdated, setLastUpdated] = useState('');
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [showRaw, setShowRaw] = useState(false);
  const [history, setHistory] = useState([]);

  const loadData = useCallback(async ({ silent = false } = {}) => {
    if (!silent) {
      setLoading(true);
    }
    setError('');
    try {
      const timedFetch = async (url) => {
        const start = performance.now();
        const response = await fetch(url, { credentials: 'include' });
        return {
          response,
          durationMs: Math.round(performance.now() - start),
        };
      };

      const [healthResult, statusResult] = await Promise.allSettled([
        timedFetch(API_HEALTH),
        timedFetch(API_STATUS),
      ]);

      const nextMetrics = {
        healthMs: null,
        statusMs: null,
        healthCode: null,
        statusCode: null,
      };
      const issues = [];

      let fetchedHealth = '';
      let fetchedStatus = null;

      if (healthResult.status === 'fulfilled') {
        const { response, durationMs } = healthResult.value;
        nextMetrics.healthCode = response.status;
        nextMetrics.healthMs = durationMs;
        if (!response.ok) {
          issues.push(`Health request failed (${response.status})`);
        } else {
          const healthText = await response.text();
          fetchedHealth = healthText.trim();
          setHealth(fetchedHealth);
        }
      } else {
        issues.push('Health request failed (network error)');
      }

      if (statusResult.status === 'fulfilled') {
        const { response, durationMs } = statusResult.value;
        nextMetrics.statusCode = response.status;
        nextMetrics.statusMs = durationMs;
        if (!response.ok) {
          issues.push(`Status request failed (${response.status})`);
        } else {
          const statusJson = await response.json();
          fetchedStatus = statusJson;
          setStatus(statusJson);
        }
      } else {
        issues.push('Status request failed (network error)');
      }

      setMetrics(nextMetrics);
      const sampledAt = new Date();
      setLastUpdated(sampledAt.toLocaleString());

      const historyItem = {
        sampledAt: sampledAt.toISOString(),
        healthText: fetchedHealth || 'unknown',
        dbText: fetchedStatus?.database || 'unknown',
        healthCode: nextMetrics.healthCode,
        statusCode: nextMetrics.statusCode,
        healthMs: nextMetrics.healthMs,
        statusMs: nextMetrics.statusMs,
        errorText: issues.join(' | '),
      };
      setHistory((prev) => [historyItem, ...prev].slice(0, MAX_HISTORY_ITEMS));

      if (issues.length > 0) {
        setError(issues.join(' | '));
      }
    } catch (err) {
      const errorText = err instanceof Error ? err.message : 'Unknown error';
      setError(errorText);
      const sampledAt = new Date();
      setLastUpdated(sampledAt.toLocaleString());
      setHistory((prev) => [
        {
          sampledAt: sampledAt.toISOString(),
          healthText: 'unknown',
          dbText: 'unknown',
          healthCode: null,
          statusCode: null,
          healthMs: null,
          statusMs: null,
          errorText,
        },
        ...prev,
      ].slice(0, MAX_HISTORY_ITEMS));
    } finally {
      if (!silent) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEffect(() => {
    if (!autoRefresh) {
      return undefined;
    }

    const intervalId = setInterval(() => {
      loadData({ silent: true });
    }, 30000);

    return () => clearInterval(intervalId);
  }, [autoRefresh, loadData]);

  const isHealthy = useMemo(() => health.toLowerCase() === 'ok', [health]);
  const dbConnected = useMemo(
    () => (status?.database || '').toLowerCase() === 'connected',
    [status]
  );

  return (
    <div className="App stock-app">
      <div className="container">
        <header className="header">
          <a href="/" className="logo">
            <div className="logo-icon">LW</div>
            Lianel World
          </a>
          <div className="header-right">
            <a href="/profile" className="profile-link" aria-label="Open profile">
              <span className="profile-avatar">U</span>
              <span>Profile</span>
            </a>
          </div>
        </header>

        <main className="main">
          <div className="page-header">
            <a href="/" className="back-to-home-btn">← Back to Home</a>
            <h1 className="page-title">Stock Exchange Monitoring</h1>
            <p className="page-subtitle">Live operational status for the EU markets MVP backend.</p>
            <div className="quick-links">
              <a href="/services">All Services</a>
              <a href="/profile">Profile</a>
              <a href="/stock-monitoring/endpoints">Endpoints</a>
            </div>
          </div>

          <div className="status-toolbar">
            <button
              type="button"
              onClick={() => loadData()}
              disabled={loading}
              className="refresh-btn"
            >
              {loading ? 'Refreshing...' : 'Refresh status'}
            </button>
            <label className="auto-refresh-toggle">
              <input
                type="checkbox"
                checked={autoRefresh}
                onChange={(event) => setAutoRefresh(event.target.checked)}
              />
              Auto-refresh (30s)
            </label>
            {lastUpdated && (
              <span className="last-updated">Last updated: {lastUpdated}</span>
            )}
          </div>

          {error && (
            <div className="stock-error">
              <p className="stock-error-title">Unable to load stock service status</p>
              <p>{error}</p>
            </div>
          )}

          <section className="status-grid">
            <article className="status-card">
              <p className="status-label">Backend health</p>
              <p className="status-value">
                <span className={`status-pill ${isHealthy ? 'ok' : 'bad'}`}>
                  {loading && !health ? 'Checking...' : (health || 'unknown').toUpperCase()}
                </span>
              </p>
            </article>

            <article className="status-card">
              <p className="status-label">Database status</p>
              <p className="status-value">
                <span className={`status-pill ${dbConnected ? 'ok' : 'bad'}`}>
                  {loading && !status ? 'Checking...' : (status?.database || 'unknown').toUpperCase()}
                </span>
              </p>
            </article>

            <article className="status-card">
              <p className="status-label">Service name</p>
              <p className="status-value">{status?.service || (loading ? 'Loading...' : 'unknown')}</p>
            </article>

            <article className="status-card">
              <p className="status-label">Version</p>
              <p className="status-value">{status?.version || (loading ? 'Loading...' : 'unknown')}</p>
            </article>

            <article className="status-card">
              <p className="status-label">Health endpoint</p>
              <p className="status-value">
                {metrics.healthCode ? `HTTP ${metrics.healthCode}` : 'n/a'}
              </p>
              <p className="status-meta">
                {metrics.healthMs !== null ? `${metrics.healthMs} ms` : 'n/a'}
              </p>
            </article>

            <article className="status-card">
              <p className="status-label">Status endpoint</p>
              <p className="status-value">
                {metrics.statusCode ? `HTTP ${metrics.statusCode}` : 'n/a'}
              </p>
              <p className="status-meta">
                {metrics.statusMs !== null ? `${metrics.statusMs} ms` : 'n/a'}
              </p>
            </article>
          </section>

          <section className="status-card scope-card">
            <p className="status-label">Scope</p>
            <p className="status-value">{status?.scope || (loading ? 'Loading...' : 'unknown')}</p>
          </section>

          <section className="status-card scope-card">
            <div className="raw-header">
              <p className="status-label">Raw status JSON</p>
              <button
                type="button"
                className="raw-toggle-btn"
                onClick={() => setShowRaw((current) => !current)}
              >
                {showRaw ? 'Hide' : 'Show'}
              </button>
            </div>
            {showRaw && (
              <pre className="raw-json">{JSON.stringify(status || {}, null, 2)}</pre>
            )}
          </section>

          <section className="status-card scope-card">
            <div className="raw-header">
              <p className="status-label">Recent checks</p>
              <span className="history-count">{history.length} / {MAX_HISTORY_ITEMS}</span>
            </div>
            {history.length === 0 ? (
              <p className="status-meta">No checks yet.</p>
            ) : (
              <div className="history-table-wrapper">
                <table className="history-table">
                  <thead>
                    <tr>
                      <th>Time</th>
                      <th>Health</th>
                      <th>DB</th>
                      <th>Health API</th>
                      <th>Status API</th>
                      <th>Issue</th>
                    </tr>
                  </thead>
                  <tbody>
                    {history.map((item) => (
                      <tr key={`${item.sampledAt}-${item.healthCode}-${item.statusCode}`}>
                        <td>{new Date(item.sampledAt).toLocaleTimeString()}</td>
                        <td>{item.healthText.toUpperCase()}</td>
                        <td>{String(item.dbText).toUpperCase()}</td>
                        <td>
                          {item.healthCode ? `HTTP ${item.healthCode}` : 'n/a'}
                          {item.healthMs !== null ? ` (${item.healthMs}ms)` : ''}
                        </td>
                        <td>
                          {item.statusCode ? `HTTP ${item.statusCode}` : 'n/a'}
                          {item.statusMs !== null ? ` (${item.statusMs}ms)` : ''}
                        </td>
                        <td>{item.errorText || '-'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        </main>

        <footer className="footer">
          <p>&copy; 2026 Lianel World. All rights reserved.</p>
        </footer>
      </div>
    </div>
  );
}

export default App;
