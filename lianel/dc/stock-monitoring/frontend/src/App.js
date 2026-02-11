import React, { useCallback, useEffect, useMemo, useState } from 'react';
import './App.css';

const API_HEALTH = '/api/v1/stock-monitoring/health';
const API_STATUS = '/api/v1/stock-monitoring/status';
const MAX_HISTORY_ITEMS = 20;
const WARN_LATENCY_MS = 800;
const CRITICAL_LATENCY_MS = 2000;
const ALERTS_STORAGE_KEY = 'stock_monitoring_alerts_v1';
const NOTIFICATIONS_STORAGE_KEY = 'stock_monitoring_notifications_v1';
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
  const [watchlists, setWatchlists] = useState([]);
  const [activeWatchlistId, setActiveWatchlistId] = useState(null);
  const [watchlistItems, setWatchlistItems] = useState([]);
  const [symbolInput, setSymbolInput] = useState('');
  const [watchlistError, setWatchlistError] = useState('');
  const [prices, setPrices] = useState({});
  const [quotesAsOf, setQuotesAsOf] = useState(null);
  const [quotesError, setQuotesError] = useState('');
  const [alerts, setAlerts] = useState(() => {
    try {
      const raw = localStorage.getItem(ALERTS_STORAGE_KEY);
      const parsed = raw ? JSON.parse(raw) : [];
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  });
  const [notifications, setNotifications] = useState(() => {
    try {
      const raw = localStorage.getItem(NOTIFICATIONS_STORAGE_KEY);
      const parsed = raw ? JSON.parse(raw) : [];
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  });
  const [alertSymbol, setAlertSymbol] = useState('');
  const [alertDirection, setAlertDirection] = useState('above');
  const [alertTarget, setAlertTarget] = useState('');
  const [alertError, setAlertError] = useState('');
  const [routePath, setRoutePath] = useState(() => window.location.pathname);

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

  const fetchQuotes = useCallback(async () => {
    const symbols = watchlistItems.map((item) => String(item.symbol).toUpperCase()).filter(Boolean);
    if (symbols.length === 0) {
      setPrices({});
      setQuotesError('');
      return;
    }
    try {
      const params = new URLSearchParams({ symbols: symbols.join(',') });
      const response = await fetch(`/api/v1/stock-monitoring/quotes?${params.toString()}`, {
        credentials: 'include',
      });
      if (!response.ok) {
        throw new Error(`Quotes request failed (${response.status})`);
      }
      const payload = await response.json();
      const map = {};
      for (const item of payload.quotes || []) {
        if (item?.symbol && typeof item?.price === 'number') {
          map[String(item.symbol).toUpperCase()] = item.price;
        }
      }
      setPrices(map);
      setQuotesAsOf(payload?.as_of_ms || null);
      setQuotesError('');
    } catch (err) {
      setQuotesError(err instanceof Error ? err.message : 'Failed to load quotes');
    }
  }, [watchlistItems]);

  const loadWatchlistItems = useCallback(async (watchlistId) => {
    const response = await fetch(`/api/v1/stock-monitoring/watchlists/${watchlistId}/items`, {
      credentials: 'include',
    });
    if (!response.ok) {
      const detail = await response.text();
      throw new Error(detail || `Watchlist items request failed (${response.status})`);
    }
    const payload = await response.json();
    setWatchlistItems(Array.isArray(payload) ? payload : []);
  }, []);

  const loadWatchlists = useCallback(async () => {
    const response = await fetch('/api/v1/stock-monitoring/watchlists', { credentials: 'include' });
    if (!response.ok) {
      const detail = await response.text();
      throw new Error(detail || `Watchlists request failed (${response.status})`);
    }
    const payload = await response.json();
    const items = Array.isArray(payload) ? payload : [];
    setWatchlists(items);

    if (items.length > 0) {
      const preferred =
        items.find((w) => w.id === activeWatchlistId)
        || items[0];
      setActiveWatchlistId(preferred.id);
      await loadWatchlistItems(preferred.id);
      return;
    }

    const createResponse = await fetch('/api/v1/stock-monitoring/watchlists', {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Primary' }),
    });
    if (!createResponse.ok) {
      const detail = await createResponse.text();
      throw new Error(detail || `Create watchlist failed (${createResponse.status})`);
    }
    const created = await createResponse.json();
    setWatchlists([created]);
    setActiveWatchlistId(created.id);

    for (const symbol of DEFAULT_WATCHLIST) {
      await fetch(`/api/v1/stock-monitoring/watchlists/${created.id}/items`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ symbol }),
      });
    }
    await loadWatchlistItems(created.id);
  }, [activeWatchlistId, loadWatchlistItems]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEffect(() => {
    if (!autoRefresh) {
      return undefined;
    }

    const intervalId = setInterval(() => {
      loadData({ silent: true });
      fetchQuotes();
    }, 30000);

    return () => clearInterval(intervalId);
  }, [autoRefresh, fetchQuotes, loadData]);

  useEffect(() => {
    localStorage.setItem(ALERTS_STORAGE_KEY, JSON.stringify(alerts));
  }, [alerts]);
  useEffect(() => {
    localStorage.setItem(NOTIFICATIONS_STORAGE_KEY, JSON.stringify(notifications));
  }, [notifications]);
  useEffect(() => {
    const symbols = watchlistItems.map((item) => String(item.symbol).toUpperCase()).filter(Boolean);
    if (!symbols.includes(alertSymbol)) {
      setAlertSymbol(symbols[0] || '');
    }
  }, [alertSymbol, watchlistItems]);
  useEffect(() => {
    loadWatchlists().catch((err) => {
      setWatchlistError(err instanceof Error ? err.message : 'Failed to load watchlists');
    });
  }, [loadWatchlists]);
  useEffect(() => {
    // Trigger alerts when prices cross thresholds (one-shot until condition resets).
    const fired = [];
    setAlerts((prev) => prev.map((alert) => {
      if (!alert.enabled) {
        return alert.active ? { ...alert, active: false } : alert;
      }
      const current = prices[alert.symbol];
      if (typeof current !== 'number' || Number.isNaN(current)) {
        return alert.active ? { ...alert, active: false } : alert;
      }
      const conditionMet = alert.direction === 'above'
        ? current >= alert.target
        : current <= alert.target;
      if (conditionMet && !alert.active) {
        const now = new Date().toISOString();
        fired.push({
          id: `${now}-${alert.id}`,
          createdAt: now,
          severity: 'warning',
          message: `${alert.symbol} is ${current.toFixed(2)} (${alert.direction} ${alert.target.toFixed(2)})`,
        });
        return { ...alert, active: true, lastTriggeredAt: now };
      }
      if (!conditionMet && alert.active) {
        return { ...alert, active: false };
      }
      return alert;
    }));
    if (fired.length > 0) {
      setNotifications((prev) => [...fired, ...prev].slice(0, 50));
    }
  }, [prices]);
  const isHealthy = useMemo(() => health.toLowerCase() === 'ok', [health]);
  const dbConnected = useMemo(
    () => (status?.database || '').toLowerCase() === 'connected',
    [status]
  );
  const watchlistSymbols = useMemo(
    () => watchlistItems.map((item) => String(item.symbol).toUpperCase()).filter(Boolean),
    [watchlistItems]
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
        quotesAsOf,
        health,
        status,
        metrics,
        prices,
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
  }, [health, history, lastUpdated, latestCheck?.severity, metrics, prices, quotesAsOf, status]);

  const addSymbol = useCallback(async () => {
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
    const symbols = watchlistItems.map((item) => String(item.symbol).toUpperCase());
    if (symbols.includes(normalized)) {
      setWatchlistError('Symbol already exists in your watchlist.');
      return;
    }
    if (!activeWatchlistId) {
      setWatchlistError('No watchlist selected yet.');
      return;
    }
    try {
      const response = await fetch(`/api/v1/stock-monitoring/watchlists/${activeWatchlistId}/items`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ symbol: normalized }),
      });
      if (!response.ok) {
        const detail = await response.json().catch(() => ({}));
        throw new Error(detail?.error || `Add symbol failed (${response.status})`);
      }
      const created = await response.json();
      setWatchlistItems((prev) => [created, ...prev]);
      setSymbolInput('');
    } catch (err) {
      setWatchlistError(err instanceof Error ? err.message : 'Failed to add symbol.');
    }
  }, [activeWatchlistId, symbolInput, watchlistItems]);

  const removeSymbol = useCallback(async (symbolToRemove) => {
    if (!activeWatchlistId) {
      return;
    }
    const item = watchlistItems.find((candidate) => candidate.symbol === symbolToRemove);
    if (!item?.id) {
      return;
    }
    try {
      const response = await fetch(
        `/api/v1/stock-monitoring/watchlists/${activeWatchlistId}/items/${item.id}`,
        {
          method: 'DELETE',
          credentials: 'include',
        }
      );
      if (!response.ok) {
        const detail = await response.json().catch(() => ({}));
        throw new Error(detail?.error || `Delete symbol failed (${response.status})`);
      }
      setWatchlistItems((prev) => prev.filter((candidate) => candidate.id !== item.id));
    } catch (err) {
      setWatchlistError(err instanceof Error ? err.message : 'Failed to remove symbol.');
    }
  }, [activeWatchlistId, watchlistItems]);
  const addAlert = useCallback(() => {
    setAlertError('');
    const symbol = alertSymbol.toUpperCase().trim();
    const target = Number(alertTarget);
    if (!symbol) {
      setAlertError('Choose a symbol for the alert.');
      return;
    }
    if (Number.isNaN(target) || target <= 0) {
      setAlertError('Target price must be a positive number.');
      return;
    }
    const duplicate = alerts.some(
      (item) => item.symbol === symbol && item.direction === alertDirection && item.target === target
    );
    if (duplicate) {
      setAlertError('Same alert already exists.');
      return;
    }
    const now = new Date().toISOString();
    setAlerts((prev) => [
      {
        id: `${symbol}-${alertDirection}-${target}-${now}`,
        symbol,
        direction: alertDirection,
        target,
        enabled: true,
        active: false,
        createdAt: now,
        lastTriggeredAt: null,
      },
      ...prev,
    ]);
    setAlertTarget('');
  }, [alertDirection, alertSymbol, alertTarget, alerts]);
  const removeAlert = useCallback((alertId) => {
    setAlerts((prev) => prev.filter((item) => item.id !== alertId));
  }, []);
  const toggleAlert = useCallback((alertId, enabled) => {
    setAlerts((prev) => prev.map((item) => (
      item.id === alertId ? { ...item, enabled, active: enabled ? item.active : false } : item
    )));
  }, []);
  const clearNotifications = useCallback(() => {
    setNotifications([]);
  }, []);
  const changeActiveWatchlist = useCallback(async (nextId) => {
    setWatchlistError('');
    setActiveWatchlistId(nextId);
    try {
      await loadWatchlistItems(nextId);
    } catch (err) {
      setWatchlistError(err instanceof Error ? err.message : 'Failed to load watchlist');
    }
  }, [loadWatchlistItems]);
  const refreshAll = useCallback(async () => {
    await Promise.all([loadData(), fetchQuotes()]);
  }, [fetchQuotes, loadData]);

  useEffect(() => {
    fetchQuotes();
  }, [fetchQuotes]);

  useEffect(() => {
    const onPopState = () => setRoutePath(window.location.pathname);
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  }, []);

  const navigateToView = useCallback((path) => {
    if (window.location.pathname !== path) {
      window.history.pushState({}, '', path);
      setRoutePath(path);
    }
  }, []);

  const isOpsPage = routePath.startsWith('/stock/ops');

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
            <p className="page-subtitle">
              {isOpsPage
                ? 'Operations and runtime diagnostics for the stock service.'
                : 'Market watchlist, provider quotes, and alert workflows for the EU markets MVP.'}
            </p>
            <div className="quick-links">
              <a href="/services">All Services</a>
              <a href="/profile">Profile</a>
              <button type="button" className="inline-link-btn" onClick={() => navigateToView('/stock')}>
                Market
              </button>
              <button type="button" className="inline-link-btn" onClick={() => navigateToView('/stock/ops')}>
                Ops
              </button>
              <a href="/stock-monitoring/endpoints">Endpoints</a>
            </div>
          </div>

          <div className="view-tabs">
            <button
              type="button"
              className={`view-tab ${!isOpsPage ? 'active' : ''}`}
              onClick={() => navigateToView('/stock')}
            >
              Market view
            </button>
            <button
              type="button"
              className={`view-tab ${isOpsPage ? 'active' : ''}`}
              onClick={() => navigateToView('/stock/ops')}
            >
              Ops view
            </button>
          </div>

          <div className="status-toolbar">
            <button
              type="button"
              onClick={refreshAll}
              disabled={loading}
              className="refresh-btn"
            >
              {loading ? 'Refreshing...' : (isOpsPage ? 'Refresh status' : 'Refresh market data')}
            </button>
            {isOpsPage && (
              <button type="button" className="copy-btn" onClick={copyDiagnostics}>
                {copied ? 'Copied!' : 'Copy diagnostics'}
              </button>
            )}
            <label className="auto-refresh-toggle">
              <input
                type="checkbox"
                checked={autoRefresh}
                onChange={(event) => setAutoRefresh(event.target.checked)}
              />
              Auto-refresh (30s)
            </label>
            {isOpsPage && lastUpdated && (
              <span className="last-updated">Last updated: {lastUpdated}</span>
            )}
            {quotesAsOf && (
              <span className="last-updated">Data as of: {new Date(quotesAsOf).toLocaleString()}</span>
            )}
          </div>
          {!isOpsPage && quotesError && (
            <div className="stock-error">
              <p className="stock-error-title">Unable to load provider quotes</p>
              <p>{quotesError}</p>
            </div>
          )}

          {isOpsPage && (hasIncident || hasWarningStreak) && (
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

          {isOpsPage && error && (
            <div className="stock-error">
              <p className="stock-error-title">Unable to load stock service status</p>
              <p>{error}</p>
            </div>
          )}

          {isOpsPage && (
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
          )}

          {isOpsPage && (
          <section className="status-card scope-card">
            <p className="status-label">Scope</p>
            <p className="status-value">{status?.scope || (loading ? 'Loading...' : 'unknown')}</p>
          </section>
          )}

          {!isOpsPage && (
          <section className="status-card scope-card">
            <div className="raw-header">
              <p className="status-label">Watchlist (P0.2 MVP)</p>
              <span className="history-count">
                {watchlistSymbols.length} symbols · {watchlists.length} list{watchlists.length === 1 ? '' : 's'}
              </span>
            </div>
            {watchlists.length > 1 && (
              <div className="watchlist-form">
                <select
                  value={activeWatchlistId || ''}
                  onChange={(event) => changeActiveWatchlist(Number(event.target.value))}
                  aria-label="Select watchlist"
                >
                  {watchlists.map((list) => (
                    <option key={list.id} value={list.id}>
                      {list.name}
                    </option>
                  ))}
                </select>
              </div>
            )}
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
              {watchlistSymbols.map((symbol) => (
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
          )}

          {!isOpsPage && (
          <section className="status-card scope-card">
            <div className="raw-header">
              <p className="status-label">Current prices (provider feed)</p>
            </div>
            <div className="price-grid">
              {watchlistSymbols.map((symbol) => (
                <div className="price-input-row" key={`price-${symbol}`}>
                  <span>{symbol}</span>
                  <strong>{typeof prices[symbol] === 'number' ? prices[symbol].toFixed(2) : 'n/a'}</strong>
                </div>
              ))}
            </div>
          </section>
          )}

          {!isOpsPage && (
          <section className="status-card scope-card">
            <div className="raw-header">
              <p className="status-label">Price alerts (P0.3 MVP)</p>
              <span className="history-count">{alerts.length} alerts</span>
            </div>
            <div className="alert-form">
              <select
                value={alertSymbol}
                onChange={(event) => setAlertSymbol(event.target.value)}
                  disabled={watchlistSymbols.length === 0}
              >
                {watchlistSymbols.map((symbol) => (
                  <option key={`opt-${symbol}`} value={symbol}>{symbol}</option>
                ))}
              </select>
              <select value={alertDirection} onChange={(event) => setAlertDirection(event.target.value)}>
                <option value="above">above</option>
                <option value="below">below</option>
              </select>
              <input
                type="number"
                step="0.01"
                inputMode="decimal"
                value={alertTarget}
                onChange={(event) => setAlertTarget(event.target.value)}
                placeholder="Target price"
              />
              <button type="button" className="raw-toggle-btn" onClick={addAlert}>
                Add alert
              </button>
            </div>
            {watchlistSymbols.length === 0 && (
              <p className="status-meta">Add at least one watchlist symbol to create alerts.</p>
            )}
            {alertError && <p className="watchlist-error">{alertError}</p>}
            {alerts.length === 0 ? (
              <p className="status-meta">No alerts yet.</p>
            ) : (
              <div className="alerts-list">
                {alerts.map((alert) => (
                  <div className={`alert-item ${alert.active ? 'active' : ''}`} key={alert.id}>
                    <div>
                      <strong>{alert.symbol}</strong> {alert.direction} {alert.target.toFixed(2)}
                      <div className="status-meta">
                        {alert.lastTriggeredAt
                          ? `Last triggered: ${new Date(alert.lastTriggeredAt).toLocaleString()}`
                          : `Created: ${new Date(alert.createdAt).toLocaleString()}`}
                      </div>
                    </div>
                    <div className="alert-actions">
                      <label className="toggle-inline">
                        <input
                          type="checkbox"
                          checked={alert.enabled}
                          onChange={(event) => toggleAlert(alert.id, event.target.checked)}
                        />
                        Enabled
                      </label>
                      <button type="button" className="delete-btn" onClick={() => removeAlert(alert.id)}>
                        Delete
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
          )}

          {!isOpsPage && (
          <section className="status-card scope-card">
            <div className="raw-header">
              <p className="status-label">In-app notifications (P0.4 channel)</p>
              <button type="button" className="raw-toggle-btn" onClick={clearNotifications}>
                Clear
              </button>
            </div>
            {notifications.length === 0 ? (
              <p className="status-meta">No notifications yet.</p>
            ) : (
              <div className="notifications-list">
                {notifications.map((note) => (
                  <div className="notification-item" key={note.id}>
                    <div className="status-meta">{new Date(note.createdAt).toLocaleTimeString()}</div>
                    <div>{note.message}</div>
                  </div>
                ))}
              </div>
            )}
          </section>
          )}

          {isOpsPage && (
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
          )}

          {isOpsPage && (
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
          )}
        </main>

        <footer className="footer">
          <p>&copy; 2026 Lianel World. All rights reserved.</p>
        </footer>
      </div>
    </div>
  );
}

export default App;
