/**
 * Stock service – simple SSO verification page.
 * Verifies that the same Keycloak token used by the main app works with the stock service backend.
 */
import React, { useEffect, useState } from 'react';
import { useKeycloak } from '../KeycloakProvider';
import { apiJson } from './stockApiClient';
import './StockServicePage.css';

const API_ME = '/me';
const API_STATUS = '/status';

export default function StockServicePage() {
  const { keycloak, authenticated } = useKeycloak();
  const [me, setMe] = useState(null);
  const [status, setStatus] = useState(null);
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(true);

  const verify = () => {
    setError(null);
    setLoading(true);
    setMe(null);
    setStatus(null);

    const load = async () => {
      try {
        const [meRes, statusRes] = await Promise.all([
          authenticated ? apiJson(API_ME).catch((e) => ({ error: e.message, status: e.status })) : Promise.resolve(null),
          fetch('/api/v1/stock-service/status', { credentials: 'include', headers: { Accept: 'application/json' } })
            .then((r) => r.json())
            .catch(() => null),
        ]);

        if (meRes && meRes.error) {
          setError(meRes.error);
          setMe(null);
        } else if (meRes) {
          setMe(meRes);
          setError(null);
        } else if (!authenticated) {
          setError('Not logged in. Sign in with Keycloak to verify SSO.');
        }

        setStatus(statusRes);
      } catch (e) {
        setError(e?.message || 'Request failed');
        setMe(null);
      } finally {
        setLoading(false);
      }
    };

    load();
  };

  useEffect(() => {
    verify();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- only re-run when login state changes
  }, [authenticated]);

  const login = () => {
    if (keycloak) keycloak.login();
  };

  const logout = () => {
    if (keycloak) keycloak.logout();
  };

  return (
    <div className="stock-service-page">
      <header className="stock-service-header">
        <h1>Stock Service</h1>
        <p className="stock-service-subtitle">SSO verification</p>
      </header>

      <section className="stock-service-card">
        <h2>Authentication &amp; SSO</h2>
        {loading && <p className="stock-service-loading">Verifying…</p>}
        {!loading && error && (
          <div className="stock-service-error">
            <p><strong>Verification failed</strong></p>
            <p>{error}</p>
            {!authenticated && (
              <button type="button" className="stock-service-btn primary" onClick={login}>
                Sign in with Keycloak
              </button>
            )}
          </div>
        )}
        {!loading && !error && me && (
          <div className="stock-service-success">
            <p><strong>SSO verified</strong></p>
            <p>You are logged in as: <strong>{me.display_name || me.user_id || me.email || 'User'}</strong></p>
            {me.email && <p className="stock-service-email">{me.email}</p>}
            <p className="stock-service-hint">The same Keycloak token is accepted by the stock service backend.</p>
            <button type="button" className="stock-service-btn" onClick={verify}>
              Verify again
            </button>
          </div>
        )}
        {!loading && !error && !me && authenticated && (
          <p>No /me response. Check backend and token.</p>
        )}
      </section>

      {status && (
        <section className="stock-service-card">
          <h2>Service status</h2>
          <pre className="stock-service-status-json">
            {JSON.stringify(status, null, 2)}
          </pre>
        </section>
      )}

      <footer className="stock-service-footer">
        {authenticated ? (
          <button type="button" className="stock-service-btn secondary" onClick={logout}>
            Sign out
          </button>
        ) : (
          <button type="button" className="stock-service-btn primary" onClick={login}>
            Sign in
          </button>
        )}
      </footer>
    </div>
  );
}
