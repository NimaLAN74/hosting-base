/**
 * Stock service – watchlist: price table for chosen symbols (IBKR), refreshes every 60s.
 */
import React, { useEffect, useState, useCallback, useRef } from 'react';
import { Link } from 'react-router-dom';
import PageTemplate from '../PageTemplate';
import './StockServicePage.css';

const WATCHLIST_URL = '/api/v1/stock-service/watchlist';
const HISTORY_URL = '/api/v1/stock-service/history';
const TODAY_URL = '/api/v1/stock-service/today';
const DAILY_SIGNALS_MODEL_URL = '/api/v1/stock-service/daily-signals/model?train_days=120&quantile=0.2&short=true';
const DAILY_SIGNALS_PHASE1_URL = '/api/v1/stock-service/daily-signals?backtest=true';
const SELECTION_URL = '/api/v1/stock-service/selection?top=20&quantile=0.2&short=true&train_days=120';
const PAPER_TRADE_STATUS_URL = '/api/v1/stock-service/paper-trade/status';
const PAPER_TRADE_RECORDS_URL = '/api/v1/stock-service/paper-trade/records?limit=5';
const PAPER_TRADE_BACKFILL_URL = '/api/v1/stock-service/paper-trade/backfill?days=60&quantile=0.2&short_enabled=true&overwrite=true';
const PAPER_TRADE_RUN_URL = '/api/v1/stock-service/paper-trade/run?quantile=0.2&short_enabled=true&train_days=120';
const PAPER_TRADE_ORDER_PLAN_URL = '/api/v1/stock-service/paper-trade/order-plan?quantile=0.2&short_enabled=true&train_days=120';
const WATCHLIST_REFRESH_MS = 60_000;
const DAILY_SIGNALS_REFRESH_MS = 5 * 60_000;
const PAPER_TRADE_REFRESH_MS = 5 * 60_000;
const ORDER_PLAN_REFRESH_MS = 5 * 60_000;
const SELECTION_REFRESH_MS = 5 * 60_000;

/* ViewBox coordinates — SVG scales to 100% width of container via CSS */
const CHART_WIDTH = 960;
const CHART_HEIGHT = 280;
const PAD_LEFT = 52;
const PAD_RIGHT = 16;
const PAD_TOP = 16;
const PAD_BOTTOM = 36;
const PLOT_W = CHART_WIDTH - PAD_LEFT - PAD_RIGHT;
const PLOT_H = CHART_HEIGHT - PAD_TOP - PAD_BOTTOM;

const SYMBOL_SHORT_NAMES = {
  AAPL: 'Apple',
  MSFT: 'Microsoft',
  GOOGL: 'Alphabet',
  AMZN: 'Amazon',
  META: 'Meta',
  NVDA: 'NVIDIA',
  TSLA: 'Tesla',
  JPM: 'JPMorgan',
  V: 'Visa',
  JNJ: 'Johnson & Johnson',
};

/** Bar `t` from API: backend sends Unix seconds; if older payloads were ms, normalize for charts. */
const barTimeToMs = (t) => {
  const n = Number(t);
  if (!Number.isFinite(n) || n <= 0) return Date.now();
  return n > 1e11 ? n : n * 1000;
};

const formatSvDateTime24h = (isoLike) => {
  if (!isoLike) return '—';
  const d = new Date(isoLike);
  if (Number.isNaN(d.getTime())) return String(isoLike);
  return new Intl.DateTimeFormat('sv-SE', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(d);
};

const formatSignalReason = (reason) => {
  const map = {
    insufficient_symbols_with_history: 'Too few symbols have daily history from IBKR right now.',
    no_overlapping_trading_days: 'No shared trading days across symbols (likely market holiday / feed gap).',
    insufficient_overlapping_history_for_features: 'Not enough overlapping days to build 20-day features yet.',
    not_enough_symbols_after_alignment: 'Too few symbols left after aligning history windows.',
    feature_computation_unavailable: 'Feature calculation is unavailable for the latest bar (holiday/partial session).',
  };
  return map[reason] || reason || 'No signal data available.';
};

const pickTopCoefficientEntries = (coefficients) => {
  if (!coefficients || typeof coefficients !== 'object') return [];
  return Object.entries(coefficients)
    .filter(([k]) => k !== 'intercept')
    .map(([k, v]) => ({ key: k, value: Number(v || 0) }))
    .sort((a, b) => Math.abs(b.value) - Math.abs(a.value))
    .slice(0, 4);
};

export default function StockServicePage() {
  const [watchlist, setWatchlist] = useState(null);
  const [watchlistLoading, setWatchlistLoading] = useState(true);
  const [watchlistError, setWatchlistError] = useState(null);
  const previousPricesRef = useRef({});
  const lastPricesRef = useRef({});
  const sessionChartPointsRef = useRef({});
  const [expandedSymbol, setExpandedSymbol] = useState(null);
  const [historyRangeBySymbol, setHistoryRangeBySymbol] = useState({});
  const [historyBySymbol, setHistoryBySymbol] = useState({});
  const [historyLoading, setHistoryLoading] = useState(null);
  const [historyError, setHistoryError] = useState(null);
  const [todayBySymbol, setTodayBySymbol] = useState({});
  const [chartHover, setChartHover] = useState({ symbol: null, barIndex: null });
  const [dailySignals, setDailySignals] = useState(null);
  const [dailySignalsLoading, setDailySignalsLoading] = useState(true);
  const [dailySignalsError, setDailySignalsError] = useState(null);
  const [selection, setSelection] = useState(null);
  const [selectionLoading, setSelectionLoading] = useState(true);
  const [selectionError, setSelectionError] = useState(null);
  const [dailySignalsSource, setDailySignalsSource] = useState('model');
  const [showSignalsJson, setShowSignalsJson] = useState(false);

  const [paperTradeStatus, setPaperTradeStatus] = useState(null);
  const [paperTradeLoading, setPaperTradeLoading] = useState(true);
  const [paperTradeError, setPaperTradeError] = useState(null);

  const [paperTradeRecords, setPaperTradeRecords] = useState(null);
  const [paperTradeRecordsLoading, setPaperTradeRecordsLoading] = useState(true);
  const [paperTradeRecordsError, setPaperTradeRecordsError] = useState(null);
  const [paperTradeBackfillLoading, setPaperTradeBackfillLoading] = useState(false);
  const [paperTradeBackfillMsg, setPaperTradeBackfillMsg] = useState('');
  const [paperTradeRunLoading, setPaperTradeRunLoading] = useState(false);
  const [paperTradeRunMsg, setPaperTradeRunMsg] = useState('');
  const [paperTradePnlMode, setPaperTradePnlMode] = useState('net'); // 'net' | 'gross'

  const [paperTradeOrderPlan, setPaperTradeOrderPlan] = useState(null);
  const [paperTradeOrderPlanLoading, setPaperTradeOrderPlanLoading] = useState(false);
  const [paperTradeOrderPlanError, setPaperTradeOrderPlanError] = useState(null);
  const watchlistInFlightRef = useRef(false);
  const dailySignalsInFlightRef = useRef(false);

  const loadWatchlist = useCallback(() => {
    if (watchlistInFlightRef.current) return;
    watchlistInFlightRef.current = true;
    setWatchlistLoading(true);
    setWatchlistError(null);
    fetch(WATCHLIST_URL, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => {
        if (!r.ok) {
          setWatchlistError(r.status === 502 ? 'Stock service unavailable. Try again shortly.' : `Request failed (${r.status}).`);
          return null;
        }
        return r.json();
      })
      .then((data) => {
        if (data && data.symbols && data.symbols.length) {
          setWatchlist(data);
          if (data.symbols.every((s) => s.error)) {
            setWatchlistError('Prices not yet available from provider. Check back in a minute.');
          }
        } else {
          setWatchlist(data);
        }
      })
      .catch(() => {
        setWatchlist(null);
        setWatchlistError('Could not load watchlist. Check your connection and try again.');
      })
      .finally(() => {
        watchlistInFlightRef.current = false;
        setWatchlistLoading(false);
      });
  }, []);

  useEffect(() => {
    loadWatchlist();
    const id = setInterval(loadWatchlist, WATCHLIST_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadWatchlist]);

  const loadDailySignals = useCallback((opts = {}) => {
    if (dailySignalsInFlightRef.current) return;
    dailySignalsInFlightRef.current = true;
    const background = Boolean(opts.background);
    if (!background) setDailySignalsLoading(true);
    setDailySignalsError(null);
    setDailySignalsSource('model');
    fetch(DAILY_SIGNALS_MODEL_URL, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then(async (r) => {
        if (!r.ok) {
          throw new Error(`Model request failed (${r.status}).`);
        }
        const modelData = await r.json();
        // Prefer model output; if unavailable for current bar/session, fallback to phase-1 engine.
        if (modelData && modelData.data_available) {
          setDailySignals(modelData);
          setDailySignalsSource('model');
          return;
        }

        const fallbackResp = await fetch(DAILY_SIGNALS_PHASE1_URL, {
          credentials: 'include',
          headers: { Accept: 'application/json' },
        });
        if (!fallbackResp.ok) {
          // Keep model payload so reason is visible.
          setDailySignals(modelData);
          setDailySignalsSource('model');
          return;
        }
        const fallbackData = await fallbackResp.json();
        setDailySignals({
          ...fallbackData,
          fallback_reason: modelData?.reason || null,
          model_unavailable: true,
          model: modelData?.model || null,
          training_rows: modelData?.training_rows ?? null,
          coefficients: modelData?.coefficients || null,
          model_as_of_ts: modelData?.as_of_ts ?? null,
          model_reason: modelData?.reason || null,
        });
        setDailySignalsSource('phase1-fallback');
      })
      .catch(() => {
        setDailySignalsError('Could not load daily signals.');
        setDailySignals(null);
        setDailySignalsSource('model');
      })
      .finally(() => {
        dailySignalsInFlightRef.current = false;
        setDailySignalsLoading(false);
      });
  }, []);

  const loadSelection = useCallback(() => {
    setSelectionLoading(true);
    setSelectionError(null);
    fetch(SELECTION_URL, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then(async (r) => {
        const j = await r.json().catch(() => null);
        if (!r.ok) throw new Error(j?.error || `Selection request failed (${r.status}).`);
        return j;
      })
      .then((j) => setSelection(j))
      .catch((e) => {
        setSelection(null);
        setSelectionError(String(e?.message || e));
      })
      .finally(() => setSelectionLoading(false));
  }, []);

  const loadPaperTradeStatus = useCallback(() => {
    setPaperTradeLoading(true);
    setPaperTradeError(null);
    fetch(PAPER_TRADE_STATUS_URL, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then(async (r) => {
        if (!r.ok) {
          throw new Error(`Paper-trade status request failed (${r.status}).`);
        }
        return r.json();
      })
      .then((data) => setPaperTradeStatus(data))
      .catch(() => {
        // Endpoint may not exist right after deploy; keep UI non-blocking.
        setPaperTradeError(null);
        setPaperTradeStatus(null);
      })
      .finally(() => setPaperTradeLoading(false));
  }, []);

  const loadPaperTradeRecords = useCallback(() => {
    setPaperTradeRecordsLoading(true);
    setPaperTradeRecordsError(null);
    fetch(PAPER_TRADE_RECORDS_URL, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then(async (r) => {
        if (!r.ok) {
          throw new Error(`Paper-trade records request failed (${r.status}).`);
        }
        return r.json();
      })
      .then((data) => setPaperTradeRecords(data?.records || []))
      .catch(() => {
        // Keep UI non-blocking; this may fail right after deploy.
        setPaperTradeRecordsError(null);
        setPaperTradeRecords(null);
      })
      .finally(() => setPaperTradeRecordsLoading(false));
  }, []);

  useEffect(() => {
    loadDailySignals();
    const id = setInterval(() => loadDailySignals({ background: true }), DAILY_SIGNALS_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadDailySignals]);

  useEffect(() => {
    loadSelection();
    const id = setInterval(loadSelection, SELECTION_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadSelection]);

  useEffect(() => {
    loadPaperTradeStatus();
    const id = setInterval(loadPaperTradeStatus, PAPER_TRADE_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadPaperTradeStatus]);

  useEffect(() => {
    loadPaperTradeRecords();
    const id = setInterval(loadPaperTradeRecords, PAPER_TRADE_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadPaperTradeRecords]);

  useEffect(() => {
    if (watchlist?.symbols) {
      const next = {};
      const now = Date.now();
      const updatedPoints = { ...sessionChartPointsRef.current };
      watchlist.symbols.forEach((s) => {
        if (s.price != null) {
          const price = Number(s.price);
          next[s.symbol] = price;
          const prev = updatedPoints[s.symbol] || [];
          const pts = [...prev, { ts: now, price }];
          // keep last 120 points (~2h if 60s refresh)
          updatedPoints[s.symbol] = pts.slice(-120);
        }
      });
      previousPricesRef.current = lastPricesRef.current;
      lastPricesRef.current = next;
      sessionChartPointsRef.current = updatedPoints;
    }
  }, [watchlist]);

  const getRangeForSymbol = useCallback(
    (symbol) => historyRangeBySymbol[symbol] || '7d',
    [historyRangeBySymbol]
  );

  const expectedBarsMinByRange = {
    '7d': 3,
    '1m': 18,
    '3m': 45,
    '1y': 180,
  };

  // When a row is expanded, fetch IBKR history for that symbol/range (cached per symbol+range).
  useEffect(() => {
    if (!expandedSymbol) return;
    const sym = expandedSymbol;
    const range = getRangeForSymbol(sym);
    const key = `${sym}:${range}`;
    const existing = historyBySymbol[key];
    if (existing) return;
    const rangeToDays = (r) => {
      switch (r) {
        case '1m':
          return 30;
        case '3m':
          return 90;
        case '1y':
          return 365;
        case '7d':
        default:
          return 7;
      }
    };
    const days = rangeToDays(range);
    setHistoryLoading(sym);
    setHistoryError(null);
    const url = `${HISTORY_URL}?symbol=${encodeURIComponent(sym)}&range=${encodeURIComponent(range)}&days=${encodeURIComponent(days)}`;
    fetch(url, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => r.json())
      .then((res) => {
        if (res && res.error) {
          setHistoryBySymbol((prev) => ({ ...prev, [key]: null }));
          setHistoryError(res.error);
        } else {
          const data = Array.isArray(res?.data) ? res.data : [];
          setHistoryBySymbol((prev) => ({
            ...prev,
            [key]: {
              range,
              data,
              period: res.period,
              barCount: typeof res.bars === 'number' ? res.bars : data.length,
              expectedBarsMin: expectedBarsMinByRange[range] || 0,
            },
          }));
          setHistoryError(null);
        }
      })
      .catch((err) => {
        setHistoryBySymbol((prev) => ({ ...prev, [key]: null }));
        setHistoryError(err.message || 'Failed to load history');
      })
      .finally(() => setHistoryLoading((prev) => (prev === sym ? null : prev)));
  }, [expandedSymbol, getRangeForSymbol, historyBySymbol]);

  // When a row is expanded, fetch today's cached points from backend (Redis) for that symbol.
  useEffect(() => {
    if (!expandedSymbol) return;
    const sym = expandedSymbol;
    fetch(`${TODAY_URL}?symbol=${encodeURIComponent(sym)}`, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => {
        if (!r.ok) {
          console.warn('stock-service today:', r.status, r.statusText);
          return { data: [] };
        }
        return r.json();
      })
      .then((res) => {
        const data = Array.isArray(res?.data) ? res.data : [];
        setTodayBySymbol((prev) => ({ ...prev, [sym]: data }));
      })
      .catch(() => setTodayBySymbol((prev) => ({ ...prev, [sym]: [] })));
  }, [expandedSymbol]);

  const getChangeIndicator = (symbol, currentPrice) => {
    if (currentPrice == null) return null;
    const prev = previousPricesRef.current[symbol];
    if (prev == null) return '—';
    const cur = Number(currentPrice);
    if (cur > prev) return 'up';
    if (cur < prev) return 'down';
    return 'unchanged';
  };

  const getSessionPoints = (symbol) => sessionChartPointsRef.current[symbol] || [];

  const modelName = dailySignals?.model || null;
  const topCoefficientEntries = pickTopCoefficientEntries(dailySignals?.coefficients);
  const firstFeature = Array.isArray(dailySignals?.features) && dailySignals.features.length > 0
    ? dailySignals.features[0]
    : null;
  const coefficients = dailySignals?.coefficients || null;
  const modelHealth = dailySignals?.model_health || null;
  const publishSignals = dailySignals?.publish_signals ?? Boolean(dailySignals?.data_available);
  const paperTradeStats = React.useMemo(() => {
    const rows = Array.isArray(paperTradeRecords) ? paperTradeRecords : [];
    if (!rows.length) {
      return {
        count: 0,
        winRate: 0,
        avgReturn: 0,
        bestReturn: 0,
        worstReturn: 0,
      };
    }
    const rets = rows.map((r) => Number(
      paperTradePnlMode === 'gross'
        ? (r?.pnl_return_gross ?? r?.pnl_return ?? 0)
        : (r?.pnl_return_net ?? r?.pnl_return ?? 0)
    ));
    const wins = rets.filter((x) => x > 0).length;
    const avg = rets.reduce((a, b) => a + b, 0) / rets.length;
    return {
      count: rows.length,
      winRate: wins / rows.length,
      avgReturn: avg,
      bestReturn: Math.max(...rets),
      worstReturn: Math.min(...rets),
    };
  }, [paperTradeRecords, paperTradePnlMode]);
  const paperTradeEquity = React.useMemo(() => {
    const rows = (Array.isArray(paperTradeRecords) ? paperTradeRecords : []).slice().reverse();
    if (!rows.length) return [];
    let cumLn = 0;
    return rows.map((r, idx) => {
      const thisLn = Number(
        paperTradePnlMode === 'gross'
          ? (r?.pnl_ln_gross ?? r?.pnl_ln ?? 0)
          : (r?.pnl_ln_net ?? r?.pnl_ln ?? 0)
      );
      cumLn += thisLn;
      return {
        x: idx,
        y: Math.exp(cumLn) - 1,
        ts: Number(r?.execution_as_of_ts || 0),
      };
    });
  }, [paperTradeRecords, paperTradePnlMode]);

  const orderPlan = React.useMemo(() => {
    const signals = dailySignals?.signals || [];
    const prices = {};
    (watchlist?.symbols || []).forEach((s) => {
      if (s?.symbol && s?.price != null) prices[s.symbol] = Number(s.price);
    });
    const cap = Number(paperTradeStatus?.sizing_assumptions?.capital_usd || 10_000);
    const grossLong = Number(paperTradeStatus?.sizing_assumptions?.gross_long || 0.5);
    const grossShort = Number(paperTradeStatus?.sizing_assumptions?.gross_short || 0.5);
    const minNotional = Number(paperTradeStatus?.sizing_assumptions?.min_notional_usd || 0);
    const roundMode = String(paperTradeStatus?.sizing_assumptions?.share_rounding || 'none');
    const lotSize = Number(paperTradeStatus?.sizing_assumptions?.lot_size || 1);
    const roundShares = (raw) => {
      const x = Number(raw || 0);
      if (!Number.isFinite(x) || x <= 0) return 0;
      if (roundMode === 'whole') return Math.floor(x);
      if (roundMode === 'lot') {
        const lot = Math.max(1, lotSize || 1);
        return Math.floor(x / lot) * lot;
      }
      return x;
    };
    const rows = signals
      .filter((s) => s && (s.side === 'LONG' || s.side === 'SHORT'))
      .map((s) => {
        const px = prices[s.symbol] || null;
        const grossSide = s.side === 'SHORT' ? grossShort : grossLong;
        const notionalTarget = cap * grossSide * Number(s.weight || 0);
        const sharesRaw = px && px > 0 ? notionalTarget / px : null;
        const shares = sharesRaw != null ? roundShares(sharesRaw) : null;
        const notionalUsed = px && px > 0 && shares != null ? shares * px : null;
        const skipped = notionalUsed != null && minNotional > 0 ? notionalUsed < minNotional : false;
        return {
          symbol: s.symbol,
          side: s.side,
          weight: Number(s.weight || 0),
          price_est: px,
          notional_target: notionalTarget,
          notional_used: notionalUsed,
          shares,
          skipped,
        };
      });
    return { cap, grossLong, grossShort, minNotional, roundMode, lotSize, rows };
  }, [dailySignals, watchlist, paperTradeStatus]);

  const orderPlanView = React.useMemo(() => {
    const serverRows = Array.isArray(paperTradeOrderPlan?.rows) ? paperTradeOrderPlan.rows : null;
    const usingServer = Array.isArray(serverRows) && serverRows.length > 0;
    if (usingServer) {
      const s = paperTradeOrderPlan?.sizing_assumptions || {};
      return {
        source: 'server',
        watchlistAsOf: paperTradeOrderPlan?.watchlist_as_of || null,
        cap: Number(s.capital_usd || 0),
        grossLong: Number(s.gross_long || 0),
        grossShort: Number(s.gross_short || 0),
        minNotional: Number(s.min_notional_usd || 0),
        roundMode: String(s.share_rounding || 'none'),
        lotSize: Number(s.lot_size || 1),
        rows: serverRows.map((r) => ({
          symbol: r.symbol,
          side: r.side,
          weight: Number(r.weight || 0),
          price_est: r.price_est ?? null,
          notional_target: Number(r.target_usd || 0),
          notional_used: Number(r.used_usd || 0),
          shares: Number(r.shares || 0),
          skipped: !!r.skipped,
        })),
      };
    }
    return {
      source: 'estimate',
      watchlistAsOf: watchlist?.as_of || null,
      ...orderPlan,
    };
  }, [orderPlan, paperTradeOrderPlan, watchlist]);

  const loadPaperTradeOrderPlan = useCallback(() => {
    setPaperTradeOrderPlanLoading(true);
    setPaperTradeOrderPlanError(null);
    fetch(PAPER_TRADE_ORDER_PLAN_URL, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then(async (r) => {
        const j = await r.json().catch(() => null);
        if (!r.ok) throw new Error(j?.error || `HTTP ${r.status}`);
        return j;
      })
      .then((j) => setPaperTradeOrderPlan(j))
      .catch((e) => setPaperTradeOrderPlanError(String(e?.message || e)))
      .finally(() => setPaperTradeOrderPlanLoading(false));
  }, []);

  useEffect(() => {
    loadPaperTradeOrderPlan();
    const t = setInterval(loadPaperTradeOrderPlan, ORDER_PLAN_REFRESH_MS);
    return () => clearInterval(t);
  }, [loadPaperTradeOrderPlan]);

  const orderPlanExport = React.useMemo(() => {
    const serverRows = Array.isArray(paperTradeOrderPlan?.rows) ? paperTradeOrderPlan.rows : null;
    const localRows = orderPlan?.rows || [];
    const usingServer = Array.isArray(serverRows) && serverRows.length > 0;

    const longUsed = usingServer
      ? Number(paperTradeOrderPlan?.totals?.long_used_usd || 0)
      : localRows.filter((r) => r.side === 'LONG' && !r.skipped && r.notional_used != null).reduce((a, r) => a + Number(r.notional_used || 0), 0);
    const shortUsed = usingServer
      ? Number(paperTradeOrderPlan?.totals?.short_used_usd || 0)
      : localRows.filter((r) => r.side === 'SHORT' && !r.skipped && r.notional_used != null).reduce((a, r) => a + Number(r.notional_used || 0), 0);
    const net = usingServer
      ? Number(paperTradeOrderPlan?.totals?.net_exposure_usd || 0)
      : longUsed - shortUsed;
    const gross = usingServer
      ? Number(paperTradeOrderPlan?.totals?.gross_exposure_usd || 0)
      : longUsed + shortUsed;

    const payload = {
      generated_at: new Date().toISOString(),
      source: usingServer ? 'server' : 'estimate',
      watchlist_as_of: usingServer ? (paperTradeOrderPlan?.watchlist_as_of || null) : (watchlist?.as_of || null),
      capital_usd: usingServer ? Number(paperTradeOrderPlan?.sizing_assumptions?.capital_usd || 0) : Number(orderPlan?.cap || 0),
      gross_long: usingServer ? Number(paperTradeOrderPlan?.sizing_assumptions?.gross_long || 0) : Number(orderPlan?.grossLong || 0),
      gross_short: usingServer ? Number(paperTradeOrderPlan?.sizing_assumptions?.gross_short || 0) : Number(orderPlan?.grossShort || 0),
      share_rounding: usingServer ? (paperTradeOrderPlan?.sizing_assumptions?.share_rounding || 'none') : (orderPlan?.roundMode || 'none'),
      lot_size: usingServer ? Number(paperTradeOrderPlan?.sizing_assumptions?.lot_size || 1) : Number(orderPlan?.lotSize || 1),
      min_notional_usd: usingServer ? Number(paperTradeOrderPlan?.sizing_assumptions?.min_notional_usd || 0) : Number(orderPlan?.minNotional || 0),
      totals: {
        long_used_usd: longUsed,
        short_used_usd: shortUsed,
        net_exposure_usd: net,
        gross_exposure_usd: gross,
      },
      rows: usingServer
        ? serverRows.map((r) => ({
          symbol: r.symbol,
          side: r.side,
          weight: Number(r.weight || 0),
          est_price: r.price_est ?? null,
          target_usd: Number(r.target_usd || 0),
          used_usd: Number(r.used_usd || 0),
          est_shares: Number(r.shares || 0),
          status: r.skipped ? 'skip' : 'ok',
        }))
        : localRows.map((r) => ({
          symbol: r.symbol,
          side: r.side,
          weight: Number(r.weight || 0),
          est_price: r.price_est,
          target_usd: Number(r.notional_target || 0),
          used_usd: r.notional_used,
          est_shares: r.shares,
          status: r.skipped ? 'skip' : 'ok',
        })),
    };

    const esc = (v) => {
      const s = v == null ? '' : String(v);
      if (s.includes('"') || s.includes(',') || s.includes('\n')) return `"${s.replaceAll('"', '""')}"`;
      return s;
    };
    const header = ['symbol', 'side', 'weight', 'est_price', 'target_usd', 'used_usd', 'est_shares', 'status'];
    const lines = [header.join(',')];
    for (const r of payload.rows) {
      lines.push(header.map((k) => esc(r[k])).join(','));
    }
    // Add totals as comment-style lines (still valid CSV-ish for humans).
    lines.push(`# totals_long_used_usd,${longUsed.toFixed(2)}`);
    lines.push(`# totals_short_used_usd,${shortUsed.toFixed(2)}`);
    lines.push(`# totals_net_exposure_usd,${net.toFixed(2)}`);
    lines.push(`# totals_gross_exposure_usd,${gross.toFixed(2)}`);

    return {
      payload,
      csv: lines.join('\n') + '\n',
      totals: { longUsed, shortUsed, net, gross },
    };
  }, [orderPlan, paperTradeOrderPlan, watchlist]);

  const liveReadiness = React.useMemo(() => {
    const wlAsOf = watchlist?.as_of ? new Date(watchlist.as_of) : null;
    const wlAgeMs = wlAsOf && !Number.isNaN(wlAsOf.getTime()) ? Date.now() - wlAsOf.getTime() : null;
    const wlFresh = wlAgeMs != null ? wlAgeMs < 3 * 60_000 : false;
    const wlHasPrices = Array.isArray(watchlist?.symbols) ? watchlist.symbols.some((s) => s?.price != null && !s?.error) : false;
    const orderPlanReady = Boolean(wlFresh && wlHasPrices);
    const publishReady = Boolean(dailySignals?.publish_signals ?? dailySignals?.data_available);
    const paperOk = Boolean((paperTradeStatus?.execution_count ?? 0) >= 0);
    const ok = orderPlanReady && publishReady && paperOk;
    return {
      ok,
      orderPlanReady,
      publishReady,
      paperOk,
      wlFresh,
      wlHasPrices,
      wlAgeMs,
    };
  }, [watchlist, dailySignals, paperTradeStatus]);
  const featureAvailability = {
    rankFeatures: Boolean(
      (firstFeature && Object.prototype.hasOwnProperty.call(firstFeature, 'rank_mom5_cs'))
      || (coefficients && Object.prototype.hasOwnProperty.call(coefficients, 'rank_mom5_cs'))
    ),
    volRegime: Boolean(
      (firstFeature && Object.prototype.hasOwnProperty.call(firstFeature, 'vol_regime'))
      || (coefficients && Object.prototype.hasOwnProperty.call(coefficients, 'vol_regime'))
    ),
  };
  const dailySignalsJson = dailySignals ? JSON.stringify(dailySignals, null, 2) : '';

  /** Merged today points: Redis/cached (todayBySymbol) + current session (sessionChartPointsRef), sorted by ts. */
  const getMergedTodayPoints = (symbol) => {
    const cached = todayBySymbol[symbol] || [];
    const session = getSessionPoints(symbol);
    const cachedPts = cached.map((p) => ({ ts: Number(p.t) * 1000, price: Number(p.price) }));
    const sessionPts = session.map((p) => ({ ts: p.ts, price: p.price }));
    const byTs = {};
    [...cachedPts, ...sessionPts].forEach((p) => {
      if (!byTs[p.ts] || byTs[p.ts].price !== p.price) byTs[p.ts] = p;
    });
    return Object.values(byTs).sort((a, b) => a.ts - b.ts);
  };

  /** Long-period chart: one vertical bar per day (low→high), filled rect, green/red by close vs open. Tooltip shows O H L C. */
  const renderHistoryChart = (bars, isUp, symbol) => {
    if (!bars?.length) return <p className="stock-service-history-empty">No historical data.</p>;
    const barsNorm = bars.map((b) => ({
      t: barTimeToMs(b.t),
      o: Number(b.open ?? b.o ?? b.close ?? b.c ?? 0),
      h: Number(b.high ?? b.h ?? b.close ?? b.c ?? 0),
      l: Number(b.low ?? b.l ?? b.open ?? b.o ?? b.close ?? b.c ?? 0),
      c: Number(b.close ?? b.c ?? 0),
    }));
    const allPrices = barsNorm.flatMap((b) => [b.l, b.h]);
    const min = Math.min(...allPrices);
    const max = Math.max(...allPrices);
    const range = max - min || 1;
    const n = barsNorm.length;
    const stepX = n > 1 ? PLOT_W / (n - 1) : PLOT_W;
    const barW = Math.max(2, stepX * 0.6);
    const toY = (price) => PAD_TOP + PLOT_H - ((price - min) / range) * PLOT_H;

    const gridLines = [];
    for (let i = 0; i <= 4; i++) {
      const y = PAD_TOP + (PLOT_H * i) / 4;
      gridLines.push(<line key={`h${i}`} x1={PAD_LEFT} y1={y} x2={PAD_LEFT + PLOT_W} y2={y} className="stock-service-chart-grid" />);
    }
    for (let i = 0; i <= 4; i++) {
      const x = PAD_LEFT + (PLOT_W * i) / 4;
      gridLines.push(<line key={`v${i}`} x1={x} y1={PAD_TOP} x2={x} y2={PAD_TOP + PLOT_H} className="stock-service-chart-grid" />);
    }
    const yLabels = [max, min + range * 0.75, min + range * 0.5, min + range * 0.25, min].map((v, i) => (
      <text key={i} x={PAD_LEFT - 6} y={PAD_TOP + (PLOT_H * i) / 4 + 4} className="stock-service-chart-axis" textAnchor="end">{v.toFixed(2)}</text>
    ));
    const stepLabel = Math.max(1, Math.floor(n / 5));
    const xLabels = barsNorm
      .map((b, idx) => (idx % stepLabel === 0 || idx === n - 1 ? { b, idx } : null))
      .filter(Boolean)
      .map(({ b, idx }) => (
        <text key={idx} x={PAD_LEFT + idx * stepX} y={CHART_HEIGHT - 6} className="stock-service-chart-axis" textAnchor="middle">
          {new Date(b.t).toLocaleDateString('sv-SE', { month: 'short', day: 'numeric' })}
        </text>
      ));

    const barRects = barsNorm.map((b, idx) => {
      const x = PAD_LEFT + idx * stepX - barW / 2;
      const yH = toY(b.h);
      const yL = toY(b.l);
      const up = b.c >= b.o;
      const color = up ? '#0d9488' : '#dc2626';
      const dateStr = new Date(b.t).toLocaleDateString('sv-SE', { weekday: 'short', month: 'short', day: 'numeric', year: 'numeric' });
      const isHover = chartHover.symbol === symbol && chartHover.barIndex === idx;
      return (
        <g
          key={idx}
          onMouseEnter={() => setChartHover({ symbol, barIndex: idx })}
          onMouseLeave={() => setChartHover({ symbol: null, barIndex: null })}
        >
          <rect
            x={x}
            y={yH}
            width={barW}
            height={Math.max(1, yL - yH)}
            fill={color}
            stroke={isHover ? '#111' : 'none'}
            strokeWidth={1}
            className="stock-service-chart-bar"
          />
          {isHover && (
            <g className="stock-service-chart-tooltip">
              <rect x={PAD_LEFT + idx * stepX - 52} y={PAD_TOP} width={104} height={56} rx={4} fill="#1f2937" fillOpacity={0.95} />
              <text x={PAD_LEFT + idx * stepX} y={PAD_TOP + 14} textAnchor="middle" fill="#fff" fontSize={10}>{dateStr}</text>
              <text x={PAD_LEFT + idx * stepX} y={PAD_TOP + 28} textAnchor="middle" fill="#9ca3af" fontSize={9}>O {b.o.toFixed(2)}  H {b.h.toFixed(2)}</text>
              <text x={PAD_LEFT + idx * stepX} y={PAD_TOP + 42} textAnchor="middle" fill="#9ca3af" fontSize={9}>L {b.l.toFixed(2)}  C {b.c.toFixed(2)}</text>
            </g>
          )}
        </g>
      );
    });

    return (
      <div className="stock-service-chart-card stock-service-chart-card--full">
        <p className="stock-service-chart-legend">Each bar = one trading day (low → high). Hover a bar for O · H · L · C.</p>
        <svg
          className="stock-service-chart-svg"
          viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
          preserveAspectRatio="xMidYMid meet"
          role="img"
          aria-label="OHLC by day"
        >
          <rect x={0} y={0} width={CHART_WIDTH} height={CHART_HEIGHT} className="stock-service-chart-bg" />
          {gridLines}
          {yLabels}
          {xLabels}
          {barRects}
        </svg>
      </div>
    );
  };

  /** Today chart: merged Redis + session points, line + area, grid, axes. */
  const renderTodayChart = (symbol) => {
    const pts = getMergedTodayPoints(symbol);
    const cached = todayBySymbol[symbol] || [];
    const sessionOnly = cached.length === 0 && pts.length > 0;
    if (!pts.length) {
      return (
        <p className="stock-service-history-empty">
          No data for today yet. Live prices appear here after the next 60s refresh.
          Set <code>REDIS_URL</code> on the server to cache today&apos;s prices across restarts.
        </p>
      );
    }
    if (pts.length === 1) {
      return (
        <div className="stock-service-history-single">
          <span>Last: {pts[0].price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 4 })}</span>
        </div>
      );
    }
    const values = pts.map((p) => p.price);
    const min = Math.min(...values);
    const max = Math.max(...values);
    const first = pts[0].price;
    const last = pts[pts.length - 1].price;
    const abs = last - first;
    const pct = first > 0 ? (abs / first) * 100 : 0;
    const range = max - min || 1;
    const n = pts.length;
    const stepX = n > 1 ? PLOT_W / (n - 1) : PLOT_W;
    const toY = (price) => PAD_TOP + PLOT_H - ((price - min) / range) * PLOT_H;
    const pathLine = pts.map((p, idx) => `${idx === 0 ? 'M' : 'L'}${PAD_LEFT + idx * stepX},${toY(p.price)}`).join(' ');
    const pathArea = `${pathLine} L${PAD_LEFT + (n - 1) * stepX},${PAD_TOP + PLOT_H} L${PAD_LEFT},${PAD_TOP + PLOT_H} Z`;

    const gridLines = [];
    for (let i = 0; i <= 4; i++) {
      const y = PAD_TOP + (PLOT_H * i) / 4;
      gridLines.push(<line key={`h${i}`} x1={PAD_LEFT} y1={y} x2={PAD_LEFT + PLOT_W} y2={y} className="stock-service-chart-grid" />);
    }
    for (let i = 0; i <= 4; i++) {
      const x = PAD_LEFT + (PLOT_W * i) / 4;
      gridLines.push(<line key={`v${i}`} x1={x} y1={PAD_TOP} x2={x} y2={PAD_TOP + PLOT_H} className="stock-service-chart-grid" />);
    }
    const yLabels = [max, min + range * 0.75, min + range * 0.5, min + range * 0.25, min].map((v, i) => (
      <text key={i} x={PAD_LEFT - 6} y={PAD_TOP + (PLOT_H * i) / 4 + 4} className="stock-service-chart-axis" textAnchor="end">{v.toFixed(2)}</text>
    ));
    const timeLabels = [0, Math.floor(n / 4), Math.floor(n / 2), Math.floor((3 * n) / 4), n - 1].filter((i) => i >= 0 && i < n).map((i) => (
      <text key={i} x={PAD_LEFT + i * stepX} y={CHART_HEIGHT - 6} className="stock-service-chart-axis" textAnchor="middle">
        {new Date(pts[i].ts).toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit', hour12: false })}
      </text>
    ));

    const gradId = `todayGradient-${symbol}`;
    return (
      <div className="stock-service-chart-card stock-service-chart-card--full">
        <div className="stock-service-today-metrics">
          <span>Open-ish: {first.toFixed(2)}</span>
          <span>High: {max.toFixed(2)}</span>
          <span>Low: {min.toFixed(2)}</span>
          <span className={abs >= 0 ? 'up' : 'down'}>
            Session: {abs >= 0 ? '+' : ''}{abs.toFixed(2)} ({abs >= 0 ? '+' : ''}{pct.toFixed(2)}%)
          </span>
        </div>
        <p className="stock-service-chart-legend">
          {sessionOnly ? 'Intraday today (live only — set REDIS_URL on server to persist across restarts)' : 'Intraday today (server cache + live updates)'}
        </p>
        <svg
          className="stock-service-chart-svg"
          viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
          preserveAspectRatio="xMidYMid meet"
          role="img"
          aria-label="Today intraday"
        >
          <defs>
            <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#0d9488" stopOpacity="0.2" />
              <stop offset="100%" stopColor="#0d9488" stopOpacity="0" />
            </linearGradient>
          </defs>
          <rect x={0} y={0} width={CHART_WIDTH} height={CHART_HEIGHT} className="stock-service-chart-bg" />
          {gridLines}
          {yLabels}
          {timeLabels}
          <path d={pathArea} fill={`url(#${gradId})`} className="stock-service-chart-area" />
          <path d={pathLine} fill="none" stroke="#0d9488" strokeWidth="2" className="stock-service-chart-line" />
        </svg>
      </div>
    );
  };

  return (
    <PageTemplate
      title="Stock Service"
      subtitle="Watchlist – live prices (IBKR, every 60s)"
    >
      <div className="stock-service-content">
        <p className="stock-service-explainer">
          <Link to="/stock/gateway" className="stock-service-gateway-link">Gateway session</Link> (paste cookie when needed)
          {' · '}
          <Link to="/stock/simulator" className="stock-service-gateway-link">Replay simulator</Link>
        </p>
        <section className="stock-service-card stock-service-signals-card">
          <h2 className="stock-service-card-title">Daily Signals (next open → next close)</h2>
          <p className="stock-service-explainer">
            Decision at close, then execute at next session open and exit at close.
          </p>
          {!watchlistLoading && (
            <div className={`stock-service-readiness ${liveReadiness.ok ? 'ok' : 'warn'}`}>
              <div className="stock-service-readiness-title">
                Live readiness: {liveReadiness.ok ? 'OK' : 'CHECK'}
              </div>
              <div className="stock-service-readiness-items">
                <span className={liveReadiness.orderPlanReady ? 'ok' : 'bad'}>
                  prices {liveReadiness.wlFresh ? 'fresh' : 'stale'} / {liveReadiness.wlHasPrices ? 'available' : 'missing'}
                </span>
                <span className={liveReadiness.publishReady ? 'ok' : 'bad'}>
                  publish {liveReadiness.publishReady ? 'ready' : 'blocked'}
                </span>
                <span className={liveReadiness.paperOk ? 'ok' : 'bad'}>
                  paper loop {liveReadiness.paperOk ? 'ok' : 'n/a'}
                </span>
              </div>
              {!liveReadiness.orderPlanReady && (
                <div className="stock-service-readiness-note">
                  Order plan exports are disabled until prices are fresh and available.
                </div>
              )}
            </div>
          )}
          {dailySignalsLoading && <p className="stock-service-loading">Loading signals…</p>}
          {!dailySignalsLoading && dailySignalsError && (
            <p className="stock-service-error" role="alert">{dailySignalsError}</p>
          )}
          {!dailySignalsLoading && !dailySignalsError && dailySignals && (
            <div className="stock-service-signals-wrap">
              <div className="stock-service-watchlist-meta">
                <span>As of: {dailySignals.as_of_ts ? formatSvDateTime24h(new Date(Number(dailySignals.as_of_ts) * 1000).toISOString()) : '—'}</span>
                <span>Universe: {Array.isArray(dailySignals.universe) ? dailySignals.universe.length : Number(dailySignals.symbols_with_history || 0)}</span>
                <span>Overlap days: {Number(dailySignals.overlapping_days || 0)}</span>
                <span className="stock-service-signal-source">
                  Source: {dailySignalsSource === 'phase1-fallback' ? 'phase1 fallback' : 'model'}
                </span>
              </div>
              {dailySignalsSource === 'phase1-fallback' && (
                <p className="stock-service-signal-empty">
                  Model unavailable now: {formatSignalReason(dailySignals.fallback_reason)}. Using phase-1 fallback.
                </p>
              )}
              {!dailySignals.data_available && (
                <p className="stock-service-signal-empty">
                  {formatSignalReason(dailySignals.reason)}
                </p>
              )}
              {(modelName || topCoefficientEntries.length > 0 || dailySignals.training_rows != null) && (
                <div className="stock-service-model-diagnostics">
                  <div className="stock-service-status-row stock-service-model-publish-row">
                    <span className="stock-service-status-label">Publish status:</span>
                    <span className={`stock-service-badge ${publishSignals ? 'stock-service-badge-ok' : 'stock-service-badge-warn'}`}>
                      {publishSignals ? 'PUBLISH' : 'DO NOT PUBLISH'}
                    </span>
                    {!publishSignals && dailySignals.reason && (
                      <span>{formatSignalReason(dailySignals.reason)}</span>
                    )}
                  </div>
                  <div className="stock-service-model-diag-row">
                    <span>Feature set:</span>
                    <span>
                      rank features {featureAvailability.rankFeatures ? 'enabled' : 'n/a'}; vol regime {featureAvailability.volRegime ? 'enabled' : 'n/a'}
                    </span>
                  </div>
                  {modelHealth && (
                    <div className="stock-service-model-diag-row">
                      <span>Model health:</span>
                      <span>
                        rows={Number(modelHealth.feature_row_count || 0)}, non-finite={Number(modelHealth.nan_or_inf_count || 0)}, coef-norm={Number(modelHealth.coef_norm || 0).toFixed(4)}, train-window-days={Number(modelHealth.train_window_days || 0)}
                      </span>
                    </div>
                  )}
                  {topCoefficientEntries.length > 0 && (
                    <div className="stock-service-model-diag-row">
                      <span>Top coefficients:</span>
                      <span>
                        {topCoefficientEntries.map((c) => `${c.key}=${c.value.toFixed(4)}`).join(', ')}
                      </span>
                    </div>
                  )}
                </div>
              )}
              <div className="stock-service-paper-plan">
                <div className="stock-service-paper-plan-head">
                  <span className="stock-service-paper-plan-title">Dynamic Selection (hybrid)</span>
                  {!selectionLoading && selection && (
                    <span className="stock-service-paper-plan-sub">
                      universe={Number(selection.universe_size || 0)} selected={Number(selection.selected_size || 0)} price-priority&lt;=${Number(selection.price_priority_usd || 50).toFixed(0)} gate={selection.promotion_ready ? 'ready' : 'degraded'}
                    </span>
                  )}
                </div>
                {selectionLoading && <p className="stock-service-loading">Loading selection…</p>}
                {!selectionLoading && selectionError && (
                  <p className="stock-service-error" role="alert">{selectionError}</p>
                )}
                {!selectionLoading && selection && (
                  <>
                    <div className="stock-service-paper-plan-totals">
                      <span>wDaily: <b>{Number(selection?.weights?.daily || 0).toFixed(2)}</b></span>
                      <span>wIntraday: <b>{Number(selection?.weights?.intraday || 0).toFixed(2)}</b></span>
                      <span>wUnder50: <b>{Number(selection?.weights?.under50 || 0).toFixed(2)}</b></span>
                      <span>wRisk: <b>{Number(selection?.weights?.risk || 0).toFixed(2)}</b></span>
                    </div>
                    <div className="stock-service-model-diag-row" style={{ marginTop: '0.4rem' }}>
                      <span>Promotion gates:</span>
                      <span>
                        {selection.promotion_ready ? 'ready' : 'degraded'}; missing-price={Number(selection.missing_price_count || 0)}/{Number(selection.selected_size || 0)} ({(Number(selection.missing_price_ratio || 0) * 100).toFixed(1)}%)
                        {Array.isArray(selection.promotion_gate_failures) && selection.promotion_gate_failures.length > 0 ? `; fails=${selection.promotion_gate_failures.join(', ')}` : ''}
                      </span>
                    </div>
                    <div className="stock-service-table-wrap" style={{ marginTop: '0.35rem' }}>
                      <table className="stock-service-watchlist-table" aria-label="Dynamic hybrid selected symbols">
                        <thead>
                          <tr>
                            <th>Symbol</th>
                            <th className="stock-service-th-number">Price</th>
                            <th className="stock-service-th-number">Hybrid</th>
                            <th className="stock-service-th-number">Daily</th>
                            <th className="stock-service-th-number">Intraday</th>
                            <th className="stock-service-th-number">Under50</th>
                            <th className="stock-service-th-number">Risk</th>
                            <th>Why selected</th>
                          </tr>
                        </thead>
                        <tbody>
                          {(selection.selected || []).slice(0, 12).map((c) => (
                            <tr key={c.symbol}>
                              <td className="stock-service-wl-symbol">{c.symbol}</td>
                              <td className="stock-service-td-number">{c.price != null ? Number(c.price).toFixed(2) : '—'}</td>
                              <td className="stock-service-td-number">{Number(c.hybrid_score || 0).toFixed(3)}</td>
                              <td className="stock-service-td-number">{Number(c.daily_score || 0).toFixed(3)}</td>
                              <td className="stock-service-td-number">{Number(c.intraday_score || 0).toFixed(3)}</td>
                              <td className="stock-service-td-number">{Number(c.under50_bonus || 0).toFixed(2)}</td>
                              <td className="stock-service-td-number">{Number(c.risk_penalty || 0).toFixed(2)}</td>
                              <td className="stock-service-selection-reasons">{(c.reasons || []).join(', ')}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </>
                )}
              </div>
              <div className="stock-service-paper-card">
                <div className="stock-service-paper-header">
                  <h3 className="stock-service-paper-title">Paper trade</h3>
                  <div className="stock-service-paper-mode">
                    <span className="stock-service-paper-mode-label">PnL:</span>
                    <button
                      type="button"
                      className={`stock-service-paper-mode-btn ${paperTradePnlMode === 'net' ? 'active' : ''}`}
                      onClick={() => setPaperTradePnlMode('net')}
                    >
                      Net
                    </button>
                    <button
                      type="button"
                      className={`stock-service-paper-mode-btn ${paperTradePnlMode === 'gross' ? 'active' : ''}`}
                      onClick={() => setPaperTradePnlMode('gross')}
                    >
                      Gross
                    </button>
                  </div>
                </div>

                <div className="stock-service-paper-grid">
                  <div className="stock-service-paper-kpi">
                    <div className="stock-service-paper-kpi-label">Pending</div>
                    <div className="stock-service-paper-kpi-value">{paperTradeLoading ? '—' : Number(paperTradeStatus?.pending_after || 0)}</div>
                  </div>
                  <div className="stock-service-paper-kpi">
                    <div className="stock-service-paper-kpi-label">Executions</div>
                    <div className="stock-service-paper-kpi-value">{paperTradeLoading ? '—' : Number(paperTradeStatus?.execution_count || 0)}</div>
                  </div>
                  <div className="stock-service-paper-kpi">
                    <div className="stock-service-paper-kpi-label">Cumulative ({paperTradePnlMode})</div>
                    <div className="stock-service-paper-kpi-value">
                      {paperTradeLoading ? '—' : `${(Number(
                        paperTradePnlMode === 'gross'
                          ? (paperTradeStatus?.cumulative_pnl_return_gross ?? paperTradeStatus?.cumulative_pnl_return ?? 0)
                          : (paperTradeStatus?.cumulative_pnl_return_net ?? paperTradeStatus?.cumulative_pnl_return ?? 0)
                      ) * 100).toFixed(2)}%`}
                    </div>
                  </div>
                  <div className="stock-service-paper-kpi">
                    <div className="stock-service-paper-kpi-label">Last day ({paperTradePnlMode})</div>
                    <div className="stock-service-paper-kpi-value">
                      {paperTradeLoading
                        ? '—'
                        : (paperTradeStatus?.last_execution
                          ? `${(Number(
                            paperTradePnlMode === 'gross'
                              ? (paperTradeStatus.last_execution.pnl_return_gross ?? paperTradeStatus.last_execution.pnl_return ?? 0)
                              : (paperTradeStatus.last_execution.pnl_return_net ?? paperTradeStatus.last_execution.pnl_return ?? 0)
                          ) * 100).toFixed(2)}%`
                          : '—')}
                    </div>
                  </div>
                </div>

                {paperTradeStatus?.cost_assumptions && (
                  <div className="stock-service-paper-costs">
                    Costs (bps): slippage/side <b>{Number(paperTradeStatus.cost_assumptions.slippage_bps_per_side || 0).toFixed(1)}</b>, commission/side <b>{Number(paperTradeStatus.cost_assumptions.commission_bps_per_side || 0).toFixed(1)}</b>, borrow/day <b>{Number(paperTradeStatus.cost_assumptions.short_borrow_bps_daily || 0).toFixed(1)}</b>
                  </div>
                )}

                {paperTradeRecords && paperTradeRecords.length > 0 && (
                  <div className="stock-service-paper-equity">
                    <div className="stock-service-paper-equity-top">
                      <div className="stock-service-paper-equity-stats">
                        <span>Win-rate: <b>{Number(paperTradeStats.winRate * 100).toFixed(1)}%</b></span>
                        <span>Avg: <b>{Number(paperTradeStats.avgReturn * 100).toFixed(2)}%</b></span>
                        <span>Best: <b>{Number(paperTradeStats.bestReturn * 100).toFixed(2)}%</b></span>
                        <span>Worst: <b>{Number(paperTradeStats.worstReturn * 100).toFixed(2)}%</b></span>
                      </div>
                      <button
                        type="button"
                        className="stock-service-paper-action"
                        disabled={paperTradeRunLoading}
                        onClick={async () => {
                          setPaperTradeRunLoading(true);
                          setPaperTradeRunMsg('');
                          try {
                            const r = await fetch(PAPER_TRADE_RUN_URL, {
                              method: 'POST',
                              credentials: 'include',
                              headers: { Accept: 'application/json' },
                            });
                            const j = await r.json().catch(() => null);
                            if (!r.ok) throw new Error(j?.error || `Run failed (${r.status})`);
                            setPaperTradeRunMsg(
                              `Run done: executed ${Number(j?.executed_count || 0)}, pending ${Number(j?.pending_after || 0)}, stored ${j?.stored_decision ? 'yes' : 'no'}`
                            );
                            loadDailySignals();
                            loadPaperTradeStatus();
                            loadPaperTradeRecords();
                            loadPaperTradeOrderPlan();
                          } catch (e) {
                            setPaperTradeRunMsg(`Run failed: ${String(e?.message || e)}`);
                          } finally {
                            setPaperTradeRunLoading(false);
                          }
                        }}
                      >
                        {paperTradeRunLoading ? 'Running…' : 'Run paper loop now'}
                      </button>
                      <button
                        type="button"
                        className="stock-service-paper-action"
                        disabled={paperTradeBackfillLoading}
                        onClick={async () => {
                          setPaperTradeBackfillLoading(true);
                          setPaperTradeBackfillMsg('');
                          try {
                            const r = await fetch(PAPER_TRADE_BACKFILL_URL, {
                              method: 'POST',
                              credentials: 'include',
                              headers: { Accept: 'application/json' },
                            });
                            if (!r.ok) throw new Error(`Backfill failed (${r.status})`);
                            const j = await r.json();
                            setPaperTradeBackfillMsg(`Backfill: inserted ${Number(j.inserted_count || 0)}, skipped ${Number(j.skipped_existing_count || 0)}`);
                            loadPaperTradeStatus();
                            loadPaperTradeRecords();
                          } catch {
                            setPaperTradeBackfillMsg('Backfill failed. Try again later.');
                          } finally {
                            setPaperTradeBackfillLoading(false);
                          }
                        }}
                      >
                        {paperTradeBackfillLoading ? 'Backfilling…' : 'Backfill history'}
                      </button>
                    </div>
                    {paperTradeRunMsg && (
                      <div className="stock-service-paper-note">{paperTradeRunMsg}</div>
                    )}
                    {paperTradeBackfillMsg && (
                      <div className="stock-service-paper-note">{paperTradeBackfillMsg}</div>
                    )}
                    {paperTradeEquity.length > 0 && (
                      <div className="stock-service-paper-trade-equity">
                        <p className="stock-service-chart-legend" style={{ marginBottom: '0.35rem' }}>
                          Equity curve ({paperTradePnlMode})
                        </p>
                        <svg
                          className="stock-service-paper-trade-equity-svg"
                          viewBox="0 0 560 130"
                          preserveAspectRatio="none"
                          role="img"
                          aria-label="Paper trade equity curve"
                        >
                          {(() => {
                            const W = 560;
                            const H = 130;
                            const L = 42;
                            const R = 8;
                            const T = 10;
                            const B = 24;
                            const PW = W - L - R;
                            const PH = H - T - B;
                            const vals = paperTradeEquity.map((p) => p.y);
                            const min = Math.min(...vals, 0);
                            const max = Math.max(...vals, 0);
                            const range = Math.max(1e-9, max - min);
                            const toX = (i) => L + (paperTradeEquity.length > 1 ? (i / (paperTradeEquity.length - 1)) * PW : 0);
                            const toY = (v) => T + PH - ((v - min) / range) * PH;
                            const line = paperTradeEquity
                              .map((p, i) => `${i === 0 ? 'M' : 'L'}${toX(i)},${toY(p.y)}`)
                              .join(' ');
                            const area = `${line} L${toX(paperTradeEquity.length - 1)},${T + PH} L${toX(0)},${T + PH} Z`;
                            return (
                              <>
                                <rect x="0" y="0" width={W} height={H} className="stock-service-chart-bg" />
                                <line x1={L} y1={toY(0)} x2={L + PW} y2={toY(0)} className="stock-service-chart-grid" />
                                <path d={area} className="stock-service-paper-trade-equity-area" />
                                <path d={line} className="stock-service-paper-trade-equity-line" />
                                <text x={L - 6} y={toY(max) + 4} className="stock-service-chart-axis" textAnchor="end">{(max * 100).toFixed(1)}%</text>
                                <text x={L - 6} y={toY(min) + 4} className="stock-service-chart-axis" textAnchor="end">{(min * 100).toFixed(1)}%</text>
                              </>
                            );
                          })()}
                        </svg>
                      </div>
                    )}
                  </div>
                )}

                <details className="stock-service-paper-details">
                  <summary className="stock-service-paper-summary">
                    Recent executions {paperTradeRecordsLoading ? '(loading…) ' : ''}{paperTradeRecords?.length ? `(${paperTradeRecords.length})` : ''}
                  </summary>
                  {paperTradeRecords && paperTradeRecords.length > 0 ? (
                    <div className="stock-service-paper-trade-records">
                      {paperTradeRecords.slice(0, 3).map((exec) => (
                        <div key={exec.executed_at_ts} className="stock-service-paper-trade-exec">
                          <div className="stock-service-paper-trade-exec-meta">
                            <span>Decision: <b>{exec.decision_as_of_ts ? formatSvDateTime24h(new Date(Number(exec.decision_as_of_ts) * 1000).toISOString()) : '—'}</b></span>
                            <span>Exec: <b>{exec.execution_as_of_ts ? formatSvDateTime24h(new Date(Number(exec.execution_as_of_ts) * 1000).toISOString()) : '—'}</b></span>
                            <span>Gross: <b>{(Number(exec.pnl_return_gross ?? exec.pnl_return ?? 0) * 100).toFixed(2)}%</b></span>
                            <span>Net: <b>{(Number(exec.pnl_return_net ?? exec.pnl_return ?? 0) * 100).toFixed(2)}%</b></span>
                            <span>Cost ln: <b>{Number(exec.cost_total_ln ?? 0).toFixed(4)}</b></span>
                          </div>
                          <div className="stock-service-table-wrap" style={{ marginTop: '0.35rem' }}>
                            <table className="stock-service-watchlist-table" aria-label="Paper trade execution legs">
                              <thead>
                                <tr>
                                  <th>Symbol</th>
                                  <th>Side</th>
                                  <th className="stock-service-th-number">Weight</th>
                                  <th className="stock-service-th-number">y_oc_next</th>
                                  <th className="stock-service-th-number">Gross</th>
                                  <th className="stock-service-th-number">Cost</th>
                                  <th className="stock-service-th-number">Net</th>
                                </tr>
                              </thead>
                              <tbody>
                                {(exec.legs || []).map((leg) => (
                                  <tr key={`${leg.symbol}-${leg.side}-${leg.y_oc_next}`}>
                                    <td className="stock-service-wl-symbol">{leg.symbol}</td>
                                    <td>
                                      <span className={leg.side === 'LONG' ? 'stock-service-signal-long' : 'stock-service-signal-short'}>
                                        {leg.side}
                                      </span>
                                    </td>
                                    <td className="stock-service-td-number">{Number(leg.weight || 0).toFixed(3)}</td>
                                    <td className="stock-service-td-number">{Number(leg.y_oc_next || 0).toFixed(5)}</td>
                                    <td className="stock-service-td-number">{Number(leg.contrib_gross ?? 0).toFixed(5)}</td>
                                    <td className="stock-service-td-number">{Number(leg.cost_ln ?? 0).toFixed(5)}</td>
                                    <td className="stock-service-td-number">{Number(leg.contrib_net ?? leg.contrib ?? 0).toFixed(5)}</td>
                                  </tr>
                                ))}
                              </tbody>
                            </table>
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="stock-service-explainer" style={{ margin: '0.5rem 0 0 0' }}>
                      {paperTradeRecordsLoading ? 'Loading executions…' : 'No executions yet.'}
                    </p>
                  )}
                </details>
              </div>
              {dailySignals.data_available && (
                <>
                  <div className="stock-service-signals-kpis">
                    <span>Long/Short quantile: {Number(dailySignals.quantile || 0).toFixed(2)}</span>
                    <span>Short enabled: {dailySignals.short_enabled ? 'yes' : 'no'}</span>
                    {modelName && <span>Model: {modelName}</span>}
                    {dailySignals.training_rows != null && (
                      <span>Training rows: {Number(dailySignals.training_rows || 0)}</span>
                    )}
                    {dailySignals.backtest && (
                      <span>Backtest Sharpe(252): {Number(dailySignals.backtest.sharpe_252 || 0).toFixed(2)}</span>
                    )}
                  </div>
                  {orderPlanView?.rows?.length > 0 && (
                    <div className="stock-service-paper-plan">
                      <div className="stock-service-paper-plan-head">
                                <span className="stock-service-paper-plan-title">
                                  Order plan {Array.isArray(paperTradeOrderPlan?.rows) && paperTradeOrderPlan.rows.length > 0 ? <span style={{ opacity: 0.75 }}>(server)</span> : <span style={{ opacity: 0.75 }}>(estimate)</span>}
                                </span>
                        <span className="stock-service-paper-plan-sub">
                          capital=${Number(orderPlanView.cap || 0).toFixed(0)}, gross long={Number(orderPlanView.grossLong || 0).toFixed(2)}, gross short={Number(orderPlanView.grossShort || 0).toFixed(2)}, rounding={orderPlanView.roundMode}{orderPlanView.roundMode === 'lot' ? `(${Number(orderPlanView.lotSize || 1)})` : ''}, min notional=${Number(orderPlanView.minNotional || 0).toFixed(0)}
                        </span>
                      </div>
                      <div className="stock-service-paper-plan-actions">
                        <div className="stock-service-paper-plan-totals">
                          <span>Long used: <b>${Number(orderPlanExport?.totals?.longUsed || 0).toFixed(0)}</b></span>
                          <span>Short used: <b>${Number(orderPlanExport?.totals?.shortUsed || 0).toFixed(0)}</b></span>
                          <span>Net: <b>${Number(orderPlanExport?.totals?.net || 0).toFixed(0)}</b></span>
                          <span>Gross: <b>${Number(orderPlanExport?.totals?.gross || 0).toFixed(0)}</b></span>
                        </div>
                        <div className="stock-service-paper-plan-buttons">
                          <button
                            type="button"
                            className="stock-service-btn secondary"
                            disabled={!liveReadiness.orderPlanReady}
                            onClick={async () => {
                              try {
                                await navigator.clipboard.writeText(orderPlanExport.csv);
                              } catch {
                                // silent
                              }
                            }}
                          >
                            Copy CSV
                          </button>
                          <button
                            type="button"
                            className="stock-service-btn secondary"
                            disabled={!liveReadiness.orderPlanReady}
                            onClick={() => {
                              const blob = new Blob([orderPlanExport.csv], { type: 'text/csv;charset=utf-8' });
                              const url = URL.createObjectURL(blob);
                              const a = document.createElement('a');
                              a.href = url;
                              a.download = 'order-plan.csv';
                              a.click();
                              URL.revokeObjectURL(url);
                            }}
                          >
                            Download CSV
                          </button>
                          <button
                            type="button"
                            className="stock-service-btn secondary"
                            disabled={!liveReadiness.orderPlanReady}
                            onClick={() => {
                              const blob = new Blob([JSON.stringify(orderPlanExport.payload, null, 2)], { type: 'application/json;charset=utf-8' });
                              const url = URL.createObjectURL(blob);
                              const a = document.createElement('a');
                              a.href = url;
                              a.download = 'order-plan.json';
                              a.click();
                              URL.revokeObjectURL(url);
                            }}
                          >
                            Download JSON
                          </button>
                        </div>
                      </div>
                      <div className="stock-service-table-wrap">
                        <table className="stock-service-watchlist-table" aria-label="Order plan">
                          <thead>
                            <tr>
                              <th>Symbol</th>
                              <th>Side</th>
                              <th className="stock-service-th-number">Weight</th>
                              <th className="stock-service-th-number">Est price</th>
                              <th className="stock-service-th-number">Target $</th>
                              <th className="stock-service-th-number">Used $</th>
                              <th className="stock-service-th-number">Est shares</th>
                              <th>Status</th>
                            </tr>
                          </thead>
                          <tbody>
                            {orderPlanView.rows.map((r) => (
                              <tr key={`${r.symbol}-${r.side}`}>
                                <td className="stock-service-wl-symbol">{r.symbol}</td>
                                <td>
                                  <span className={r.side === 'LONG' ? 'stock-service-signal-long' : 'stock-service-signal-short'}>{r.side}</span>
                                </td>
                                <td className="stock-service-td-number">{Number(r.weight || 0).toFixed(3)}</td>
                                <td className="stock-service-td-number">{r.price_est != null ? Number(r.price_est).toFixed(2) : '—'}</td>
                                <td className="stock-service-td-number">${Number(r.notional_target || 0).toFixed(0)}</td>
                                <td className="stock-service-td-number">{r.notional_used != null ? `$${Number(r.notional_used || 0).toFixed(0)}` : '—'}</td>
                                <td className="stock-service-td-number">{r.shares != null ? Number(r.shares).toFixed(2) : '—'}</td>
                                <td>{r.skipped ? 'skip (too small)' : 'ok'}</td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                      <p className="stock-service-explainer" style={{ marginTop: '0.35rem' }}>
                        {orderPlanView.source === 'server'
                          ? `Server-computed using latest watchlist snapshot (${orderPlanView.watchlistAsOf || '—'}).`
                          : 'Uses current watchlist price as estimate; actual fills are computed from next-session open/close.'}
                      </p>
                    </div>
                  )}
                  <div className="stock-service-table-wrap">
                    <table className="stock-service-watchlist-table" aria-label="Daily strategy signals">
                      <thead>
                        <tr>
                          <th>Symbol</th>
                          <th>Side</th>
                          <th className="stock-service-th-number">Weight</th>
                          <th className="stock-service-th-number">Score</th>
                        </tr>
                      </thead>
                      <tbody>
                        {(dailySignals.signals || []).map((s) => (
                          <tr key={`${s.symbol}-${s.side}`}>
                            <td className="stock-service-wl-symbol">{s.symbol}</td>
                            <td>
                              <span className={s.side === 'LONG' ? 'stock-service-signal-long' : 'stock-service-signal-short'}>
                                {s.side}
                              </span>
                            </td>
                            <td className="stock-service-td-number">{Number(s.weight || 0).toFixed(3)}</td>
                            <td className="stock-service-td-number">{Number(s.score || 0).toFixed(3)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              )}
              <div className="stock-service-signals-json-controls">
                <button
                  type="button"
                  className="stock-service-btn secondary"
                  onClick={() => setShowSignalsJson((prev) => !prev)}
                >
                  {showSignalsJson ? 'Hide raw JSON' : 'View raw JSON'}
                </button>
                {dailySignals && (
                  <>
                    <button
                      type="button"
                      className="stock-service-btn secondary"
                      onClick={async () => {
                        try {
                          await navigator.clipboard.writeText(dailySignalsJson);
                        } catch {
                          // Clipboard can fail in non-secure contexts; keep UI silent.
                        }
                      }}
                    >
                      Copy JSON
                    </button>
                    <button
                      type="button"
                      className="stock-service-btn secondary"
                      onClick={() => {
                        const blob = new Blob([dailySignalsJson], { type: 'application/json;charset=utf-8' });
                        const url = URL.createObjectURL(blob);
                        const a = document.createElement('a');
                        a.href = url;
                        a.download = 'daily-signals.json';
                        a.click();
                        URL.revokeObjectURL(url);
                      }}
                    >
                      Download JSON
                    </button>
                  </>
                )}
              </div>
              {showSignalsJson && dailySignals && (
                <pre className="stock-service-signals-json" aria-label="Daily signals raw JSON">
                  {dailySignalsJson}
                </pre>
              )}
            </div>
          )}
        </section>
        <section className="stock-service-card stock-service-watchlist-card">
          <h2 className="stock-service-card-title">Prices</h2>
          <p className="stock-service-explainer">
            Chosen symbols. Data from IBKR. Updates every 60 seconds.
          </p>
          {watchlistLoading && <p className="stock-service-loading">Loading…</p>}
          {watchlistError && !watchlistLoading && (
            <p className="stock-service-error" role="alert">
              {watchlistError}
            </p>
          )}
          {!watchlistLoading && watchlist && watchlist.symbols && watchlist.symbols.length > 0 && (
            <>
              <div className="stock-service-watchlist-meta">
                <span>As of: {formatSvDateTime24h(watchlist.as_of)}</span>
                {watchlist.provider && (
                  <span className="stock-service-watchlist-provider">{watchlist.provider}</span>
                )}
              </div>
              <div className="stock-service-table-wrap">
                <table className="stock-service-watchlist-table" aria-label="Watchlist prices">
                  <thead>
                    <tr>
                      <th>Symbol</th>
                      <th>Name</th>
                      <th className="stock-service-th-number">Price</th>
                      <th className="stock-service-th-center">Change</th>
                      <th>Currency</th>
                      <th>Updated</th>
                      <th>Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {watchlist.symbols.map((row) => {
                      const change = getChangeIndicator(row.symbol, row.price);
                      const isExpanded = expandedSymbol === row.symbol;
                      const symbolRange = getRangeForSymbol(row.symbol);
                      const historyEntry = historyBySymbol[`${row.symbol}:${symbolRange}`];
                      const updatedSource = row.updated_at || watchlist.as_of || null;
                      const updatedDt = updatedSource ? new Date(updatedSource) : null;
                      const updatedAgeMs = updatedDt && !Number.isNaN(updatedDt.getTime()) ? (Date.now() - updatedDt.getTime()) : null;
                      const isStale = updatedAgeMs != null && updatedAgeMs > (2 * 60 * 60 * 1000);
                      return (
                        <React.Fragment key={row.symbol}>
                          <tr
                            className="stock-service-row-clickable"
                            onClick={() =>
                              setExpandedSymbol((prev) => {
                                if (prev === row.symbol) return null;
                                setHistoryRangeBySymbol((ranges) => ({
                                  ...ranges,
                                  [row.symbol]: ranges[row.symbol] || '7d',
                                }));
                                return row.symbol;
                              })
                            }
                          >
                            <td className="stock-service-wl-symbol">
                              {row.symbol}
                            </td>
                            <td className="stock-service-wl-name">{SYMBOL_SHORT_NAMES[row.symbol] || row.symbol}</td>
                            <td className="stock-service-wl-price stock-service-td-number">
                              {row.price != null
                                ? Number(row.price).toLocaleString(undefined, {
                                    minimumFractionDigits: 2,
                                    maximumFractionDigits: 4,
                                  })
                                : '—'}
                            </td>
                            <td className="stock-service-wl-change stock-service-td-center">
                              {change === 'up' && <span className="stock-service-change-up" title="Up since last run">↑</span>}
                              {change === 'down' && <span className="stock-service-change-down" title="Down since last run">↓</span>}
                              {change === 'unchanged' && <span className="stock-service-change-unchanged" title="Unchanged">—</span>}
                              {change === null && <span>—</span>}
                            </td>
                            <td>{row.currency || '—'}</td>
                            <td className="stock-service-wl-updated">
                              {formatSvDateTime24h(updatedSource)}
                              {isStale ? ' (stale)' : ''}
                            </td>
                            <td>
                              {row.error ? (
                                row.error.includes('pre-flight or stream not ready') ? (
                                  <span className="stock-service-wl-pending" title={row.error}>
                                    Pending
                                  </span>
                                ) : (
                                  <span className="stock-service-wl-error" title={row.error}>
                                    {row.error}
                                  </span>
                                )
                              ) : (
                                <span className="stock-service-wl-ok">{isExpanded ? 'Hide' : 'Details'}</span>
                              )}
                            </td>
                          </tr>
                          {isExpanded && (
                            <tr className="stock-service-history-row">
                              <td colSpan={7}>
                                <div className="stock-service-history-header">
                                  <span className="stock-service-history-title">
                                    Historical data – {row.symbol}
                                  </span>
                                  <div
                                    className="stock-service-history-range-toggle"
                                    onClick={(e) => e.stopPropagation()}
                                  >
                                    {[
                                      { key: '7d', label: '7d' },
                                      { key: '1m', label: '1m' },
                                      { key: '3m', label: '3m' },
                                      { key: '1y', label: '1y' },
                                    ].map((opt) => (
                                      <button
                                        key={opt.key}
                                        type="button"
                                        className={
                                          symbolRange === opt.key
                                            ? 'stock-service-history-range-btn active'
                                            : 'stock-service-history-range-btn'
                                        }
                                        aria-pressed={symbolRange === opt.key}
                                        onClick={() =>
                                          setHistoryRangeBySymbol((ranges) => ({
                                            ...ranges,
                                            [row.symbol]: opt.key,
                                          }))
                                        }
                                      >
                                        {opt.label}
                                      </button>
                                    ))}
                                  </div>
                                </div>
                                <div className="stock-service-chart-section stock-service-chart-section--today">
                                  <h4 className="stock-service-chart-section-title">Intraday — today</h4>
                                  <div className="stock-service-history-chart-wrap stock-service-history-chart-wrap--full">
                                    {renderTodayChart(row.symbol)}
                                  </div>
                                </div>
                                {historyLoading === row.symbol && (
                                  <p className="stock-service-history-empty">Loading from IBKR…</p>
                                )}
                                {!historyLoading && historyError && historyEntry === null && (
                                  <p className="stock-service-history-empty stock-service-wl-error">{historyError}</p>
                                )}
                                {!historyLoading && historyEntry?.data?.length > 0 && (
                                  <div className="stock-service-history-layout">
                                    <div className="stock-service-chart-section stock-service-chart-section--history">
                                      <h4 className="stock-service-chart-section-title">Daily bars — selected range</h4>
                                      {(historyEntry.period || historyEntry.barCount != null) && (
                                        <p className="stock-service-history-meta">
                                          IBKR period: <code>{historyEntry.period ?? '—'}</code>
                                          {historyEntry.barCount != null
                                            ? ` · ${historyEntry.barCount} daily bars`
                                            : ''}
                                        </p>
                                      )}
                                      {(() => {
                                        const bars = historyEntry.data;
                                        const first = bars[0];
                                        const last = bars[bars.length - 1];
                                        const start = Number(first.close ?? first.c ?? 0);
                                        const end = Number(last.close ?? last.c ?? 0);
                                        const abs = end - start;
                                        const pct = start ? (abs / start) * 100 : 0;
                                        const high = Math.max(...bars.map((b) => Number(b.high ?? b.h ?? b.o ?? b.close ?? b.c ?? 0)));
                                        const low = Math.min(...bars.map((b) => Number(b.low ?? b.l ?? b.o ?? b.close ?? b.c ?? 0)));
                                        const startDate = new Date(barTimeToMs(first.t));
                                        const endDate = new Date(barTimeToMs(last.t));
                                        const fmtDate = (d) =>
                                          Number.isNaN(d.getTime())
                                            ? '—'
                                            : new Intl.DateTimeFormat('sv-SE', {
                                                month: '2-digit',
                                                day: '2-digit',
                                                year: 'numeric',
                                              }).format(d);
                                        const up = abs >= 0;
                                        const rangeLabels = { '7d': '7d', '1m': '1m', '3m': '3m', '1y': '1y' };
                                        const changeLabel = `${rangeLabels[symbolRange] || symbolRange} change (close)`;
                                        const barsCoverageMsg = historyEntry.expectedBarsMin && historyEntry.barCount < historyEntry.expectedBarsMin
                                          ? `Coverage is limited (${historyEntry.barCount} bars). This usually means recent IPO/holiday/IBKR data availability for this symbol.`
                                          : '';
                                        return (
                                          <>
                                            <div className="stock-service-history-metrics-row" role="group" aria-label="Period summary">
                                              <div className="stock-service-history-metric">
                                                <span className="label">{changeLabel}</span>
                                                <span className={up ? 'value up' : 'value down'}>
                                                  {abs >= 0 ? '+' : ''}
                                                  {abs.toFixed(2)} ({abs >= 0 ? '+' : ''}
                                                  {pct.toFixed(1)}%)
                                                </span>
                                              </div>
                                              <div className="stock-service-history-metric">
                                                <span className="label">Range high / low</span>
                                                <span className="value">
                                                  {high.toFixed(2)} / {low.toFixed(2)}
                                                </span>
                                              </div>
                                              <div className="stock-service-history-metric">
                                                <span className="label">First — last day</span>
                                                <span className="value">
                                                  {fmtDate(startDate)} — {fmtDate(endDate)}
                                                </span>
                                              </div>
                                            </div>
                                            <div className="stock-service-history-chart-wrap stock-service-history-chart-wrap--full">
                                              {renderHistoryChart(bars, up, row.symbol)}
                                            </div>
                                            {barsCoverageMsg && (
                                              <p className="stock-service-history-coverage-note">{barsCoverageMsg}</p>
                                            )}
                                          </>
                                        );
                                      })()}
                                    </div>
                                  </div>
                                )}
                                {!historyLoading && (!historyBySymbol[row.symbol]?.data?.length) && historyBySymbol[row.symbol] !== null && (
                                  <p className="stock-service-history-empty">No bars returned.</p>
                                )}
                                {!historyLoading && historyBySymbol[row.symbol] === undefined && !historyError && renderTodayChart(row.symbol)}
                              </td>
                            </tr>
                          )}
                        </React.Fragment>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </>
          )}
          {!watchlistLoading && (!watchlist || !watchlist.symbols?.length) && !watchlistError && (
            <p className="stock-service-explainer">No watchlist data available.</p>
          )}
        </section>
      </div>
    </PageTemplate>
  );
}
