/**
 * Stock service – watchlist: price table for chosen symbols (IBKR), refreshes every 60s.
 */
import React, { useEffect, useState, useCallback, useRef } from 'react';
import { Link } from 'react-router-dom';
import PageTemplate from '../PageTemplate';
import './StockServicePage.css';

const WATCHLIST_URL = '/api/v1/stock-service/watchlist';
const HISTORY_URL = '/api/v1/stock-service/history';
const WATCHLIST_REFRESH_MS = 60_000;

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
          setHistoryBySymbol((prev) => ({ ...prev, [sym]: { range, data } }));
          setHistoryError(null);
        }
      })
      .catch((err) => {
        setHistoryBySymbol((prev) => ({ ...prev, [sym]: null }));
        setHistoryError(err.message || 'Failed to load history');
      })
      .finally(() => setHistoryLoading((prev) => (prev === sym ? null : prev)));
  }, [expandedSymbol, historyRange, historyBySymbol]);

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

  /** Render chart from IBKR history bars (array of { t, open, high, low, close }). */
  const renderHistoryChart = (bars, isUp) => {
    if (!bars?.length) return <p className="stock-service-history-empty">No historical data.</p>;
    const pts = bars.map((b) => ({ ts: Number(b.t) * 1000, price: Number(b.close) }));
    if (pts.length === 1) {
      return (
        <div className="stock-service-history-single">
          <span>Close: {pts[0].price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 4 })}</span>
        </div>
      );
    }
    const values = pts.map((p) => p.price);
    const min = Math.min(...values);
    const max = Math.max(...values);
    const range = max - min || 1;
    const width = 320;
    const height = 120;
    const stepX = pts.length > 1 ? width / (pts.length - 1) : width;
    const path = pts
      .map((p, idx) => {
        const x = idx * stepX;
        const y = height - ((p.price - min) / range) * (height - 20) - 10;
        return `${idx === 0 ? 'M' : 'L'}${x},${y}`;
      })
      .join(' ');
    const color = isUp ? '#198754' : '#dc3545';
    return (
      <svg className="stock-service-history-chart" viewBox={`0 0 ${width} ${height}`} role="img" aria-label="7-day history">
        <defs>
          <linearGradient id="historyGradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity="0.25" />
            <stop offset="100%" stopColor={color} stopOpacity="0" />
          </linearGradient>
        </defs>
        <rect x="0" y="0" width={width} height={height} fill="#f8f9fa" />
        <path d={path} fill="none" stroke={color} strokeWidth="2" />
        <path
          d={`${path} L${width},${height} L0,${height} Z`}
          fill="url(#historyGradient)"
          stroke="none"
        />
      </svg>
    );
  };

  const renderSparkline = (symbol) => {
    const pts = getSessionPoints(symbol);
    if (!pts.length) return <p className="stock-service-history-empty">No session history yet.</p>;
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
    const width = 260;
    const height = 80;
    const stepX = pts.length > 1 ? width / (pts.length - 1) : width;
    const path = pts
      .map((p, idx) => {
        const x = idx * stepX;
        const y = height - ((p.price - min) / range) * (height - 10) - 5;
        return `${idx === 0 ? 'M' : 'L'}${x},${y}`;
      })
      .join(' ');
    return (
      <svg className="stock-service-history-chart" viewBox={`0 0 ${width} ${height}`} role="img" aria-label={`Session history for ${symbol}`}>
        <path d={path} fill="none" stroke="#0d6efd" strokeWidth="2" />
      </svg>
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
                                <div className="stock-service-history-today">
                                  <div className="stock-service-history-metric">
                                    <span className="label">Today</span>
                                  </div>
                                  <div className="stock-service-history-chart-wrap today">
                                    {renderSparkline(row.symbol)}
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
                                      const startDate = new Date(Number(first.t) * 1000);
                                      const endDate = new Date(Number(last.t) * 1000);
                                      const fmtDate = (d) =>
                                        Number.isNaN(d.getTime())
                                          ? '—'
                                          : new Intl.DateTimeFormat('sv-SE', {
                                              month: '2-digit',
                                              day: '2-digit',
                                            }).format(d);
                                      const up = abs >= 0;
                                      return (
                                        <>
                                          <div className="stock-service-history-summary">
                                            <div className="stock-service-history-metric">
                                              <span className="label">7d change</span>
                                              <span className={up ? 'value up' : 'value down'}>
                                                {abs >= 0 ? '+' : ''}
                                                {abs.toFixed(2)} ({abs >= 0 ? '+' : ''}
                                                {pct.toFixed(1)}%)
                                              </span>
                                            </div>
                                            <div className="stock-service-history-metric">
                                              <span className="label">High / Low</span>
                                              <span className="value">
                                                {high.toFixed(2)} / {low.toFixed(2)}
                                              </span>
                                            </div>
                                            <div className="stock-service-history-metric">
                                              <span className="label">Period</span>
                                              <span className="value">
                                                {fmtDate(startDate)} – {fmtDate(endDate)}
                                              </span>
                                            </div>
                                          </div>
                                          <div className="stock-service-history-chart-wrap">
                                            {renderHistoryChart(bars, up)}
                                          </div>
                                        </>
                                      );
                                    })()}
                                  </div>
                                )}
                                {!historyLoading && (!historyBySymbol[row.symbol]?.data?.length) && historyBySymbol[row.symbol] !== null && (
                                  <p className="stock-service-history-empty">No bars returned.</p>
                                )}
                                {!historyLoading && historyBySymbol[row.symbol] === undefined && !historyError && renderSparkline(row.symbol)}
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
