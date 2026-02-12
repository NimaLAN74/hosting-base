import React, { useEffect, useRef, useState } from 'react';
import { getLoginUrl, getLogoutUrl } from './apiClient';

function getUserInitials(userId) {
  const source = String(userId || '').trim();
  if (!source) {
    return 'U';
  }
  const parts = source.split(/[\s._-]+/).filter(Boolean);
  if (parts.length >= 2) {
    return `${parts[0][0] || ''}${parts[1][0] || ''}`.toUpperCase();
  }
  return source.substring(0, 2).toUpperCase();
}

function StockPageTemplate({
  authState,
  routePath,
  pageTitle,
  pageSubtitle,
  onNavigate,
  children,
}) {
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const userMenuRef = useRef(null);
  const isDashboardPage = !routePath.startsWith('/stock/watchlists')
    && !routePath.startsWith('/stock/alerts')
    && !routePath.startsWith('/stock/ops');
  const isWatchlistsPage = routePath.startsWith('/stock/watchlists');
  const isAlertsPage = routePath.startsWith('/stock/alerts');
  const isOpsPage = routePath.startsWith('/stock/ops');
  const displayUser = authState.userId || 'User';
  const initials = getUserInitials(displayUser);

  useEffect(() => {
    const handleClickOutside = (event) => {
      if (userMenuRef.current && !userMenuRef.current.contains(event.target)) {
        setUserMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <div className="App stock-app">
      <div className="container">
        <header className="header">
          <a href="/" className="logo">
            <div className="logo-icon">LW</div>
            Lianel World
          </a>
          <div className="header-right">
            {authState.checking ? (
              <span className="last-updated">Auth...</span>
            ) : authState.isAuthenticated ? (
              <div className="user-dropdown" ref={userMenuRef}>
                <button
                  className="user-dropdown-toggle"
                  onClick={() => setUserMenuOpen((current) => !current)}
                  aria-label="User menu"
                  type="button"
                >
                  <div className="user-avatar">{initials}</div>
                </button>
                {userMenuOpen && (
                  <div className="user-dropdown-menu">
                    <div className="user-dropdown-header">
                      <div className="user-avatar-large">{initials}</div>
                      <div className="user-info">
                        <div className="user-name">{displayUser}</div>
                        <div className="user-email">Authenticated session</div>
                      </div>
                    </div>
                    <div className="user-dropdown-divider"></div>
                    <a href="/profile" className="user-dropdown-item" onClick={() => setUserMenuOpen(false)}>
                      <span className="dropdown-icon">👤</span>
                      Profile
                    </a>
                    <a href="/services" className="user-dropdown-item" onClick={() => setUserMenuOpen(false)}>
                      <span className="dropdown-icon">🧩</span>
                      Services
                    </a>
                    <a href={getLogoutUrl('/stock')} className="user-dropdown-item" onClick={() => setUserMenuOpen(false)}>
                      <span className="dropdown-icon">🚪</span>
                      Logout
                    </a>
                  </div>
                )}
              </div>
            ) : (
              <a
                href={getLoginUrl(routePath)}
                className="profile-link"
                aria-label="Sign in with Keycloak"
              >
                <span className="profile-avatar">U</span>
                <span>Sign in</span>
              </a>
            )}
          </div>
        </header>

        <main className="main">
          <div className="page-header">
            <a href="/" className="back-to-home-btn">← Back to Home</a>
            <h1 className="page-title">{pageTitle}</h1>
            <p className="page-subtitle">{pageSubtitle}</p>
          </div>

          <div className="view-tabs">
            <button
              type="button"
              className={`view-tab ${isDashboardPage ? 'active' : ''}`}
              onClick={() => onNavigate('/stock')}
            >
              Dashboard
            </button>
            <button
              type="button"
              className={`view-tab ${isWatchlistsPage ? 'active' : ''}`}
              onClick={() => onNavigate('/stock/watchlists')}
            >
              Watchlists
            </button>
            <button
              type="button"
              className={`view-tab ${isAlertsPage ? 'active' : ''}`}
              onClick={() => onNavigate('/stock/alerts')}
            >
              Alerts
            </button>
            <button
              type="button"
              className={`view-tab ${isOpsPage ? 'active' : ''}`}
              onClick={() => onNavigate('/stock/ops')}
            >
              Ops
            </button>
          </div>

          {children}
        </main>

        <footer className="footer">
          <p>&copy; 2026 Lianel World. All rights reserved.</p>
        </footer>
      </div>
    </div>
  );
}

export default StockPageTemplate;
