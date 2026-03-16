/**
 * Gateway session sub-page: paste and save IBKR Gateway api cookie (when using Gateway mode).
 */
import React, { useState } from 'react';
import { Link } from 'react-router-dom';
import PageTemplate from '../PageTemplate';
import { useKeycloak } from '../KeycloakProvider';
import './StockServicePage.css';

const GATEWAY_COOKIE_URL = '/api/v1/stock-service/ibkr/gateway-cookie';

export default function StockServiceGatewayPage() {
  const [gatewayCookie, setGatewayCookie] = useState('');
  const [cookieSaveStatus, setCookieSaveStatus] = useState(null);
  const { authenticatedFetch } = useKeycloak();
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
        }
      })
      .catch(() => setCookieSaveStatus({ ok: false, error: 'Request failed' }));
  };

  return (
    <PageTemplate title="Stock Service – Gateway session" subtitle="Save Gateway session cookie when needed">
      <div className="stock-service-content">
        <p className="stock-service-explainer">
          <Link to="/stock" className="stock-service-back-link">← Back to watchlist</Link>
        </p>
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
              {cookieSaveStatus.ok ? 'Cookie saved. Watchlist will refresh on next cycle.' : cookieSaveStatus.error}
            </p>
          )}
        </section>
      </div>
    </PageTemplate>
  );
}
