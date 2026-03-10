/**
 * Stock service – watchlist: price table for chosen symbols (IBKR), refreshes every 60s.
 */
import React, { useEffect, useState, useCallback } from 'react';
import PageTemplate from '../PageTemplate';
import './StockServicePage.css';

const WATCHLIST_URL = '/api/v1/stock-service/watchlist';
const WATCHLIST_REFRESH_MS = 60_000;

export default function StockServicePage() {
  const [watchlist, setWatchlist] = useState(null);
  const [watchlistLoading, setWatchlistLoading] = useState(true);

  const loadWatchlist = useCallback(() => {
    setWatchlistLoading(true);
    fetch(WATCHLIST_URL, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => (r.ok ? r.json() : null))
      .then((data) => setWatchlist(data))
      .catch(() => setWatchlist(null))
      .finally(() => setWatchlistLoading(false));
  }, []);

  useEffect(() => {
    loadWatchlist();
    const id = setInterval(loadWatchlist, WATCHLIST_REFRESH_MS);
    return () => clearInterval(id);
  }, [loadWatchlist]);

  return (
    <PageTemplate
      title="Stock Service"
      subtitle="Watchlist – live prices (IBKR, every 60s)"
    >
      <div className="stock-service-content">
        <section className="stock-service-card stock-service-watchlist-card">
          <h2 className="stock-service-card-title">Prices</h2>
          <p className="stock-service-explainer">
            Chosen symbols. Data from IBKR. Updates every 60 seconds.
          </p>
          {watchlistLoading && <p className="stock-service-loading">Loading…</p>}
          {!watchlistLoading && watchlist && watchlist.symbols && watchlist.symbols.length > 0 && (
            <>
              <div className="stock-service-watchlist-meta">
                <span>As of: {watchlist.as_of || '—'}</span>
                {watchlist.provider && (
                  <span className="stock-service-watchlist-provider">{watchlist.provider}</span>
                )}
              </div>
              <table className="stock-service-watchlist-table" aria-label="Watchlist prices">
                <thead>
                  <tr>
                    <th>Symbol</th>
                    <th>Price</th>
                    <th>Currency</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {watchlist.symbols.map((row) => (
                    <tr key={row.symbol}>
                      <td className="stock-service-wl-symbol">{row.symbol}</td>
                      <td className="stock-service-wl-price">
                        {row.price != null
                          ? Number(row.price).toLocaleString(undefined, {
                              minimumFractionDigits: 2,
                              maximumFractionDigits: 4,
                            })
                          : '—'}
                      </td>
                      <td>{row.currency || '—'}</td>
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
                  ))}
                </tbody>
              </table>
            </>
          )}
          {!watchlistLoading && (!watchlist || !watchlist.symbols?.length) && (
            <p className="stock-service-explainer">No watchlist data available.</p>
          )}
        </section>
      </div>
    </PageTemplate>
  );
}
