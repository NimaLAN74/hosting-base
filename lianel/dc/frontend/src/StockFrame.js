/**
 * Stock Monitoring UI inside the main app (same KeycloakProvider as Profile).
 * Renders the stock app in an iframe so auth is shared; persists token to localStorage
 * before loading the iframe so the iframe content (same origin) sees it. Use route /stock-app –
 * links should go here instead of /stock so the user stays in the main app and stays authenticated.
 */
import React, { useEffect, useState } from 'react';
import { useKeycloak } from './KeycloakProvider';
import { persistTokenForStock } from './keycloak';
import './StockFrame.css';

function StockFrame() {
  const { authenticated, login } = useKeycloak();
  const [tokenPersisted, setTokenPersisted] = useState(false);

  useEffect(() => {
    if (authenticated) {
      persistTokenForStock();
      setTokenPersisted(true);
    } else {
      setTokenPersisted(false);
    }
  }, [authenticated]);

  if (!authenticated) {
    return (
      <div className="stock-frame-gate">
        <p>Authentication required to open Stock Monitoring.</p>
        <button type="button" className="stock-frame-login-btn" onClick={() => login(true)}>
          Sign in
        </button>
      </div>
    );
  }

  // Only render iframe after token is in localStorage so the stock app sees it on load
  if (!tokenPersisted) {
    return <div className="stock-frame-gate">Loading…</div>;
  }

  return (
    <iframe
      src="/stock/"
      title="Stock Exchange Monitoring"
      className="stock-frame-iframe"
    />
  );
}

export default StockFrame;
