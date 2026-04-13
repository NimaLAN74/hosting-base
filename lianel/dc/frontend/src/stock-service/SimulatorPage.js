import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import PageTemplate from '../PageTemplate';
import './SimulatorPage.css';

const RUNS_URL = '/api/v1/stock-service/sim/runs';
const PURGE_URL = '/api/v1/stock-service/sim/purge';
const PURGE_STATUS_URL = '/api/v1/stock-service/sim/purge-status';

/** Default campaign: live quotes, 60s cadence, ~6-month readiness window, high max_cycles. */
const SIX_MONTH_LIVE_PRESET = {
  days: 126,
  top: 16,
  quantile: 0.2,
  short_enabled: true,
  initial_capital_usd: 100,
  replay_delay_ms: 60000,
  reinvest_profit: true,
  live_market_data: true,
  max_cycles: 500000,
  readiness_min_days: 126,
  replay_require_full_horizon: false,
  // Production calibration: avoid "no-trade" LIVE runs while keeping cost discipline.
  edge_cost_buffer_bps: 3,
  min_signal_abs_return_bps: 4,
  // Close hysteresis: avoid micro flip churn (buy then sell at same price).
  close_signal_abs_return_bps: 2,
  live_max_quote_age_seconds: 60,
  live_require_bid_ask: true,
  live_max_spread_bps: 80,
  /** Wall-clock LIVE campaign: stop after this many calendar days (or bankrupt / max_cycles). */
  live_campaign_calendar_days: 126,
};

/** Replay on IBKR daily bars: one step per aligned trading day; fails start if overlap &lt; days (reliable 126d training). */
const RESEARCH_126_REPLAY_PRESET = {
  days: 126,
  top: 12,
  quantile: 0.2,
  short_enabled: true,
  initial_capital_usd: 100,
  replay_delay_ms: 0,
  reinvest_profit: true,
  live_market_data: false,
  max_cycles: 200000,
  readiness_min_days: 126,
  replay_require_full_horizon: true,
  live_max_quote_age_seconds: 60,
  live_require_bid_ask: true,
  live_max_spread_bps: 80,
};

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function fetchJson(url, opts = {}) {
  const r = await fetch(url, { credentials: 'include', headers: { Accept: 'application/json' }, ...opts });
  const j = await r.json().catch(() => null);
  if (!r.ok) throw new Error(j?.error || `Failed (${r.status})`);
  return j;
}

function normalizeErrorPayload(payload, status) {
  const msg = String(payload?.error || `Failed (${status})`);
  const retryableByBody =
    /history failed|chart data unavailable|temporar|unavailable/i.test(msg) &&
    !/replay_require_full_horizon/i.test(msg);
  return {
    message: msg,
    retryable: Boolean(payload?.retryable) || status === 503 || retryableByBody,
  };
}

function fmtTs(ts) {
  if (!ts) return '—';
  const d = new Date(Number(ts) * 1000);
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleString('sv-SE', { hour12: false });
}

/** UTC calendar day YYYY-MM-DD for a unix-seconds timestamp (replay filter). */
function utcYmdFromTs(tsSec) {
  const t = Number(tsSec || 0);
  if (!t) return '';
  return new Date(t * 1000).toISOString().slice(0, 10);
}

function sideActionLabel(side) {
  const s = String(side || '').toUpperCase();
  if (s === 'SHORT') return 'SELL (short)';
  if (s === 'LONG') return 'BUY (long)';
  return s || '—';
}

function statusTone(status) {
  const s = String(status || '').toLowerCase();
  if (s.includes('fail') || s.includes('bankrupt') || s.includes('kill')) return 'bad';
  if (s.includes('pass') || s.includes('ready') || s.includes('completed')) return 'good';
  if (s.includes('running') || s.includes('pause')) return 'warn';
  return 'neutral';
}

function summarizeRecentExecution(timeline, orders, runStatus) {
  const live = runStatus?.live_market_data === true;
  const nowSec = Date.now() / 1000;
  const windowSec = 60 * 60;
  const cutoff = nowSec - windowSec;

  const noise = new Set(['MarketSnapshotSeen', 'PortfolioValued', 'RiskSnapshot', 'HoldingsSnapshot']);
  const events = Array.isArray(timeline) ? timeline : [];
  const recentEvents = events.filter((e) => {
    const ts = Number(e?.ts || 0);
    if (!ts) return false;
    if (ts < cutoff) return false;
    const kind = String(e?.kind || '');
    if (!kind) return false;
    return !noise.has(kind);
  });

  const ordersArr = Array.isArray(orders) ? orders : [];
  const recentFills = ordersArr.filter((o) => {
    const status = String(o?.status || '').toLowerCase();
    if (status !== 'filled' && status !== 'partially_filled') return false;
    const ts = Number(live ? (o?.wall_clock_ts || o?.ts || 0) : (o?.ts || 0));
    return ts && ts >= cutoff;
  });

  const skipped = recentEvents.filter((e) => String(e?.kind || '') === 'TradeSkipped');
  const byReason = new Map();
  skipped.forEach((e) => {
    const reason = String(e?.payload?.reason || 'unknown');
    byReason.set(reason, (byReason.get(reason) || 0) + 1);
  });
  const topReasons = Array.from(byReason.entries())
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5);

  return {
    cutoff,
    recentFills,
    recentFillCount: recentFills.length,
    recentSkipCount: skipped.length,
    topSkipReasons: topReasons,
    recentDecisionCount: recentEvents.filter((e) => String(e?.kind || '') === 'DecisionCreated').length,
  };
}

export default function SimulatorPage() {
  const [runs, setRuns] = useState([]);
  const [runsLoading, setRunsLoading] = useState(true);
  const [runsError, setRunsError] = useState('');
  const [selectedRunId, setSelectedRunId] = useState('');

  const [runStatus, setRunStatus] = useState(null);
  const [runStatusError, setRunStatusError] = useState('');
  const [timeline, setTimeline] = useState([]);
  const [timelineError, setTimelineError] = useState('');
  const [hideNoisyTimeline, setHideNoisyTimeline] = useState(true);
  const [orders, setOrders] = useState([]);
  const [ordersError, setOrdersError] = useState('');
  const [riskSeries, setRiskSeries] = useState([]);
  const [riskError, setRiskError] = useState('');
  const [readiness, setReadiness] = useState(null);
  const [readinessError, setReadinessError] = useState('');
  const [biasFindings, setBiasFindings] = useState([]);
  const [biasError, setBiasError] = useState('');
  const [holdings, setHoldings] = useState(null);
  const [holdingsError, setHoldingsError] = useState('');
  const [portfolioQuery, setPortfolioQuery] = useState('');

  const [exchangeFilter, setExchangeFilter] = useState('ALL');
  const [refreshEnabled, setRefreshEnabled] = useState(true);
  const [refreshMs, setRefreshMs] = useState(60000);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [lastRefreshTs, setLastRefreshTs] = useState(0);

  const [selectedOrderKey, setSelectedOrderKey] = useState('');
  // Default to showing all orders to avoid the common "no trades" confusion
  // when the simulator traded earlier today or across midnight boundaries.
  const [onlyTodayOrders, setOnlyTodayOrders] = useState(false);
  const [explainData, setExplainData] = useState(null);
  const [explainError, setExplainError] = useState('');

  const [form, setForm] = useState(() => ({ ...SIX_MONTH_LIVE_PRESET }));
  const [startLoading, setStartLoading] = useState(false);
  const [startMsg, setStartMsg] = useState('');
  const [purgeSecret, setPurgeSecret] = useState('');
  const [purgeLoading, setPurgeLoading] = useState(false);
  /** null = backend did not report (older deploy); false = SIMULATOR_PURGE_SECRET unset */
  const [serverPurgeAvailable, setServerPurgeAvailable] = useState(null);
  const [controlLoading, setControlLoading] = useState(false);
  const [controlMsg, setControlMsg] = useState('');
  const readinessUnavailableRunsRef = useRef(new Set());
  const latestRunLoadTokenRef = useRef(0);

  const exchanges = useMemo(() => {
    const set = new Set();
    runs.forEach((r) => (r.exchanges || []).forEach((x) => set.add(x)));
    return ['ALL', ...Array.from(set)];
  }, [runs]);

  const selectedOrder = useMemo(
    () => orders.find((o) => `${o.order_id}-${o.ts}-${o.status}` === selectedOrderKey) || null,
    [orders, selectedOrderKey]
  );

  const blotterOrders = useMemo(() => {
    const live = runStatus?.live_market_data === true;
    const sorted = [...orders].sort((a, b) => {
      const ta = live ? Number(a?.wall_clock_ts || a?.ts || 0) : Number(a?.ts || 0);
      const tb = live ? Number(b?.wall_clock_ts || b?.ts || 0) : Number(b?.ts || 0);
      return tb - ta;
    });
    if (!onlyTodayOrders) return sorted;
    if (live) {
      const today = new Date();
      const y = today.getFullYear();
      const m = today.getMonth();
      const d = today.getDate();
      return sorted.filter((o) => {
        const t = Number(o?.wall_clock_ts || o?.ts || 0);
        if (!t) return false;
        const dt = new Date(t * 1000);
        return dt.getFullYear() === y && dt.getMonth() === m && dt.getDate() === d;
      });
    }
    // Replay: wall_clock_ts is always "now" on the server — filter by latest *simulation* bar date instead.
    const days = sorted.map((o) => utcYmdFromTs(o.ts)).filter(Boolean);
    if (!days.length) return sorted;
    const latestDay = days.reduce((a, b) => (a > b ? a : b));
    return sorted.filter((o) => utcYmdFromTs(o.ts) === latestDay);
  }, [orders, onlyTodayOrders, runStatus?.live_market_data]);

  const latestRisk = useMemo(() => (riskSeries.length > 0 ? riskSeries[riskSeries.length - 1] : null), [riskSeries]);
  const runStartedEvent = useMemo(
    () => timeline.find((e) => e?.kind === 'RunStarted') || null,
    [timeline]
  );
  const filteredTimeline = useMemo(() => {
    if (!hideNoisyTimeline) return timeline;
    const noisy = new Set(['MarketSnapshotSeen', 'PortfolioValued', 'RiskSnapshot', 'HoldingsSnapshot']);
    return timeline.filter((e) => !noisy.has(String(e?.kind || '')));
  }, [timeline, hideNoisyTimeline]);
  const runDataModeLabel = useMemo(() => {
    if (runStatus?.live_market_data === true) return 'LIVE (real-time quotes)';
    if (runStatus?.live_market_data === false) return 'REPLAY (historical daily bars)';
    if (!runStartedEvent) return '—';
    return runStartedEvent?.payload?.live_market_data
      ? 'LIVE (real-time quotes)'
      : 'REPLAY (historical daily bars)';
  }, [runStatus, runStartedEvent]);

  const holdingsDeployedTotal = useMemo(() => {
    const d = Number(holdings?.deployed_usd || 0);
    return d > 0 ? d : 1;
  }, [holdings]);

  const portfolio = useMemo(() => {
    const legs = Array.isArray(holdings?.legs) ? holdings.legs : [];
    const q = String(portfolioQuery || '').trim().toLowerCase();
    const filtered = q
      ? legs.filter((r) => {
        const sym = String(r?.symbol || '').toLowerCase();
        const ex = String(r?.exchange || '').toLowerCase();
        const side = String(r?.side || '').toLowerCase();
        return sym.includes(q) || ex.includes(q) || side.includes(q);
      })
      : legs;
    const sorted = [...filtered].sort((a, b) => Number(b?.notional_usd || 0) - Number(a?.notional_usd || 0));
    const totals = sorted.reduce(
      (acc, r) => {
        const notional = Number(r?.notional_usd || 0);
        const isShort = String(r?.side || '').includes('SHORT');
        if (isShort) acc.short += Math.abs(notional);
        else acc.long += Math.abs(notional);
        return acc;
      },
      { long: 0, short: 0 }
    );
    return { legs: sorted, totals, query: q };
  }, [holdings, portfolioQuery]);

  const recentExecution = useMemo(
    () => summarizeRecentExecution(timeline, orders, runStatus),
    [timeline, orders, runStatus]
  );

  const runLooksStuck = useMemo(() => {
    if (!runStatus || String(runStatus.status || '').toLowerCase() !== 'running') return false;
    if (Number(runStatus.cycles_completed || 0) > 0) return false;
    const started = Number(runStatus.started_at_ts || 0);
    if (!started) return false;
    const ageSec = Date.now() / 1000 - started;
    return ageSec > 180;
  }, [runStatus]);

  const clearRunPanels = useCallback(() => {
    setRunStatus(null);
    setRunStatusError('');
    setTimeline([]);
    setTimelineError('');
    setOrders([]);
    setOrdersError('');
    setRiskSeries([]);
    setRiskError('');
    setReadiness(null);
    setReadinessError('');
    setBiasFindings([]);
    setBiasError('');
    setHoldings(null);
    setHoldingsError('');
    setSelectedOrderKey('');
    setExplainData(null);
    setExplainError('');
  }, []);

  const loadRuns = useCallback(async () => {
    setRunsLoading(true);
    setRunsError('');
    try {
      const j = await fetchJson(`${RUNS_URL}?limit=40`);
      const nextRuns = Array.isArray(j?.runs) ? j.runs : [];
      setRuns(nextRuns);
      if (!selectedRunId && nextRuns.length > 0) {
        clearRunPanels();
        setSelectedRunId(nextRuns[0].run_id);
      }
      if (selectedRunId && !nextRuns.some((r) => r.run_id === selectedRunId) && nextRuns.length > 0) {
        clearRunPanels();
        setSelectedRunId(nextRuns[0].run_id);
      }
    } catch (e) {
      setRunsError(String(e?.message || e));
    } finally {
      setRunsLoading(false);
    }
  }, [selectedRunId, clearRunPanels]);

  const loadExplain = useCallback(async (decisionId) => {
    setExplainError('');
    if (!selectedRunId || !decisionId) return;
    try {
      const url = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/decision/${encodeURIComponent(decisionId)}/explain`;
      const j = await fetchJson(url);
      setExplainData(j);
    } catch (e) {
      setExplainData(null);
      setExplainError(String(e?.message || e));
    }
  }, [selectedRunId]);

  const loadSelectedRun = useCallback(async () => {
    if (!selectedRunId) return;
    const loadToken = Date.now() + Math.random();
    latestRunLoadTokenRef.current = loadToken;
    setIsRefreshing(true);
    const statusUrl = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/status`;
    // Timeline can be noisy (per-cycle snapshot events). Pull more and allow client-side filtering.
    const timelineLimit = 5000;
    const timelineUrl = exchangeFilter === 'ALL'
      ? `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/timeline?limit=${timelineLimit}`
      : `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/exchanges/${encodeURIComponent(exchangeFilter)}?limit=${timelineLimit}`;
    const biasUrl = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/bias-report`;
    const ordersUrl = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/orders?limit=2000`;
    const riskUrl = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/risk?limit=500`;
    const readinessUrl = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/readiness`;
    const holdingsUrl = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/holdings`;
    const shouldQueryReadiness = !readinessUnavailableRunsRef.current.has(selectedRunId);

    const fetchHoldingsSnapshot = () =>
      fetch(holdingsUrl, { credentials: 'include', headers: { Accept: 'application/json' } }).then(async (r) => {
        if (r.status === 404) return null;
        const j = await r.json().catch(() => null);
        if (!r.ok) throw new Error(j?.error || `Failed (${r.status})`);
        return j?.holdings ?? null;
      });

    const [statusRes, timelineRes, biasRes, ordersRes, riskRes, readinessRes, holdingsRes] = await Promise.allSettled([
      fetchJson(statusUrl),
      fetchJson(timelineUrl),
      fetchJson(biasUrl),
      fetchJson(ordersUrl),
      fetchJson(riskUrl),
      shouldQueryReadiness ? fetchJson(readinessUrl) : Promise.resolve(null),
      fetchHoldingsSnapshot(),
    ]);
    if (latestRunLoadTokenRef.current !== loadToken) return;

    if (statusRes.status === 'fulfilled') {
      setRunStatus(statusRes.value);
      setRunStatusError('');
    } else {
      setRunStatus(null);
      setRunStatusError(String(statusRes.reason?.message || statusRes.reason || 'Failed to load status'));
    }

    if (timelineRes.status === 'fulfilled') {
      setTimeline(Array.isArray(timelineRes.value?.events) ? timelineRes.value.events : []);
      setTimelineError('');
    } else {
      setTimeline([]);
      setTimelineError(String(timelineRes.reason?.message || timelineRes.reason || 'Failed to load timeline'));
    }

    if (biasRes.status === 'fulfilled') {
      setBiasFindings(Array.isArray(biasRes.value?.findings) ? biasRes.value.findings : []);
      setBiasError('');
    } else {
      setBiasFindings([]);
      setBiasError(String(biasRes.reason?.message || biasRes.reason || 'Failed to load findings'));
    }

    if (ordersRes.status === 'fulfilled') {
      const nextOrders = Array.isArray(ordersRes.value?.orders) ? ordersRes.value.orders : [];
      setOrders(nextOrders);
      setOrdersError('');
      if (!selectedOrderKey && nextOrders.length > 0) {
        const latest = [...nextOrders].sort((a, b) => {
          const ta = Number(a?.wall_clock_ts || a?.ts || 0);
          const tb = Number(b?.wall_clock_ts || b?.ts || 0);
          return tb - ta;
        })[0];
        if (latest) setSelectedOrderKey(`${latest.order_id}-${latest.ts}-${latest.status}`);
      }
    } else {
      setOrders([]);
      setOrdersError(String(ordersRes.reason?.message || ordersRes.reason || 'Failed to load orders'));
    }

    if (riskRes.status === 'fulfilled') {
      setRiskSeries(Array.isArray(riskRes.value?.risk) ? riskRes.value.risk : []);
      setRiskError('');
    } else {
      setRiskSeries([]);
      setRiskError(String(riskRes.reason?.message || riskRes.reason || 'Failed to load risk'));
    }

    if (!shouldQueryReadiness) {
      setReadiness(null);
      setReadinessError('');
    } else if (readinessRes.status === 'fulfilled') {
      setReadiness(readinessRes.value || null);
      setReadinessError('');
    } else {
      const msg = String(readinessRes.reason?.message || readinessRes.reason || '');
      if (/Failed \(404\)|not found/i.test(msg)) {
        // Older simulator runs may not have persisted readiness artifacts.
        readinessUnavailableRunsRef.current.add(selectedRunId);
        setReadiness(null);
        setReadinessError('');
      } else {
        setReadiness(null);
        setReadinessError(msg || 'Failed to load readiness');
      }
    }

    if (holdingsRes.status === 'fulfilled') {
      setHoldings(holdingsRes.value);
      setHoldingsError('');
    } else {
      setHoldings(null);
      setHoldingsError(String(holdingsRes.reason?.message || holdingsRes.reason || 'Failed to load holdings'));
    }

    setLastRefreshTs(Date.now());
    setIsRefreshing(false);
  }, [selectedRunId, exchangeFilter, selectedOrderKey]);

  useEffect(() => {
    loadRuns();
  }, [loadRuns]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const r = await fetch(PURGE_STATUS_URL, {
          credentials: 'include',
          headers: { Accept: 'application/json' },
        });
        const j = await r.json().catch(() => null);
        if (cancelled || !r.ok || !j || typeof j.purge_enabled !== 'boolean') return;
        setServerPurgeAvailable(j.purge_enabled);
      } catch {
        /* leave null — older image without /purge-status */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    setExplainData(null);
    setExplainError('');
    loadSelectedRun();
  }, [loadSelectedRun]);

  useEffect(() => {
    if (!refreshEnabled || !selectedRunId) return undefined;
    const id = setInterval(() => {
      loadSelectedRun();
    }, refreshMs);
    return () => clearInterval(id);
  }, [refreshEnabled, refreshMs, selectedRunId, loadSelectedRun]);

  useEffect(() => {
    if (selectedOrder?.decision_id) {
      loadExplain(selectedOrder.decision_id);
    }
  }, [selectedOrder, loadExplain]);

  const startRunWithPayload = useCallback(
    async (payload) => {
      const bodyObj = payload ?? form;
      setStartLoading(true);
      setStartMsg('');
      try {
        let lastError = null;
        for (let attempt = 1; attempt <= 3; attempt += 1) {
          const r = await fetch(RUNS_URL, {
            method: 'POST',
            credentials: 'include',
            headers: {
              'Content-Type': 'application/json',
              Accept: 'application/json',
            },
            body: JSON.stringify(bodyObj),
          });
          const j = await r.json().catch(() => null);
          if (r.ok) {
            setStartMsg(`Run started: ${j.run_id}`);
            clearRunPanels();
            setSelectedRunId(j.run_id);
            await loadRuns();
            return;
          }
          const info = normalizeErrorPayload(j, r.status);
          lastError = info;
          if (info.retryable && attempt < 3) {
            setStartMsg(`Upstream market data unavailable, retrying (${attempt}/2)...`);
            await sleep(2000 * attempt);
            continue;
          }
          break;
        }
        const msg = lastError?.message || 'Unknown start failure';
        if (/not enough|selection produced too few symbols|aligned days/i.test(msg)) {
          setStartMsg(`Start failed: ${msg}. Reduce Days/Top symbols or retry when more market data is available.`);
        } else {
          setStartMsg(`Start failed: ${msg}`);
        }
      } catch (e) {
        setStartMsg(`Start failed: ${String(e?.message || e)}`);
      } finally {
        setStartLoading(false);
      }
    },
    [form, clearRunPanels, loadRuns]
  );

  const handleStart = () => startRunWithPayload(null);

  const resetLocalPage = useCallback(() => {
    clearRunPanels();
    setSelectedRunId('');
    setForm({ ...SIX_MONTH_LIVE_PRESET });
    setStartMsg('');
    setPurgeSecret('');
    loadRuns();
  }, [clearRunPanels, loadRuns]);

  const handlePurgeServer = useCallback(async () => {
    if (serverPurgeAvailable === false) {
      setStartMsg('Server purge is disabled: set SIMULATOR_PURGE_SECRET on stock-service and redeploy.');
      return;
    }
    if (!purgeSecret.trim()) {
      setStartMsg('Set the purge secret before erasing server runs.');
      return;
    }
    if (!window.confirm('Erase ALL simulator runs from the server Redis index? In-flight tasks may still write briefly.')) return;
    setPurgeLoading(true);
    setStartMsg('');
    try {
      const r = await fetch(PURGE_URL, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: JSON.stringify({ secret: purgeSecret }),
      });
      const j = await r.json().catch(() => null);
      if (!r.ok) {
        setStartMsg(`Purge failed: ${j?.error || r.status}`);
        return;
      }
      setStartMsg(`Server purge OK (${j?.runs_removed_from_index ?? 0} runs removed from index). Refreshing list…`);
      clearRunPanels();
      setSelectedRunId('');
      await loadRuns();
    } catch (e) {
      setStartMsg(`Purge failed: ${String(e?.message || e)}`);
    } finally {
      setPurgeLoading(false);
    }
  }, [purgeSecret, clearRunPanels, loadRuns, serverPurgeAvailable]);

  const handlePurgeAndStart = useCallback(async () => {
    if (serverPurgeAvailable === false) {
      setStartMsg('Server purge is disabled: set SIMULATOR_PURGE_SECRET on stock-service and redeploy.');
      return;
    }
    if (!purgeSecret.trim()) {
      setStartMsg('Set the purge secret before erase-and-start.');
      return;
    }
    if (!window.confirm('Erase ALL server simulator runs, then start a new 6-month LIVE run?')) return;
    setPurgeLoading(true);
    setStartMsg('');
    try {
      const r = await fetch(PURGE_URL, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: JSON.stringify({ secret: purgeSecret }),
      });
      const j = await r.json().catch(() => null);
      if (!r.ok) {
        setStartMsg(`Purge failed: ${j?.error || r.status}`);
        return;
      }
      setForm({ ...SIX_MONTH_LIVE_PRESET });
      clearRunPanels();
      setSelectedRunId('');
      await loadRuns();
      await startRunWithPayload({ ...SIX_MONTH_LIVE_PRESET });
    } catch (e) {
      setStartMsg(`Erase-and-start failed: ${String(e?.message || e)}`);
    } finally {
      setPurgeLoading(false);
    }
  }, [purgeSecret, clearRunPanels, loadRuns, startRunWithPayload, serverPurgeAvailable]);

  const handleSelectRun = useCallback((runId) => {
    if (!runId || runId === selectedRunId) return;
    clearRunPanels();
    setSelectedRunId(runId);
  }, [selectedRunId, clearRunPanels]);

  const sendControl = async (action) => {
    setControlMsg('');
    if (!selectedRunId) return;
    if (action === 'stop' && !window.confirm('Stop this run? This action cannot be undone.')) return;
    setControlLoading(true);
    try {
      const url = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/control`;
      const j = await fetchJson(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: JSON.stringify({ action }),
      });
      setControlMsg(`Action applied: ${j.action}`);
      await loadSelectedRun();
    } catch (e) {
      setControlMsg(`Control failed: ${String(e?.message || e)}`);
    } finally {
      setControlLoading(false);
    }
  };

  const refreshLabel = lastRefreshTs ? new Date(lastRefreshTs).toLocaleString('sv-SE', { hour12: false }) : '—';
  const decisionFeatures = explainData?.decision?.features || {};
  const blotterVisibleCount = Math.min(500, blotterOrders.length);

  return (
    <PageTemplate title="Realistic Trading Simulator" subtitle="Run control, live monitoring, and trade-level decision forensics">
      <p className="sim-note">
        <Link to="/stock" className="sim-back-link">← Back to Stock Service</Link>
      </p>
      <div className="sim-stage">
        <h2 className="sim-stage-title">1) Run Control</h2>
        <div className="sim-grid sim-grid--two">
          <section className="sim-card">
            <h3>Start New Simulation</h3>
            <p className="sim-note">
              Defaults target a <strong>6-month live campaign</strong> (126-day readiness window, 60s refresh, live quotes).
              Airflow keeps one active run unless you set <code>SIM_REPLAY_RESTART_POLICY=bankrupt_only</code> on the host.
            </p>
            <div className="sim-campaign-actions">
              <button type="button" className="sim-btn sim-btn--secondary" onClick={resetLocalPage} disabled={purgeLoading || startLoading}>
                Clear page &amp; reset form
              </button>
              <button type="button" className="sim-btn sim-btn--secondary" onClick={() => setForm({ ...SIX_MONTH_LIVE_PRESET })} disabled={purgeLoading || startLoading}>
                Apply 6-month live preset
              </button>
              <button type="button" className="sim-btn sim-btn--secondary" onClick={() => setForm({ ...RESEARCH_126_REPLAY_PRESET })} disabled={purgeLoading || startLoading}>
                126d research replay (IBKR daily)
              </button>
            </div>
            <p className="sim-control-hint">
              Research replay uses strict calendar alignment across symbols. If start fails with <code>replay_require_full_horizon</code>, reduce Top symbols or disable the checkbox below.
              Export training/bias JSON from a completed run: <strong>Research export</strong> in section 3.
            </p>
            <div className="sim-form-grid">
              <label>Days
                <input type="number" min="7" max="365" value={form.days} onChange={(e) => setForm((f) => ({ ...f, days: Number(e.target.value || 7) }))} />
              </label>
              <label>Top symbols
                <input type="number" min="6" max="40" value={form.top} onChange={(e) => setForm((f) => ({ ...f, top: Number(e.target.value || 16) }))} />
              </label>
              <label>Quantile
                <input type="number" step="0.01" min="0.05" max="0.45" value={form.quantile} onChange={(e) => setForm((f) => ({ ...f, quantile: Number(e.target.value || 0.2) }))} />
              </label>
              <label>Initial capital (USD)
                <input type="number" min="25" step="1" value={form.initial_capital_usd} onChange={(e) => setForm((f) => ({ ...f, initial_capital_usd: Number(e.target.value || 100) }))} />
              </label>
              <label>Replay delay (ms)
                <input type="number" min="0" step="50" value={form.replay_delay_ms} onChange={(e) => setForm((f) => ({ ...f, replay_delay_ms: Number(e.target.value || 250) }))} />
              </label>
              <label>Max cycles
                <input type="number" min="1000" max="1000000" step="1000" value={form.max_cycles} onChange={(e) => setForm((f) => ({ ...f, max_cycles: Number(e.target.value || 500000) }))} />
              </label>
              <label>Readiness min days
                <input type="number" min="7" max="365" value={form.readiness_min_days} onChange={(e) => setForm((f) => ({ ...f, readiness_min_days: Number(e.target.value || 126) }))} />
              </label>
              <label className="sim-check">
                <input type="checkbox" checked={form.short_enabled} onChange={(e) => setForm((f) => ({ ...f, short_enabled: e.target.checked }))} />
                Allow short side
              </label>
              <label className="sim-check">
                <input type="checkbox" checked={form.reinvest_profit} onChange={(e) => setForm((f) => ({ ...f, reinvest_profit: e.target.checked }))} />
                Reinvest PnL
              </label>
              <label className="sim-check">
                <input type="checkbox" checked={form.live_market_data} onChange={(e) => setForm((f) => ({ ...f, live_market_data: e.target.checked }))} />
                Live market data (real-time quotes)
              </label>
              <label className="sim-check">
                <input
                  type="checkbox"
                  checked={Boolean(form.live_require_bid_ask)}
                  onChange={(e) => setForm((f) => ({ ...f, live_require_bid_ask: e.target.checked }))}
                  disabled={!form.live_market_data}
                />
                Live: require bid/ask (skip if missing)
              </label>
              <label className="sim-check">
                <input
                  type="checkbox"
                  checked={Boolean(form.replay_require_full_horizon)}
                  onChange={(e) => setForm((f) => ({ ...f, replay_require_full_horizon: e.target.checked }))}
                  disabled={form.live_market_data}
                />
                Require full replay horizon for replay (server fails start if aligned IBKR days &lt; Days; recommended for 126d model runs)
              </label>
              <label>Live: max quote age (seconds)
                <input
                  type="number"
                  min="0"
                  max="600"
                  value={Number(form.live_max_quote_age_seconds || 0)}
                  onChange={(e) => setForm((f) => ({ ...f, live_max_quote_age_seconds: Number(e.target.value || 0) }))}
                  disabled={!form.live_market_data}
                />
              </label>
              <label>Live: max spread (bps)
                <input
                  type="number"
                  min="0"
                  max="2000"
                  step="1"
                  value={Number(form.live_max_spread_bps || 0)}
                  onChange={(e) => setForm((f) => ({ ...f, live_max_spread_bps: Number(e.target.value || 0) }))}
                  disabled={!form.live_market_data}
                />
              </label>
            </div>
            <div className="sim-purge-panel">
              {serverPurgeAvailable === false && (
                <p className="sim-error">
                  Server run history cannot be wiped here: <code>SIMULATOR_PURGE_SECRET</code> is not set on stock-service.
                  Use <strong>Clear page &amp; reset form</strong> to reset only this browser view, or set the secret and redeploy to enable erase.
                </p>
              )}
              <p className="sim-control-hint">
                To wipe the server run list, set env <code>SIMULATOR_PURGE_SECRET</code> on stock-service, enter it here, then erase.
              </p>
              <div className="sim-purge-panel__row">
                <input
                  className="sim-purge-input"
                  type="password"
                  autoComplete="off"
                  placeholder="Purge secret (server)"
                  value={purgeSecret}
                  onChange={(e) => setPurgeSecret(e.target.value)}
                  disabled={purgeLoading || startLoading || serverPurgeAvailable === false}
                />
                <button
                  type="button"
                  className="sim-btn sim-btn--secondary"
                  disabled={purgeLoading || startLoading || serverPurgeAvailable === false}
                  onClick={handlePurgeServer}
                >
                  {purgeLoading ? 'Working…' : 'Erase all runs (server)'}
                </button>
                <button
                  type="button"
                  className="sim-btn"
                  disabled={purgeLoading || startLoading || serverPurgeAvailable === false}
                  onClick={handlePurgeAndStart}
                >
                  Erase all &amp; start 6-month live
                </button>
              </div>
            </div>
            <button className="sim-btn" disabled={startLoading || purgeLoading} onClick={handleStart}>
              {startLoading ? 'Starting...' : 'Start Run'}
            </button>
            {startMsg && <p className="sim-note">{startMsg}</p>}
          </section>

          <section className="sim-card">
            <div className="sim-row-head">
              <h3>Runs</h3>
              <button className="sim-btn sim-btn--secondary" type="button" onClick={loadRuns}>Refresh Runs</button>
            </div>
            {runsLoading && <p className="sim-note">Loading runs...</p>}
            {runsError && <p className="sim-error">{runsError}</p>}
            {!runsLoading && runs.length === 0 && <p className="sim-note">No runs yet.</p>}
            <div className="sim-runs-list">
              {runs.map((r) => {
                const tone = statusTone(r.status);
                return (
                  <button
                    key={r.run_id}
                    className={`sim-run-row ${selectedRunId === r.run_id ? 'active' : ''}`}
                    onClick={() => handleSelectRun(r.run_id)}
                  >
                    <span className="sim-run-id">{r.run_id}</span>
                    <span className={`sim-pill sim-pill--${tone}`}>{r.status}</span>
                    <span>{fmtTs(r.created_at_ts)}</span>
                  </button>
                );
              })}
            </div>
          </section>
        </div>
      </div>

      <div className="sim-stage">
        <h2 className="sim-stage-title">2) Live Monitoring</h2>
        <section className="sim-card">
          <div className="sim-row-head sim-row-head--wrap">
            <h3>Current Run Status</h3>
            <div className="sim-monitor-bar" role="presentation">
              <div className="sim-control-group" role="group" aria-label="Run data refresh">
                <span className="sim-control-group__title">Run data</span>
                <div className="sim-control-group__row">
                  <label className="sim-check sim-check--spaced" htmlFor="sim-auto-refresh">
                    <input
                      id="sim-auto-refresh"
                      type="checkbox"
                      checked={refreshEnabled}
                      onChange={(e) => setRefreshEnabled(e.target.checked)}
                    />
                    Refresh automatically
                  </label>
                  <label className="sim-inline-field" htmlFor="sim-refresh-interval">
                    <span className="sim-inline-field__label">Interval</span>
                    <select
                      id="sim-refresh-interval"
                      value={refreshMs}
                      onChange={(e) => setRefreshMs(Number(e.target.value || 60000))}
                      aria-describedby="sim-refresh-hint"
                    >
                      <option value={30000}>Every 30 seconds</option>
                      <option value={60000}>Every 1 minute</option>
                      <option value={120000}>Every 2 minutes</option>
                    </select>
                  </label>
                  <button className="sim-btn sim-btn--secondary" type="button" onClick={loadSelectedRun}>
                    {isRefreshing ? 'Refreshing…' : 'Refresh now'}
                  </button>
                </div>
                <p id="sim-refresh-hint" className="sim-control-hint">
                  Updates status, holdings, timeline, orders, and risk for the run you selected in the list above. Last fetched:{' '}
                  {lastRefreshTs ? (
                    <time dateTime={new Date(lastRefreshTs).toISOString()}>{refreshLabel}</time>
                  ) : (
                    refreshLabel
                  )}
                </p>
              </div>
              <div className="sim-control-group" role="group" aria-label="Timeline filter">
                <span className="sim-control-group__title">Timeline</span>
                <div className="sim-control-group__row">
                  <label className="sim-inline-field sim-inline-field--grow" htmlFor="sim-exchange-filter">
                    <span className="sim-inline-field__label">Show events for</span>
                    <select
                      id="sim-exchange-filter"
                      value={exchangeFilter}
                      onChange={(e) => setExchangeFilter(e.target.value)}
                    >
                      {exchanges.map((x) => (
                        <option key={x} value={x}>{x === 'ALL' ? 'All exchanges' : x}</option>
                      ))}
                    </select>
                  </label>
                </div>
                <p className="sim-control-hint">Filters the event list only. Holdings and blotter still use the full run.</p>
              </div>
            </div>
          </div>

          {runStatusError && <p className="sim-error">{runStatusError}</p>}
          {!runStatus && !runStatusError && <p className="sim-note">Select a run to monitor.</p>}
          {runStatus && (
            <>
              {runLooksStuck && (
                <p className="sim-order-hint" role="status">
                  <strong>Stuck run?</strong> This run has been &quot;running&quot; for several minutes with zero completed steps.
                  The Airflow job also refuses to start another run while any run is active. Try <strong>Stop Run</strong>, then start again,
                  or pick a completed run above.
                </p>
              )}
              <div className="sim-status-grid">
                <div>Status: <b>{runStatus.status}</b></div>
                <div>Stop reason: <b>{runStatus.stop_reason || '—'}</b></div>
                <div>Data mode: <b>{runDataModeLabel}</b></div>
                <div>Initial: <b>${Number(runStatus.initial_capital_usd || 0).toFixed(2)}</b></div>
                <div>{runStatus.status === 'running' ? 'Current equity' : 'Ending'}:{' '}
                  <b>{runStatus.ending_equity_usd != null ? `$${Number(runStatus.ending_equity_usd).toFixed(2)}` : '—'}</b></div>
                <div>PnL: <b>{runStatus.pnl_usd != null ? `$${Number(runStatus.pnl_usd).toFixed(2)}` : '—'}</b></div>
                <div>Steps completed: <b>{Number(runStatus.cycles_completed || 0)}</b></div>
                {runStatus.live_market_data === true && Number(runStatus.live_campaign_deadline_ts || 0) > 0 && (
                  <div>
                    LIVE wall-clock deadline (UTC):{' '}
                    <b>{fmtTs(runStatus.live_campaign_deadline_ts)}</b>
                    {runStatus.live_campaign_calendar_days != null && (
                      <>
                        {' '}
                        · <b>{runStatus.live_campaign_calendar_days}</b> calendar days from start
                      </>
                    )}
                  </div>
                )}
                {runStatus.replay_aligned_trading_days_available != null && runStatus.live_market_data === false && (
                  <div>
                    IBKR aligned days (max): <b>{runStatus.replay_aligned_trading_days_available}</b>
                    {runStatus.replay_require_full_horizon ? ' · full horizon required' : ''}
                  </div>
                )}
                {runStatus.live_market_data === false &&
                  runStatus.replay_steps_total != null &&
                  Number(runStatus.replay_steps_total) > 0 && (
                    <>
                      <div>
                        Replay bar (step):{' '}
                        <b>
                          {runStatus.replay_step_current != null ? runStatus.replay_step_current : '—'} /{' '}
                          {runStatus.replay_steps_total}
                        </b>
                      </div>
                      <div>
                        Bar trading date (UTC): <b>{runStatus.replay_trading_date_utc || '—'}</b>
                      </div>
                    </>
                  )}
                <div>Readiness score: <b>{runStatus.readiness_score != null ? Number(runStatus.readiness_score).toFixed(3) : '—'}</b></div>
                <div>Readiness: <b>{runStatus.readiness_passed == null ? '—' : (runStatus.readiness_passed ? 'PASS' : 'NOT YET')}</b></div>
              </div>

              <div className="sim-control-group" role="group" aria-label="Recent execution summary">
                <span className="sim-control-group__title">Last 60 minutes</span>
                <div className="sim-control-group__row">
                  <span className="sim-pill sim-pill--neutral">
                    fills <b>{recentExecution.recentFillCount}</b>
                  </span>
                  <span className="sim-pill sim-pill--warn">
                    skipped <b>{recentExecution.recentSkipCount}</b>
                  </span>
                  <span className="sim-pill sim-pill--neutral">
                    decisions <b>{recentExecution.recentDecisionCount}</b>
                  </span>
                </div>
                {recentExecution.topSkipReasons.length > 0 && (
                  <p className="sim-control-hint">
                    Top skip reasons:{' '}
                    {recentExecution.topSkipReasons
                      .map(([reason, n]) => `${reason} (${n})`)
                      .join(' · ')}
                  </p>
                )}
                {recentExecution.recentFillCount > 0 && (
                  <p className="sim-control-hint">
                    Latest fill:{' '}
                    <b>
                      {fmtTs(
                        runStatus?.live_market_data === true
                          ? (recentExecution.recentFills[0]?.wall_clock_ts || recentExecution.recentFills[0]?.ts)
                          : recentExecution.recentFills[0]?.ts
                      )}
                    </b>
                    {' '}· {recentExecution.recentFills[0]?.symbol} · {sideActionLabel(recentExecution.recentFills[0]?.side)} · $
                    {Number(recentExecution.recentFills[0]?.filled_notional_usd || 0).toFixed(2)}
                  </p>
                )}
              </div>

              <div className="sim-control-row">
                <button className="sim-btn sim-btn--secondary" type="button" disabled={controlLoading} onClick={() => sendControl('pause')}>Pause</button>
                <button className="sim-btn sim-btn--secondary" type="button" disabled={controlLoading} onClick={() => sendControl('resume')}>Resume</button>
                <button className="sim-btn sim-btn-danger" type="button" disabled={controlLoading} onClick={() => sendControl('stop')}>Stop Run</button>
                {controlMsg && <span className="sim-note">{controlMsg}</span>}
              </div>

              <h3 className="sim-subtitle">Portfolio (simulated holdings)</h3>
              {holdingsError && <p className="sim-error">{holdingsError}</p>}
              {!holdingsError && !holdings && runStatus?.status === 'running' && (
                <p className="sim-note">Holdings appear after the first cycle completes (usually within one refresh interval).</p>
              )}
              {!holdingsError && !holdings && runStatus?.status && runStatus.status !== 'running' && (
                <p className="sim-note">No holdings snapshot for this run (older run or no completed cycles).</p>
              )}
              {holdings && (
                <>
                  <div className="sim-holdings-meta sim-status-grid">
                    <div>As of: <b>{fmtTs(holdings.ts)}</b></div>
                    <div>Step: <b>{Number(holdings.cycle_index || 0)}</b></div>
                    <div>Equity: <b>${Number(holdings.equity_usd || 0).toFixed(2)}</b></div>
                    <div>Deployed (sim): <b>${Number(holdings.deployed_usd || 0).toFixed(2)}</b></div>
                    <div>Unallocated: <b>${Number(holdings.cash_residual_usd || 0).toFixed(2)}</b></div>
                    <div>Universe: <b>{Number(holdings.universe_symbol_count || 0)}</b> symbols</div>
                  </div>
                  <div className="sim-control-group" role="group" aria-label="Portfolio view controls">
                    <span className="sim-control-group__title">View</span>
                    <div className="sim-control-group__row">
                      <label className="sim-field" htmlFor="sim-portfolio-filter">
                        <span className="sim-field__label">Filter</span>
                        <input
                          id="sim-portfolio-filter"
                          className="sim-input"
                          value={portfolioQuery}
                          onChange={(e) => setPortfolioQuery(e.target.value)}
                          placeholder="Symbol / exchange / side (e.g. NOK, XNAS, SHORT)"
                        />
                      </label>
                      <span className="sim-pill sim-pill--neutral">
                        long <b>${Number(portfolio.totals.long || 0).toFixed(2)}</b>
                      </span>
                      <span className="sim-pill sim-pill--neutral">
                        short <b>${Number(portfolio.totals.short || 0).toFixed(2)}</b>
                      </span>
                    </div>
                    {portfolioQuery && (
                      <p className="sim-control-hint">
                        Showing <strong>{portfolio.legs.length}</strong> / {holdings.legs?.length || 0} legs.
                      </p>
                    )}
                  </div>
                  <p className="sim-note sim-holdings-note">{holdings.note || ''}</p>
                  {!holdings.legs?.length && (
                    <p className="sim-note">No filled legs this cycle (skipped, rejected, or market closed).</p>
                  )}
                  {!!holdings.legs?.length && (
                    <div className="sim-holdings-wrap">
                      <table className="sim-holdings-table">
                        <thead>
                          <tr>
                            <th>Symbol</th>
                            <th>Exch</th>
                            <th>Side</th>
                            <th className="sim-num">Notional</th>
                            <th className="sim-num">Mark</th>
                            <th className="sim-num">~Shares</th>
                            <th className="sim-num">% Deployed</th>
                          </tr>
                        </thead>
                        <tbody>
                          {portfolio.legs.map((row, idx) => (
                            <tr key={`${row.symbol}-${row.exchange}-${row.side}-${idx}`}>
                              <td><b>{row.symbol}</b></td>
                              <td>{row.exchange}</td>
                              <td>
                                <span className={`sim-pill sim-pill--${String(row.side).includes('SHORT') ? 'bad' : 'good'}`}>
                                  {sideActionLabel(row.side)}
                                </span>
                              </td>
                              <td className="sim-num">${Number(row.notional_usd || 0).toFixed(2)}</td>
                              <td className="sim-num">${Number(row.mark_px || 0).toFixed(4)}</td>
                              <td className="sim-num">{Number(row.shares_approx || 0).toFixed(4)}</td>
                              <td className="sim-num">
                                {((100 * Math.abs(Number(row.notional_usd || 0))) / holdingsDeployedTotal).toFixed(1)}%
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                      <div className="sim-holdings-chart" aria-hidden="true">
                        <div className="sim-holdings-chart-title">Share of deployed notional (this cycle)</div>
                        {portfolio.legs.map((row, idx) => {
                          const w = (100 * Number(row.notional_usd || 0)) / holdingsDeployedTotal;
                          const isShort = String(row.side).includes('SHORT');
                          return (
                            <div key={`bar-${row.symbol}-${row.exchange}-${row.side}-${idx}`} className="sim-holdings-bar-row">
                              <span className="sim-holdings-bar-label">{row.symbol}</span>
                              <div className="sim-holdings-bar-track">
                                <div
                                  className={`sim-holdings-bar-fill ${isShort ? 'sim-holdings-bar-fill--short' : ''}`}
                                  style={{ width: `${Math.min(100, Math.max(2, w))}%` }}
                                />
                              </div>
                              <span className="sim-holdings-bar-val">${Number(row.notional_usd || 0).toFixed(2)}</span>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </>
              )}
            </>
          )}
        </section>

        <div className="sim-grid sim-grid--two">
          <section className="sim-card">
            <h3>Timeline</h3>
            {timelineError && <p className="sim-error">{timelineError}</p>}
            <div className="sim-control-group" role="group" aria-label="Timeline visibility">
              <span className="sim-control-group__title">View</span>
              <div className="sim-control-group__row">
                <label className="sim-check sim-check--spaced" htmlFor="sim-timeline-hide-noise">
                  <input
                    id="sim-timeline-hide-noise"
                    type="checkbox"
                    checked={hideNoisyTimeline}
                    onChange={(e) => setHideNoisyTimeline(e.target.checked)}
                  />
                  Hide per-cycle snapshot noise (MarketSnapshotSeen / PortfolioValued / RiskSnapshot / HoldingsSnapshot)
                </label>
              </div>
              <p className="sim-control-hint">
                Showing <strong>{filteredTimeline.length}</strong> / {timeline.length} events from the server.
              </p>
            </div>
            {!timelineError && filteredTimeline.length === 0 && <p className="sim-note">No events in this view yet.</p>}
            <div className="sim-timeline">
              {filteredTimeline.map((e) => (
                <div key={e.event_id} className="sim-event-row">
                  <span>{fmtTs(e.ts)}</span>
                  <span>{e.kind}</span>
                  <span>{e.exchange || 'GLOBAL'}</span>
                </div>
              ))}
            </div>
          </section>

          <section className="sim-card">
            <h3>Risk and Readiness Snapshot</h3>
            {riskError && <p className="sim-error">{riskError}</p>}
            {readinessError && <p className="sim-error">{readinessError}</p>}
            {!latestRisk && !riskError && <p className="sim-note">No risk snapshots yet.</p>}
            {latestRisk && (
              <div className="sim-status-grid">
                <div>Equity: <b>${Number(latestRisk.equity_usd || 0).toFixed(2)}</b></div>
                <div>Benchmark: <b>${Number(latestRisk.benchmark_equity_usd || 0).toFixed(2)}</b></div>
                <div>Drawdown: <b>{(100 * Number(latestRisk.drawdown || 0)).toFixed(2)}%</b></div>
                <div>VaR(95%,1d): <b>{(100 * Number(latestRisk.var_95_1d || 0)).toFixed(2)}%</b></div>
                <div>Leverage: <b>{Number(latestRisk.leverage || 0).toFixed(2)}x</b></div>
                <div>Relative return: <b>{(100 * Number(latestRisk.relative_return || 0)).toFixed(2)}%</b></div>
              </div>
            )}
            {readiness && (
              <>
                <div className="sim-readiness-head">
                  <span className={`sim-pill sim-pill--${readiness.pass ? 'good' : 'warn'}`}>
                    {readiness.pass ? 'Gate Pass' : 'Gate Not Passed'}
                  </span>
                  <span className="sim-note">
                    Score {Number(readiness.score || 0).toFixed(3)} • Steps {readiness.evaluated_days}/
                    {readiness.min_days_required} (gate uses step count; for replay one step ≈ one aligned trading day)
                  </span>
                </div>
                <div className="sim-note"><b>Recommendation:</b> {readiness.recommendation}</div>
              </>
            )}
          </section>
        </div>
      </div>

      <div className="sim-stage">
        <h2 className="sim-stage-title">3) Trade Forensics</h2>
        <div className="sim-grid sim-grid--two">
          <section className="sim-card">
            <h3>Trade Blotter</h3>
            <div className="sim-blotter-controls">
              <div className="sim-control-group" role="group" aria-label="Blotter filter">
                <span className="sim-control-group__title">Filter</span>
                <div className="sim-control-group__row">
                  <label className="sim-check sim-check--spaced" htmlFor="sim-blotter-today">
                    <input
                      id="sim-blotter-today"
                      type="checkbox"
                      checked={onlyTodayOrders}
                      onChange={(e) => setOnlyTodayOrders(e.target.checked)}
                    />
                    {runStatus?.live_market_data === false
                      ? 'Latest replay bar only'
                      : "Today's trades only"}
                  </label>
                </div>
                <p id="sim-blotter-filter-hint" className="sim-control-hint">
                  {runStatus?.live_market_data === false
                    ? 'Replay mode: filters by the latest simulated trading day (UTC date of order ts). Wall clock is not used.'
                    : "Live mode: uses your device's local calendar on wall-clock time when present."}
                </p>
              </div>
              <p className="sim-blotter-count" aria-live="polite">
                <strong>{blotterVisibleCount}</strong> row{blotterVisibleCount !== 1 ? 's' : ''} in this list
                {blotterOrders.length > blotterVisibleCount && ` (cap 500)`}
                {' · '}
                <strong>{blotterOrders.length}</strong> after filter
                {' · '}
                <strong>{orders.length}</strong> loaded from server
              </p>
            </div>
            {ordersError && <p className="sim-error">{ordersError}</p>}
            {!ordersError && blotterOrders.length === 0 && <p className="sim-note">No orders in this view yet.</p>}
            <div className="sim-runs-list sim-runs-list--tall">
              {blotterOrders.slice(0, 500).map((o) => {
                const key = `${o.order_id}-${o.ts}-${o.status}`;
                const tone = statusTone(o.status);
                return (
                  <button
                    key={key}
                    className={`sim-run-row ${selectedOrderKey === key ? 'active' : ''}`}
                    onClick={() => setSelectedOrderKey(key)}
                  >
                    <span>
                      {fmtTs(runStatus?.live_market_data === false ? o.ts : (o.wall_clock_ts || o.ts))} · {o.symbol} ·{' '}
                      {sideActionLabel(o.side)}
                    </span>
                    <span className={`sim-pill sim-pill--${tone}`}>{o.status}</span>
                    <span>${Number(o.filled_notional_usd || 0).toFixed(2)}</span>
                  </button>
                );
              })}
            </div>
          </section>

          <section className="sim-card">
            <h3>Decision Dossier</h3>
            {!selectedOrder && <p className="sim-note">Select a trade from blotter.</p>}
            {selectedOrder && explainData?.run && (explainData.run.live_market_data === true || explainData.run.live_market_data === false) && (
              <div
                className={`sim-data-mode-banner ${explainData.run.live_market_data ? '' : 'sim-data-mode-banner--replay'}`}
                role="status"
              >
                {explainData.run.live_market_data ? (
                  <>
                    <strong>LIVE run:</strong> execution times and prices follow the simulator step clock and watchlist quotes.
                  </>
                ) : (
                  <>
                    <strong>REPLAY run:</strong> buy/sell times show the exchange session template (e.g. 08:00–16:30 UTC-style)
                    and OHLC comes from historical daily bars — not current market time.
                  </>
                )}
              </div>
            )}
            {selectedOrder && String(selectedOrder.status || '').toLowerCase() === 'rejected' && (
              <p className="sim-order-hint">
                <strong>No execution:</strong> this order was rejected or had zero fill (for example short borrow unavailable).
                Execution lines below only apply to filled or partially filled orders.
              </p>
            )}
            {selectedOrder && (
              <>
                <div className="sim-status-grid">
                  <div>Order: <b>{selectedOrder.order_id}</b></div>
                  <div>Decision: <b>{selectedOrder.decision_id}</b></div>
                  <div>Symbol: <b>{selectedOrder.symbol}</b></div>
                  <div>Exchange: <b>{selectedOrder.exchange}</b></div>
                  <div>Type/TIF: <b>{selectedOrder.order_type} / {selectedOrder.tif}</b></div>
                  <div>Notional: <b>${Number(selectedOrder.qty_notional_usd || 0).toFixed(2)}</b></div>
                </div>
                <div className="sim-note"><b>Order rationale:</b> {(selectedOrder.reasons || []).join(' | ') || '—'}</div>
              </>
            )}
            {explainError && <p className="sim-error">{explainError}</p>}
            {!explainError && !explainData && selectedOrder && <p className="sim-note">Loading explainability...</p>}
            {explainData && (
              <>
                <div className="sim-status-grid">
                  <div>Position side: <b>{sideActionLabel(explainData?.decision?.side)}</b></div>
                  <div>Hybrid score: <b>{Number(explainData?.decision?.hybrid_score || 0).toFixed(4)}</b></div>
                  <div>Decision time: <b>{fmtTs(explainData?.decision?.decision_ts)}</b></div>
                  <div>Run: <b>{explainData?.run?.run_id || '—'}</b></div>
                </div>
                <div className="sim-subtitle">Feature Inputs</div>
                <div className="sim-keyvals">
                  {Object.entries(decisionFeatures).map(([k, v]) => (
                    <div key={k} className="sim-kv-row">
                      <span>{k}</span>
                      <span>{typeof v === 'number' ? Number(v).toFixed(6) : String(v)}</span>
                    </div>
                  ))}
                  {Object.keys(decisionFeatures).length === 0 && <p className="sim-note">No feature payload.</p>}
                </div>
                <div className="sim-subtitle">Model Rationale</div>
                {explainData?.decision?.short_explanation && (
                  <p className="sim-note"><b>Short explanation:</b> {explainData.decision.short_explanation}</p>
                )}
                <ul className="sim-list">
                  {(explainData?.decision?.rationale || []).map((r) => <li key={r}>{r}</li>)}
                  {(explainData?.decision?.rationale || []).length === 0 && <li>No rationale entries.</li>}
                </ul>
                  <div className="sim-subtitle">Execution Attribution</div>
                <div className="sim-keyvals">
                  {(explainData?.fills || []).map((f) => {
                    const buyPx = Number(f.buy_px) > 0 ? Number(f.buy_px) : Number(f.open_px || 0);
                    const sellPx = Number(f.sell_px) > 0 ? Number(f.sell_px) : Number(f.close_px || 0);
                    const isShort = String(explainData?.decision?.side || '').toUpperCase() === 'SHORT';
                    const ibkr = Number(f.ibkr_commission_usd || 0);
                    const ex = Number(f.exchange_fee_usd || 0);
                    const clear = Number(f.clearing_fee_usd || 0);
                    const reg = Number(f.regulatory_fee_usd || 0);
                    const fx = Number(f.fx_fee_usd || 0);
                    const tax = Number(f.tax_usd || 0);
                    const slip = Number(f.slippage_usd || 0);
                    const impact = Number(f.market_impact_usd || 0);
                    const borrow = Number(f.borrow_fee_usd || 0);
                    const totalCost = Number.isFinite(Number(f.total_cost_usd))
                      ? Number(f.total_cost_usd || 0)
                      : (ibkr + ex + clear + reg + fx + tax + slip + impact + borrow);
                    const legLabel = isShort
                      ? `Short sell $${sellPx.toFixed(4)} (${f.sell_session_time_utc || 'close'}) → cover buy $${buyPx.toFixed(4)} (${f.buy_session_time_utc || 'open'})`
                      : `Buy $${buyPx.toFixed(4)} (${f.buy_session_time_utc || 'open'}) → sell $${sellPx.toFixed(4)} (${f.sell_session_time_utc || 'close'})`;
                    return (
                      <div key={`${f.exec_ts}-${f.order_id || f.decision_id}`} className="sim-kv-row">
                        <span>{fmtTs(f.exec_ts)} · {f.order_id || f.decision_id}</span>
                        <span>
                          {legLabel}
                          {' '}
                          | Notional ${Number(f.qty_notional_usd || 0).toFixed(2)} |
                          PnL ${Number(f.pnl_usd || 0).toFixed(2)} |
                          total cost ${totalCost.toFixed(4)}
                          {' '}({`ibkr ${ibkr.toFixed(4)}, ex ${ex.toFixed(4)}, clear ${clear.toFixed(4)}, reg ${reg.toFixed(4)}, fx ${fx.toFixed(4)}, tax ${tax.toFixed(4)}, slip ${slip.toFixed(4)}, impact ${impact.toFixed(4)}, borrow ${borrow.toFixed(4)}`})
                        </span>
                      </div>
                    );
                  })}
                  {(explainData?.fills || []).length === 0 && <p className="sim-note">No fills linked to decision.</p>}
                </div>
              </>
            )}
          </section>
        </div>

        <div className="sim-grid sim-grid--two">
          <section className="sim-card">
            <h3>Bias and Data Quality Findings</h3>
            {selectedRunId && (
              <p className="sim-note">
                <a
                  className="sim-back-link"
                  href={`/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/research-export`}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Open research export (JSON)
                </a>
                {' — '}
                decisions, fills, event counts, bias findings for offline training and QA.
              </p>
            )}
            {biasError && <p className="sim-error">{biasError}</p>}
            {!biasError && biasFindings.length === 0 && <p className="sim-note">No findings in current run.</p>}
            {biasFindings.map((f, i) => (
              <div key={`${f.code}-${i}`} className="sim-finding">
                <div><b>{f.severity}</b> · {f.code}</div>
                <div>{f.message}</div>
              </div>
            ))}
          </section>

          <section className="sim-card">
            <h3>Risk Trail</h3>
            {riskError && <p className="sim-error">{riskError}</p>}
            {!riskError && riskSeries.length === 0 && <p className="sim-note">No risk snapshots yet.</p>}
            <div className="sim-runs-list">
              {riskSeries.slice(-120).reverse().map((r) => (
                <div key={`${r.ts}-${r.equity_usd}`} className="sim-event-row">
                  <span>{fmtTs(r.ts)}</span>
                  <span>Eq ${Number(r.equity_usd || 0).toFixed(2)} / Bm ${Number(r.benchmark_equity_usd || 0).toFixed(2)}</span>
                  <span>DD {(100 * Number(r.drawdown || 0)).toFixed(2)}%</span>
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </PageTemplate>
  );
}

