/**
 * Stock service – watchlist: price table for chosen symbols (IBKR), refreshes every 60s.
 */
import React, { useEffect, useState, useCallback, useRef } from 'react';
import { Link } from 'react-router-dom';
import PageTemplate from '../PageTemplate';
import './StockServicePage.css';

const WATCHLIST_URL = '/api/v1/stock-service/watchlist';
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
      watchlist.symbols.forEach((s) => {
        if (s.price != null) next[s.symbol] = Number(s.price);
      });
      previousPricesRef.current = lastPricesRef.current;
      lastPricesRef.current = next;
    }
  }, [watchlist]);

  const getChangeIndicator = (symbol, currentPrice) => {
    if (currentPrice == null) return null;
    const prev = previousPricesRef.current[symbol];
    if (prev == null) return '—';
    const cur = Number(currentPrice);
    if (cur > prev) return 'up';
    if (cur < prev) return 'down';
    return 'unchanged';
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
              <table className="stock-service-watchlist-table" aria-label="Watchlist prices">
                <thead>
                  <tr>
                    <th>Symbol</th>
                    <th>Name</th>
                    <th>Price</th>
                    <th>Change</th>
                    <th>Currency</th>
                    <th>Updated</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {watchlist.symbols.map((row) => {
                    const change = getChangeIndicator(row.symbol, row.price);
                    return (
                      <tr key={row.symbol}>
                        <td className="stock-service-wl-symbol">{row.symbol}</td>
                        <td className="stock-service-wl-name">{SYMBOL_SHORT_NAMES[row.symbol] || row.symbol}</td>
                        <td className="stock-service-wl-price">
                          {row.price != null
                            ? Number(row.price).toLocaleString(undefined, {
                                minimumFractionDigits: 2,
                                maximumFractionDigits: 4,
                              })
                            : '—'}
                        </td>
                        <td className="stock-service-wl-change">
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
                            <span className="stock-service-wl-ok">OK</span>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
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
