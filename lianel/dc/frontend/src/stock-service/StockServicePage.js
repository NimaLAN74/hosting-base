/**
 * Stock service – simple IBKR authentication verification.
 * Shows whether IBKR OAuth is configured and allows verifying the connection (LST).
 */
import React, { useEffect, useState } from 'react';
import { useKeycloak } from '../KeycloakProvider';
import { apiJson } from './stockApiClient';
import './StockServicePage.css';

const API_STATUS = '/status';
const API_IBKR_VERIFY = '/ibkr/verify';

export default function StockServicePage() {
  const { keycloak, authenticated } = useKeycloak();
  const [status, setStatus] = useState(null);
  const [ibkrVerify, setIbkrVerify] = useState(null);
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(true);
  const [verifying, setVerifying] = useState(false);

  const loadStatus = () => {
    setError(null);
    setLoading(true);
    setStatus(null);
    setIbkrVerify(null);
    fetch('/api/v1/stock-service/status', { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => r.json())
      .then((data) => setStatus(data))
      .catch(() => setStatus(null))
      .finally(() => setLoading(false));
  };

  const verifyIbkr = async () => {
    if (!authenticated) {
      setError('Sign in first to verify IBKR.');
      return;
    }
    setError(null);
    setIbkrVerify(null);
    setVerifying(true);
    try {
      const data = await apiJson(API_IBKR_VERIFY);
      setIbkrVerify(data);
    } catch (e) {
      setIbkrVerify({ ok: false, error: e?.message || 'Request failed' });
    } finally {
      setVerifying(false);
    }
  };

  useEffect(() => {
    loadStatus();
  }, [authenticated]);

  const login = () => {
    if (keycloak) keycloak.login();
  };

  const logout = () => {
    if (keycloak) keycloak.logout();
  };

  const ibkrConfigured = status && status.ibkr_oauth_configured === true;

  return (
    <div className="stock-service-page">
      <header className="stock-service-header">
        <h1>Stock Service</h1>
        <p className="stock-service-subtitle">IBKR authentication verification</p>
      </header>

      <section className="stock-service-card">
        <h2>IBKR authentication</h2>
        {loading && <p className="stock-service-loading">Loading…</p>}
        {!loading && status && (
          <>
            <p>
              <strong>IBKR OAuth:</strong>{' '}
              {ibkrConfigured ? (
                <span className="stock-service-ok">Configured</span>
              ) : (
                <span className="stock-service-warn">Not configured</span>
              )}
            </p>
            {authenticated && ibkrConfigured && (
              <div className="stock-service-verify-block">
                <button
                  type="button"
                  className="stock-service-btn primary"
                  onClick={verifyIbkr}
                  disabled={verifying}
                >
                  {verifying ? 'Verifying…' : 'Verify IBKR'}
                </button>
                {ibkrVerify !== null && (
                  <div className={ibkrVerify.ok ? 'stock-service-success' : 'stock-service-error'}>
                    {ibkrVerify.ok ? (
                      <p>{ibkrVerify.message || 'IBKR authentication verified.'}</p>
                    ) : (
                      <p>{ibkrVerify.error || 'Verification failed.'}</p>
                    )}
                  </div>
                )}
              </div>
            )}
            {authenticated && !ibkrConfigured && (
              <p className="stock-service-hint">Configure IBKR OAuth on the server to enable verification.</p>
            )}
            {!authenticated && (
              <p className="stock-service-hint">Sign in to verify IBKR authentication.</p>
            )}
          </>
        )}
        {!loading && !status && (
          <p className="stock-service-error">Could not load service status.</p>
        )}
      </section>

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
