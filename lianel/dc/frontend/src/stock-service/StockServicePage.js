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
const WATCHLIST_REFRESH_MS = 60_000;
const DAILY_SIGNALS_REFRESH_MS = 5 * 60_000;

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
  const [historyBySymbol, setHistoryBySymbol] = useState({});
  const [historyLoading, setHistoryLoading] = useState(null);
  const [historyError, setHistoryError] = useState(null);
  const [historyRange, setHistoryRange] = useState('7d'); // '7d' | '1m' | '3m' | '1y'
  const [todayBySymbol, setTodayBySymbol] = useState({});
  const [chartHover, setChartHover] = useState({ symbol: null, barIndex: null });
  const [dailySignals, setDailySignals] = useState(null);
  const [dailySignalsLoading, setDailySignalsLoading] = useState(true);
  const [dailySignalsError, setDailySignalsError] = useState(null);
  const [dailySignalsSource, setDailySignalsSource] = useState('model');

  const loadWatchlist = useCallback(() => {
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
      .finally(() => setWatchlistLoading(false));
  }, []);

  useEffect(() => {
    loadWatchlist();
    const id = setInterval(loadWatchlist, WATCHLIST_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadWatchlist]);

  const loadDailySignals = useCallback(() => {
    setDailySignalsLoading(true);
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
        });
        setDailySignalsSource('phase1-fallback');
      })
      .catch(() => {
        setDailySignalsError('Could not load daily signals.');
        setDailySignals(null);
        setDailySignalsSource('model');
      })
      .finally(() => setDailySignalsLoading(false));
  }, []);

  useEffect(() => {
    loadDailySignals();
    const id = setInterval(loadDailySignals, DAILY_SIGNALS_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadDailySignals]);

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

  // When a row is expanded, fetch IBKR history for that symbol (once per symbol).
  useEffect(() => {
    if (!expandedSymbol) return;
    const existing = historyBySymbol[expandedSymbol];
    if (existing && existing.range === historyRange) return; // already loaded for this range
    const sym = expandedSymbol;
    const range = historyRange;
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
    const url = `${HISTORY_URL}?symbol=${encodeURIComponent(sym)}&days=${encodeURIComponent(days)}`;
    fetch(url, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => r.json())
      .then((res) => {
        if (res && res.error) {
          setHistoryBySymbol((prev) => ({ ...prev, [sym]: null }));
          setHistoryError(res.error);
        } else {
          const data = Array.isArray(res?.data) ? res.data : [];
          setHistoryBySymbol((prev) => ({
            ...prev,
            [sym]: {
              range,
              data,
              period: res.period,
              barCount: typeof res.bars === 'number' ? res.bars : data.length,
            },
          }));
          setHistoryError(null);
        }
      })
      .catch((err) => {
        setHistoryBySymbol((prev) => ({ ...prev, [sym]: null }));
        setHistoryError(err.message || 'Failed to load history');
      })
      .finally(() => setHistoryLoading((prev) => (prev === sym ? null : prev)));
  }, [expandedSymbol, historyRange, historyBySymbol]);

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
        </p>
        <section className="stock-service-card stock-service-signals-card">
          <h2 className="stock-service-card-title">Daily Signals (next open → next close)</h2>
          <p className="stock-service-explainer">
            Decision at close, then execute at next session open and exit at close.
          </p>
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
                  <div className="stock-service-model-diag-row">
                    <span>Feature set:</span>
                    <span>
                      rank features {featureAvailability.rankFeatures ? 'enabled' : 'n/a'}; vol regime {featureAvailability.volRegime ? 'enabled' : 'n/a'}
                    </span>
                  </div>
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
                      const historyEntry = historyBySymbol[row.symbol];
                      return (
                        <React.Fragment key={row.symbol}>
                          <tr
                            className="stock-service-row-clickable"
                            onClick={() =>
                              setExpandedSymbol((prev) => (prev === row.symbol ? null : row.symbol))
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
                            <td className="stock-service-wl-updated">{formatSvDateTime24h(row.updated_at)}</td>
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
                                          historyRange === opt.key
                                            ? 'stock-service-history-range-btn active'
                                            : 'stock-service-history-range-btn'
                                        }
                                        aria-pressed={historyRange === opt.key}
                                        onClick={() => setHistoryRange(opt.key)}
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
                                        const changeLabel = `${rangeLabels[historyRange] || historyRange} change (close)`;
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
