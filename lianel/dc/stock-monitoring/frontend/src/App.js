import React, { useCallback, useEffect, useMemo, useState } from 'react';
import './App.css';

const API_HEALTH = '/api/v1/stock-monitoring/health';
const API_STATUS = '/api/v1/stock-monitoring/status';
const MAX_HISTORY_ITEMS = 20;
const WARN_LATENCY_MS = 800;
const CRITICAL_LATENCY_MS = 2000;
const WATCHLIST_STORAGE_KEY = 'stock_monitoring_watchlist_v1';
const DEFAULT_WATCHLIST = ['ASML.AS', 'SAP.DE', 'SHEL.L'];

function evaluateSeverity({
  healthText,
  dbText,
  healthCode,
  statusCode,
  healthMs,
  statusMs,
  errorText,
}) {
  const healthOk = String(healthText || '').toLowerCase() === 'ok';
  const dbOk = String(dbText || '').toLowerCase() === 'connected';
  const healthCodeOk = typeof healthCode === 'number' && healthCode >= 200 && healthCode < 300;
  const statusCodeOk = typeof statusCode === 'number' && statusCode >= 200 && statusCode < 300;
  const maxLatency = Math.max(healthMs ?? 0, statusMs ?? 0);

  if (errorText || !healthOk || !dbOk || !healthCodeOk || !statusCodeOk || maxLatency > CRITICAL_LATENCY_MS) {
    return 'critical';
  }
  if (maxLatency > WARN_LATENCY_MS) {
    return 'warning';
  }
  return 'ok';
}

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
  const [copied, setCopied] = useState(false);
  const [watchlist, setWatchlist] = useState(() => {
    try {
      const raw = localStorage.getItem(WATCHLIST_STORAGE_KEY);
      if (!raw) {
        return DEFAULT_WATCHLIST;
      }
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        return DEFAULT_WATCHLIST;
      }
      const sanitized = parsed
        .map((item) => String(item).toUpperCase().trim())
        .filter(Boolean);
      return sanitized.length > 0 ? sanitized : DEFAULT_WATCHLIST;
    } catch {
      return DEFAULT_WATCHLIST;
    }
  });
  const [symbolInput, setSymbolInput] = useState('');
  const [watchlistError, setWatchlistError] = useState('');

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
      const severity = evaluateSeverity(historyItem);
      setHistory((prev) => [{ ...historyItem, severity }, ...prev].slice(0, MAX_HISTORY_ITEMS));

      if (issues.length > 0) {
        setError(issues.join(' | '));
      }
    } catch (err) {
      const errorText = err instanceof Error ? err.message : 'Unknown error';
      setError(errorText);
      const sampledAt = new Date();
      setLastUpdated(sampledAt.toLocaleString());
      const historyItem = {
          sampledAt: sampledAt.toISOString(),
          healthText: 'unknown',
          dbText: 'unknown',
          healthCode: null,
          statusCode: null,
          healthMs: null,
          statusMs: null,
          errorText,
      };
      const severity = evaluateSeverity(historyItem);
      setHistory((prev) => [{ ...historyItem, severity }, ...prev].slice(0, MAX_HISTORY_ITEMS));
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

  useEffect(() => {
    localStorage.setItem(WATCHLIST_STORAGE_KEY, JSON.stringify(watchlist));
  }, [watchlist]);

  const isHealthy = useMemo(() => health.toLowerCase() === 'ok', [health]);
  const dbConnected = useMemo(
    () => (status?.database || '').toLowerCase() === 'connected',
    [status]
  );
  const latestCheck = history[0];
  const consecutiveCritical = useMemo(() => {
    let count = 0;
    for (const item of history) {
      if (item.severity === 'critical') {
        count += 1;
      } else {
        break;
      }
    }
    return count;
  }, [history]);
  const hasIncident = consecutiveCritical >= 3;
  const hasWarningStreak = !hasIncident && history.length > 0 && history[0].severity === 'warning';

  const copyDiagnostics = useCallback(async () => {
    const payload = {
      generatedAt: new Date().toISOString(),
      thresholds: {
        warningLatencyMs: WARN_LATENCY_MS,
        criticalLatencyMs: CRITICAL_LATENCY_MS,
      },
      latest: {
        lastUpdated,
        health,
        status,
        metrics,
        severity: latestCheck?.severity || 'unknown',
      },
      recentChecks: history,
    };
    const text = JSON.stringify(payload, null, 2);
    try {
      if (navigator?.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const textarea = document.createElement('textarea');
        textarea.value = text;
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
      }
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch (err) {
      setError(err instanceof Error ? `Copy failed: ${err.message}` : 'Copy failed');
    }
  }, [health, history, lastUpdated, latestCheck?.severity, metrics, status]);

  const addSymbol = useCallback(() => {
    const normalized = symbolInput.toUpperCase().trim();
    setWatchlistError('');
    if (!normalized) {
      setWatchlistError('Enter a symbol first.');
      return;
    }
    if (!/^[A-Z0-9._-]{1,20}$/.test(normalized)) {
      setWatchlistError('Use letters/numbers and . _ - only (max 20 chars).');
      return;
    }
    if (watchlist.includes(normalized)) {
      setWatchlistError('Symbol already exists in your watchlist.');
      return;
    }
    setWatchlist((prev) => [normalized, ...prev].slice(0, 50));
    setSymbolInput('');
  }, [symbolInput, watchlist]);

  const removeSymbol = useCallback((symbolToRemove) => {
    setWatchlist((prev) => prev.filter((item) => item !== symbolToRemove));
  }, []);

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
            <button type="button" className="copy-btn" onClick={copyDiagnostics}>
              {copied ? 'Copied!' : 'Copy diagnostics'}
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

          {(hasIncident || hasWarningStreak) && (
            <div className={`incident-banner ${hasIncident ? 'critical' : 'warning'}`}>
              {hasIncident ? (
                <strong>
                  Incident: {consecutiveCritical} consecutive critical checks. Investigate backend/API immediately.
                </strong>
              ) : (
                <strong>
                  Warning: latest check breached SLA threshold ({WARN_LATENCY_MS}+ms).
                </strong>
              )}
            </div>
          )}

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
              <p className="status-label">Watchlist (P0.2 MVP)</p>
              <span className="history-count">{watchlist.length} symbols</span>
            </div>
            <div className="watchlist-form">
              <input
                type="text"
                value={symbolInput}
                onChange={(event) => setSymbolInput(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault();
                    addSymbol();
                  }
                }}
                className="symbol-input"
                placeholder="Add symbol (e.g. SAN.MC)"
                aria-label="Add symbol to watchlist"
              />
              <button type="button" className="raw-toggle-btn" onClick={addSymbol}>
                Add symbol
              </button>
            </div>
            {watchlistError && <p className="watchlist-error">{watchlistError}</p>}
            <div className="watchlist-chips">
              {watchlist.map((symbol) => (
                <div className="watchlist-chip" key={symbol}>
                  <span>{symbol}</span>
                  <button
                    type="button"
                    onClick={() => removeSymbol(symbol)}
                    aria-label={`Remove ${symbol}`}
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
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
                      <tr
                        key={`${item.sampledAt}-${item.healthCode}-${item.statusCode}`}
                        className={`history-row-${item.severity || 'ok'}`}
                      >
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
