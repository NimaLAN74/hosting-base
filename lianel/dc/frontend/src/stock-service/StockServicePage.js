/**
 * Stock service – IBKR authentication verification.
 * Uses the same PageTemplate as Profile, Services, Monitoring, etc.
 * Shows whether IBKR OAuth is configured and allows verifying the connection (LST).
 */
import React, { useEffect, useState } from 'react';
import PageTemplate from '../PageTemplate';
import { useKeycloak } from '../KeycloakProvider';
import { apiJson } from './stockApiClient';
import './StockServicePage.css';

const API_IBKR_VERIFY = '/ibkr/verify';

export default function StockServicePage() {
  const { authenticated } = useKeycloak();
  const [status, setStatus] = useState(null);
  const [ibkrVerify, setIbkrVerify] = useState(null);
  const [loading, setLoading] = useState(true);
  const [verifying, setVerifying] = useState(false);

  const loadStatus = () => {
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
    if (!authenticated) return;
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

  const ibkrConfigured = status && status.ibkr_oauth_configured === true;

  return (
    <PageTemplate
      title="Stock Service"
      subtitle="IBKR authentication verification"
    >
      <div className="stock-service-content">
        <section className="stock-service-card">
          <h2 className="stock-service-card-title">IBKR authentication</h2>
          {loading && <p className="stock-service-loading">Loading…</p>}
          {!loading && status && (
            <>
              <div className="stock-service-status-row">
                <span className="stock-service-status-label">Connection to Interactive Brokers (IBKR):</span>
                {ibkrConfigured ? (
                  <span className="stock-service-badge stock-service-badge-ok">Configured</span>
                ) : (
                  <span className="stock-service-badge stock-service-badge-warn">Not configured</span>
                )}
              </div>
              {ibkrConfigured ? (
                <p className="stock-service-explainer">
                  The server can talk to the IBKR API. Use the button below to confirm the connection works.
                </p>
              ) : (
                <div className="stock-service-not-configured">
                  <p className="stock-service-explainer">
                    <strong>What this means:</strong> The server is not yet set up to connect to Interactive Brokers.
                    No API credentials or key files are configured, so verification is not available.
                  </p>
                  <p className="stock-service-explainer">
                    <strong>What to do:</strong> An administrator must add IBKR OAuth credentials and PEM key paths
                    in the server environment (e.g. <code>IBKR_OAUTH_*</code> variables). Once that is done, this page
                    will show “Configured” and you can run verification.
                  </p>
                </div>
              )}
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
              {!authenticated && (
                <p className="stock-service-hint">
                  Sign in (use the menu above) to verify IBKR authentication.
                </p>
              )}
            </>
          )}
          {!loading && !status && (
            <p className="stock-service-error">Could not load service status.</p>
          )}
        </section>
      </div>
    </PageTemplate>
  );
}
