/**
 * Stock service – watchlist: price table for chosen symbols (IBKR), refreshes every 60s.
 * When using the Gateway, you can save the session cookie here (no SSH needed).
 */
import React, { useEffect, useState, useCallback } from 'react';
import PageTemplate from '../PageTemplate';
import { useKeycloak } from '../KeycloakProvider';
import './StockServicePage.css';

const WATCHLIST_URL = '/api/v1/stock-service/watchlist';
const GATEWAY_COOKIE_URL = '/api/v1/stock-service/ibkr/gateway-cookie';
const WATCHLIST_REFRESH_MS = 60_000;

export default function StockServicePage() {
  const [watchlist, setWatchlist] = useState(null);
  const [watchlistLoading, setWatchlistLoading] = useState(true);
  const [watchlistError, setWatchlistError] = useState(null);
  const [gatewayCookie, setGatewayCookie] = useState('');
  const [cookieSaveStatus, setCookieSaveStatus] = useState(null);
  const { authenticatedFetch } = useKeycloak();

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
        setWatchlist(data);
        if (data && data.symbols && data.symbols.length && data.symbols.every((s) => s.error)) {
          setWatchlistError('Prices not yet available from provider. Check back in a minute.');
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

  const saveGatewayCookie = () => {
    const cookie = gatewayCookie.trim();
    if (!cookie) {
      setCookieSaveStatus({ ok: false, error: 'Enter the cookie value' });
      return;
    }
    setCookieSaveStatus(null);
    authenticatedFetch(GATEWAY_COOKIE_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ cookie }),
    })
      .then((r) => r.json())
      .then((data) => {
        setCookieSaveStatus(data?.ok ? { ok: true } : { ok: false, error: data?.error || 'Failed' });
        if (data?.ok) {
          setGatewayCookie('');
          loadWatchlist();
        }
      })
      .catch(() => setCookieSaveStatus({ ok: false, error: 'Request failed' }));
  };

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
          {watchlistError && !watchlistLoading && (
            <p className="stock-service-error" role="alert">
              {watchlistError}
            </p>
          )}
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
          {!watchlistLoading && (!watchlist || !watchlist.symbols?.length) && !watchlistError && (
            <p className="stock-service-explainer">No watchlist data available.</p>
          )}
        </section>
        <section className="stock-service-card stock-service-gateway-card">
          <h2 className="stock-service-card-title">Gateway session</h2>
          <p className="stock-service-explainer">
            If the watchlist shows &quot;Please query /accounts first&quot; or &quot;not authenticated&quot;, log in at{' '}
            <a href="/ibkr-gateway/" target="_blank" rel="noopener noreferrer">Gateway</a>, then paste the <code>api</code> cookie here (DevTools → Application → Cookies → <code>api</code>).
          </p>
          <div className="stock-service-gateway-form">
            <input
              type="text"
              placeholder="Paste api cookie value"
              value={gatewayCookie}
              onChange={(e) => setGatewayCookie(e.target.value)}
              className="stock-service-cookie-input"
              aria-label="Gateway session cookie"
            />
            <button type="button" onClick={saveGatewayCookie} className="stock-service-save-btn">
              Save
            </button>
          </div>
          {cookieSaveStatus && (
            <p className={cookieSaveStatus.ok ? 'stock-service-cookie-ok' : 'stock-service-error'} role="status">
              {cookieSaveStatus.ok ? 'Cookie saved. Watchlist will refresh.' : cookieSaveStatus.error}
            </p>
          )}
        </section>
      </div>
    </PageTemplate>
  );
}
