import React, { useCallback, useEffect, useMemo, useState } from 'react';
import './App.css';
import { ApiError, apiFetch, apiJson, getLoginUrl } from './apiClient';
import StockPageTemplate from './StockPageTemplate';

const API_HEALTH = '/health';
const API_STATUS = '/status';
const API_ME = '/me';
const MAX_HISTORY_ITEMS = 20;
const WARN_LATENCY_MS = 800;
const CRITICAL_LATENCY_MS = 2000;
const NOTIFICATIONS_STORAGE_KEY = 'stock_monitoring_notifications_v1';
const DEFAULT_WATCHLIST = ['ASML.AS', 'SAP.DE', 'SHEL.L'];
const IS_TEST_ENV = process.env.NODE_ENV === 'test';

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

function normalizeAlertFromApi(item) {
  const symbol = String(item?.symbol || '').toUpperCase();
  const direction = item?.condition_type === 'below' ? 'below' : 'above';
  const target = Number(item?.condition_value);
  return {
    id: String(item?.id || ''),
    symbol,
    direction,
    target: Number.isFinite(target) ? target : 0,
    enabled: Boolean(item?.enabled),
    active: Boolean(item?.triggered),
    createdAt: new Date().toISOString(),
    lastTriggeredAt: item?.triggered ? new Date().toISOString() : null,
  };
}

function formatMoney(value, currency) {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return 'n/a';
  }
  if (currency && /^[A-Z]{3}$/.test(currency)) {
    try {
      return new Intl.NumberFormat(undefined, {
        style: 'currency',
        currency,
        minimumFractionDigits: 2,
        maximumFractionDigits: 4,
      }).format(value);
    } catch {
      // Fallback below if browser rejects a currency code.
    }
  }
  return value.toFixed(2);
}

function formatMoneyWithCurrency(value, currency) {
  const output = formatMoney(value, currency);
  if (output === 'n/a') {
    return output;
  }
  return currency && /^[A-Z]{3}$/.test(currency) ? `${output} (${currency})` : output;
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
  const [autoRefresh, setAutoRefresh] = useState(!IS_TEST_ENV);
  const [showRaw, setShowRaw] = useState(false);
  const [history, setHistory] = useState([]);
  const [copied, setCopied] = useState(false);
  const [watchlists, setWatchlists] = useState([]);
  const [activeWatchlistId, setActiveWatchlistId] = useState(null);
  const [watchlistItems, setWatchlistItems] = useState([]);
  const [newWatchlistName, setNewWatchlistName] = useState('');
  const [renameWatchlistName, setRenameWatchlistName] = useState('');
  const [watchlistBusy, setWatchlistBusy] = useState(false);
  const [symbolInput, setSymbolInput] = useState('');
  const [watchlistError, setWatchlistError] = useState('');
  const [prices, setPrices] = useState({});
  const [previousPrices, setPreviousPrices] = useState({});
  const [quoteCurrencies, setQuoteCurrencies] = useState({});
  const [quotesAsOf, setQuotesAsOf] = useState(null);
  const [quotesError, setQuotesError] = useState('');
  const [alerts, setAlerts] = useState([]);
  const [alertsBusy, setAlertsBusy] = useState(false);
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
  const [authState, setAuthState] = useState({
    checking: true,
    isAuthenticated: true,
    userId: '',
  });

  const applyAuthError = useCallback((err) => {
    if (err instanceof ApiError && err.status === 401) {
      setAuthState({
        checking: false,
        isAuthenticated: false,
        userId: '',
      });
      return true;
    }
    return false;
  }, []);

  const loadAuthState = useCallback(async () => {
    setAuthState((prev) => ({ ...prev, checking: true }));
    try {
      const mePayload = await apiJson(API_ME);
      setAuthState({
        checking: false,
        isAuthenticated: true,
        userId: String(mePayload?.user_id || '').trim(),
      });
    } catch (err) {
      if (!applyAuthError(err)) {
        setAuthState((prev) => ({ ...prev, checking: false }));
      }
    }
  }, [applyAuthError]);

  const loadData = useCallback(async ({ silent = false } = {}) => {
    if (!silent) {
      setLoading(true);
    }
    setError('');
    try {
      const timedFetch = async (url) => {
        const start = performance.now();
        const response = await apiFetch(url);
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
        if (issues.some((issue) => issue.includes('(401)'))) {
          setAuthState({
            checking: false,
            isAuthenticated: false,
            userId: '',
          });
        }
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
      setQuoteCurrencies({});
      setQuotesError('');
      return;
    }
    try {
      const params = new URLSearchParams({ symbols: symbols.join(',') });
      const response = await apiFetch(`/quotes?${params.toString()}`);
      if (!response.ok) {
        throw new ApiError(`Quotes request failed (${response.status})`, response.status);
      }
      const payload = await response.json();
      const map = {};
      const currencyMap = {};
      for (const item of payload.quotes || []) {
        if (item?.symbol && typeof item?.price === 'number') {
          const symbol = String(item.symbol).toUpperCase();
          map[symbol] = item.price;
          if (item?.currency) {
            currencyMap[symbol] = String(item.currency).toUpperCase();
          }
        }
      }
      setPrices((prevPrices) => {
        setPreviousPrices((prev) => ({ ...prev, ...prevPrices }));
        return map;
      });
      setQuoteCurrencies(currencyMap);
      setQuotesAsOf(payload?.as_of_ms || null);
      setQuotesError('');
    } catch (err) {
      applyAuthError(err);
      setQuotesError(err instanceof Error ? err.message : 'Failed to load quotes');
    }
  }, [applyAuthError, watchlistItems]);

  const loadWatchlistItems = useCallback(async (watchlistId) => {
    const payload = await apiJson(`/watchlists/${watchlistId}/items`);
    setWatchlistItems(Array.isArray(payload) ? payload : []);
  }, []);

  const loadWatchlists = useCallback(async () => {
    const payload = await apiJson('/watchlists');
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

    const created = await apiJson('/watchlists', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Primary' }),
    });
    setWatchlists([created]);
    setActiveWatchlistId(created.id);

    for (const symbol of DEFAULT_WATCHLIST) {
      await apiJson(`/watchlists/${created.id}/items`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ symbol }),
      });
    }
    await loadWatchlistItems(created.id);
  }, [activeWatchlistId, loadWatchlistItems]);

  const loadAlerts = useCallback(async () => {
    const payload = await apiJson('/alerts');
    const items = Array.isArray(payload) ? payload : [];
    setAlerts(items.map(normalizeAlertFromApi).filter((item) => item.id && item.symbol));
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEffect(() => {
    loadAuthState();
  }, [loadAuthState]);

  useEffect(() => {
    if (!autoRefresh) {
      return undefined;
    }

    const intervalId = setInterval(() => {
      loadData({ silent: true });
      fetchQuotes();
      loadAlerts().catch(() => {});
    }, 30000);

    return () => clearInterval(intervalId);
  }, [autoRefresh, fetchQuotes, loadAlerts, loadData]);

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
      applyAuthError(err);
      setWatchlistError(err instanceof Error ? err.message : 'Failed to load watchlists');
    });
  }, [applyAuthError, loadWatchlists]);
  useEffect(() => {
    loadAlerts().catch((err) => {
      applyAuthError(err);
      setAlertError(err instanceof Error ? err.message : 'Failed to load alerts');
    });
  }, [applyAuthError, loadAlerts]);
  useEffect(() => {
    const active = watchlists.find((item) => item.id === activeWatchlistId);
    setRenameWatchlistName(active?.name || '');
  }, [activeWatchlistId, watchlists]);
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
          message: `${alert.symbol} is ${formatMoneyWithCurrency(current, quoteCurrencies[alert.symbol])} (${alert.direction} ${formatMoneyWithCurrency(alert.target, quoteCurrencies[alert.symbol])})`,
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
  }, [prices, quoteCurrencies]);
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
        quoteCurrencies,
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
  }, [health, history, lastUpdated, latestCheck?.severity, metrics, prices, quoteCurrencies, quotesAsOf, status]);

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
      const created = await apiJson(`/watchlists/${activeWatchlistId}/items`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ symbol: normalized }),
      });
      setWatchlistItems((prev) => [created, ...prev]);
      setSymbolInput('');
    } catch (err) {
      applyAuthError(err);
      setWatchlistError(err instanceof Error ? err.message : 'Failed to add symbol.');
    }
  }, [activeWatchlistId, applyAuthError, symbolInput, watchlistItems]);

  const removeSymbol = useCallback(async (symbolToRemove) => {
    if (!activeWatchlistId) {
      return;
    }
    const item = watchlistItems.find((candidate) => candidate.symbol === symbolToRemove);
    if (!item?.id) {
      return;
    }
    try {
      await apiJson(`/watchlists/${activeWatchlistId}/items/${item.id}`, { method: 'DELETE' });
      setWatchlistItems((prev) => prev.filter((candidate) => candidate.id !== item.id));
    } catch (err) {
      applyAuthError(err);
      setWatchlistError(err instanceof Error ? err.message : 'Failed to remove symbol.');
    }
  }, [activeWatchlistId, applyAuthError, watchlistItems]);
  const createWatchlist = useCallback(async () => {
    const name = newWatchlistName.trim();
    setWatchlistError('');
    if (!name) {
      setWatchlistError('Enter a watchlist name first.');
      return;
    }
    if (name.length > 255) {
      setWatchlistError('Watchlist name is too long.');
      return;
    }
    if (watchlists.some((item) => String(item.name).trim().toLowerCase() === name.toLowerCase())) {
      setWatchlistError('Watchlist already exists.');
      return;
    }
    setWatchlistBusy(true);
    try {
      const created = await apiJson('/watchlists', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name }),
      });
      setWatchlists((prev) => [created, ...prev]);
      setActiveWatchlistId(created.id);
      setWatchlistItems([]);
      setNewWatchlistName('');
      setSymbolInput('');
    } catch (err) {
      applyAuthError(err);
      setWatchlistError(err instanceof Error ? err.message : 'Failed to create watchlist.');
    } finally {
      setWatchlistBusy(false);
    }
  }, [applyAuthError, newWatchlistName, watchlists]);
  const deleteActiveWatchlist = useCallback(async () => {
    setWatchlistError('');
    if (!activeWatchlistId) {
      setWatchlistError('No watchlist selected.');
      return;
    }
    if (watchlists.length <= 1) {
      setWatchlistError('Create another watchlist before deleting the current one.');
      return;
    }
    setWatchlistBusy(true);
    try {
      await apiJson(`/watchlists/${activeWatchlistId}`, { method: 'DELETE' });
      const remaining = watchlists.filter((item) => item.id !== activeWatchlistId);
      setWatchlists(remaining);
      const nextId = remaining[0]?.id || null;
      setActiveWatchlistId(nextId);
      if (nextId) {
        await loadWatchlistItems(nextId);
      } else {
        setWatchlistItems([]);
      }
      setSymbolInput('');
    } catch (err) {
      applyAuthError(err);
      setWatchlistError(err instanceof Error ? err.message : 'Failed to delete watchlist.');
    } finally {
      setWatchlistBusy(false);
    }
  }, [activeWatchlistId, applyAuthError, loadWatchlistItems, watchlists]);
  const renameActiveWatchlist = useCallback(async () => {
    setWatchlistError('');
    if (!activeWatchlistId) {
      setWatchlistError('No watchlist selected.');
      return;
    }

    const name = renameWatchlistName.trim();
    if (!name) {
      setWatchlistError('Enter a new watchlist name.');
      return;
    }
    if (name.length > 255) {
      setWatchlistError('Watchlist name is too long.');
      return;
    }
    if (watchlists.some((item) => item.id !== activeWatchlistId && String(item.name).trim().toLowerCase() === name.toLowerCase())) {
      setWatchlistError('Watchlist already exists.');
      return;
    }

    setWatchlistBusy(true);
    try {
      const updated = await apiJson(`/watchlists/${activeWatchlistId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name }),
      });
      setWatchlists((prev) => prev.map((item) => (item.id === activeWatchlistId ? updated : item)));
      setRenameWatchlistName(String(updated?.name || name));
    } catch (err) {
      applyAuthError(err);
      setWatchlistError(err instanceof Error ? err.message : 'Failed to rename watchlist.');
    } finally {
      setWatchlistBusy(false);
    }
  }, [activeWatchlistId, applyAuthError, renameWatchlistName, watchlists]);
  const addAlert = useCallback(async () => {
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
    setAlertsBusy(true);
    try {
      const created = await apiJson('/alerts', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          symbol,
          condition_type: alertDirection,
          condition_value: target,
          enabled: true,
        }),
      });
      setAlerts((prev) => [normalizeAlertFromApi(created), ...prev]);
      setAlertTarget('');
    } catch (err) {
      applyAuthError(err);
      setAlertError(err instanceof Error ? err.message : 'Failed to create alert.');
    } finally {
      setAlertsBusy(false);
    }
  }, [alertDirection, alertSymbol, alertTarget, alerts, applyAuthError]);
  const removeAlert = useCallback(async (alertId) => {
    setAlertError('');
    setAlertsBusy(true);
    try {
      await apiJson(`/alerts/${alertId}`, { method: 'DELETE' });
      setAlerts((prev) => prev.filter((item) => item.id !== alertId));
    } catch (err) {
      applyAuthError(err);
      setAlertError(err instanceof Error ? err.message : 'Failed to delete alert.');
    } finally {
      setAlertsBusy(false);
    }
  }, [applyAuthError]);
  const toggleAlert = useCallback(async (alertId, enabled) => {
    setAlertError('');
    setAlertsBusy(true);
    try {
      const updated = await apiJson(`/alerts/${alertId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled }),
      });
      setAlerts((prev) => prev.map((item) => (
        item.id === String(alertId) ? normalizeAlertFromApi(updated) : item
      )));
    } catch (err) {
      applyAuthError(err);
      setAlertError(err instanceof Error ? err.message : 'Failed to update alert.');
    } finally {
      setAlertsBusy(false);
    }
  }, [applyAuthError]);
  const clearNotifications = useCallback(() => {
    setNotifications([]);
  }, []);
  const changeActiveWatchlist = useCallback(async (nextId) => {
    if (!Number.isInteger(nextId) || nextId <= 0 || nextId === activeWatchlistId) {
      return;
    }
    setWatchlistError('');
    setActiveWatchlistId(nextId);
    try {
      await loadWatchlistItems(nextId);
    } catch (err) {
      setWatchlistError(err instanceof Error ? err.message : 'Failed to load watchlist');
    }
  }, [activeWatchlistId, loadWatchlistItems]);
  const refreshAll = useCallback(async () => {
    await Promise.all([loadData(), fetchQuotes(), loadAlerts()]);
  }, [fetchQuotes, loadAlerts, loadData]);

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
  const isWatchlistsPage = routePath.startsWith('/stock/watchlists');
  const isAlertsPage = routePath.startsWith('/stock/alerts');
  const isDashboardPage = !isOpsPage && !isWatchlistsPage && !isAlertsPage;
  const isAuthReady = !authState.checking;
  const dashboardRows = useMemo(() => watchlistSymbols.map((symbol) => {
    const current = prices[symbol];
    const previous = previousPrices[symbol];
    let trend = 'flat';
    if (typeof current === 'number' && typeof previous === 'number') {
      if (current > previous) trend = 'up';
      else if (current < previous) trend = 'down';
    }
    return {
      symbol,
      current,
      previous,
      trend,
      currency: quoteCurrencies[symbol],
    };
  }), [previousPrices, prices, quoteCurrencies, watchlistSymbols]);
  const selectedAlertCurrency = alertSymbol ? (quoteCurrencies[alertSymbol] || 'n/a') : 'n/a';
  const pageSubtitle = isOpsPage
    ? 'Operations and runtime diagnostics for the stock service.'
    : isWatchlistsPage
      ? 'Manage your watchlists and symbols for EU market monitoring.'
      : isAlertsPage
        ? 'Configure and review alert rules and recent notifications.'
        : 'Dashboard view for quotes, trends, and alert/watchlist overview.';

  return (
    <StockPageTemplate
      authState={authState}
      routePath={routePath}
      pageTitle="Stock Exchange Monitoring"
      pageSubtitle={pageSubtitle}
      onNavigate={navigateToView}
    >

          {isAuthReady && !authState.isAuthenticated && (
            <div className="stock-error auth-required">
              <p className="stock-error-title">Authentication required</p>
              <p>Your session is missing or expired. Sign in again to manage watchlists and alerts.</p>
              <button
                type="button"
                className="refresh-btn auth-login-btn"
                onClick={() => {
                  window.location.href = getLoginUrl(routePath);
                }}
              >
                Sign in with Keycloak
              </button>
            </div>
          )}

          <div className="status-toolbar">
            <button
              type="button"
              onClick={refreshAll}
              disabled={loading}
              className="refresh-btn"
            >
              {loading ? 'Refreshing...' : (isOpsPage ? 'Refresh status' : 'Refresh data')}
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

          {isWatchlistsPage && (
          <section className="status-card scope-card">
            <div className="raw-header">
              <p className="status-label">Watchlist (P0.2 MVP)</p>
              <span className="history-count">
                {watchlistSymbols.length} symbols · {watchlists.length} list{watchlists.length === 1 ? '' : 's'}
              </span>
            </div>
            <div className="watchlist-form">
              <input
                type="text"
                value={newWatchlistName}
                onChange={(event) => setNewWatchlistName(event.target.value)}
                className="symbol-input"
                placeholder="New watchlist name"
                aria-label="Create watchlist"
              />
              <button type="button" className="raw-toggle-btn" onClick={createWatchlist} disabled={watchlistBusy}>
                Create list
              </button>
            </div>
            {watchlists.length > 0 && (
              <div className="watchlist-form watchlist-manager">
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
                <button
                  type="button"
                  className="delete-btn"
                  onClick={deleteActiveWatchlist}
                  disabled={watchlistBusy || watchlists.length <= 1}
                >
                  Delete active
                </button>
              </div>
            )}
            {watchlists.length > 0 && (
              <div className="watchlist-form watchlist-manager">
                <input
                  type="text"
                  value={renameWatchlistName}
                  onChange={(event) => setRenameWatchlistName(event.target.value)}
                  className="symbol-input"
                  placeholder="Rename active watchlist"
                  aria-label="Rename active watchlist"
                />
                <button
                  type="button"
                  className="raw-toggle-btn"
                  onClick={renameActiveWatchlist}
                  disabled={watchlistBusy}
                >
                  Rename active
                </button>
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
              <button type="button" className="raw-toggle-btn" onClick={addSymbol} disabled={watchlistBusy}>
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
                    disabled={watchlistBusy}
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
          </section>
          )}

          {isDashboardPage && (
          <section className="status-card scope-card">
            <div className="raw-header">
              <p className="status-label">Dashboard: current prices (provider feed)</p>
              <span className="history-count">
                {watchlistSymbols.length} symbols · {alerts.length} alerts
              </span>
            </div>
            <div className="history-table-wrapper">
              <table className="history-table market-table">
                <thead>
                  <tr>
                    <th>Symbol</th>
                    <th>Currency</th>
                    <th>Latest</th>
                    <th>Previous</th>
                    <th>Trend</th>
                    <th>As of</th>
                  </tr>
                </thead>
                <tbody>
                  {dashboardRows.map((row) => (
                    <tr key={`price-${row.symbol}`}>
                      <td>{row.symbol}</td>
                      <td>{row.currency || 'n/a'}</td>
                      <td><strong>{formatMoneyWithCurrency(row.current, row.currency)}</strong></td>
                      <td>{formatMoneyWithCurrency(row.previous, row.currency)}</td>
                      <td className={`trend-cell trend-${row.trend}`}>
                        {row.trend === 'up' ? '▲ Up' : row.trend === 'down' ? '▼ Down' : '• Flat'}
                      </td>
                      <td>{quotesAsOf ? new Date(quotesAsOf).toLocaleTimeString() : 'n/a'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
          )}

          {isAlertsPage && (
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
              <button type="button" className="raw-toggle-btn" onClick={addAlert} disabled={alertsBusy}>
                Add alert
              </button>
            </div>
            <p className="status-meta">Selected symbol currency: {selectedAlertCurrency}</p>
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
                      <strong>{alert.symbol}</strong> {alert.direction} {formatMoneyWithCurrency(alert.target, quoteCurrencies[alert.symbol])}
                      <div className="status-meta">Currency: {quoteCurrencies[alert.symbol] || 'n/a'}</div>
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
                          disabled={alertsBusy}
                        />
                        Enabled
                      </label>
                      <button type="button" className="delete-btn" onClick={() => removeAlert(alert.id)} disabled={alertsBusy}>
                        Delete
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
          )}

          {isAlertsPage && (
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
    </StockPageTemplate>
  );
}

export default App;
