/**
 * Stock monitoring page template – uses main app login/logout (same SSO as Profile, Comp-AI).
 */
import React, { useEffect, useRef, useState } from 'react';
import { login, logout } from '../keycloak';

function getUserInitials(displayName) {
  const source = String(displayName || '').trim();
  if (!source) return 'U';
  const parts = source.split(/[\s._-]+/).filter(Boolean);
  if (parts.length >= 2) return `${parts[0][0] || ''}${parts[1][0] || ''}`.toUpperCase();
  return source.substring(0, 2).toUpperCase();
}

function looksLikeUserId(value) {
  const raw = String(value || '').trim();
  if (!raw) return true;
  const uuidLike = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(raw);
  return uuidLike || raw.includes('|') || raw.startsWith('auth0|');
}

export default function StockPageTemplate({
  authState,
  routePath,
  pageTitle,
  pageSubtitle,
  onNavigate,
  children,
}) {
  const currentYear = new Date().getFullYear();
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const userMenuRef = useRef(null);
  const isDashboardPage = !routePath.startsWith('/stock/watchlists') && !routePath.startsWith('/stock/alerts') && !routePath.startsWith('/stock/ops');
  const isWatchlistsPage = routePath.startsWith('/stock/watchlists');
  const isAlertsPage = routePath.startsWith('/stock/alerts');
  const isOpsPage = routePath.startsWith('/stock/ops');
  const preferredName = String(authState.displayName || '').trim();
  const displayUser = preferredName && !looksLikeUserId(preferredName) ? preferredName : (authState.email || 'User');
  const displayEmail = authState.email || '';
  const initials = getUserInitials(displayUser);

  useEffect(() => {
    const handleClickOutside = (event) => {
      if (userMenuRef.current && !userMenuRef.current.contains(event.target)) setUserMenuOpen(false);
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleLogin = (e) => {
    e.preventDefault();
    login(true);
  };

  const handleLogout = (e) => {
    e.preventDefault();
    setUserMenuOpen(false);
    logout();
  };

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
                <button className="user-dropdown-toggle" onClick={() => setUserMenuOpen((c) => !c)} aria-label="User menu" type="button">
                  <div className="user-avatar">{initials}</div>
                </button>
                {userMenuOpen && (
                  <div className="user-dropdown-menu">
                    <div className="user-dropdown-header">
                      <div className="user-avatar-large">{initials}</div>
                      <div className="user-info">
                        <div className="user-name">{displayUser}</div>
                        <div className="user-email">{displayEmail || 'Authenticated session'}</div>
                      </div>
                    </div>
                    <div className="user-dropdown-divider" />
                    <a href="/profile" className="user-dropdown-item" onClick={() => setUserMenuOpen(false)}>
                      <span className="dropdown-icon">👤</span> Profile
                    </a>
                    <a href="#" className="user-dropdown-item" onClick={handleLogout}>
                      <span className="dropdown-icon">🚪</span> Logout
                    </a>
                  </div>
                )}
              </div>
            ) : (
              <a href="#" className="profile-link" aria-label="Sign in with Keycloak" onClick={handleLogin}>
                <span className="profile-avatar">U</span>
                <span>Sign in</span>
              </a>
            )}
          </div>
        </header>

        <main className="main">
          <div className="page-header">
            <h1 className="page-title">{pageTitle}</h1>
            <p className="page-subtitle">{pageSubtitle}</p>
          </div>

          <div className="view-tabs">
            <button type="button" className={`view-tab ${isDashboardPage ? 'active' : ''}`} onClick={() => onNavigate('/stock')}>
              Dashboard
            </button>
            <button type="button" className={`view-tab ${isWatchlistsPage ? 'active' : ''}`} onClick={() => onNavigate('/stock/watchlists')}>
              Watchlists
            </button>
            <button type="button" className={`view-tab ${isAlertsPage ? 'active' : ''}`} onClick={() => onNavigate('/stock/alerts')}>
              Alerts
            </button>
            <button type="button" className={`view-tab ${isOpsPage ? 'active' : ''}`} onClick={() => onNavigate('/stock/ops')}>
              Ops
            </button>
          </div>

          {children}
        </main>

        <footer className="footer">
          <p>&copy; {currentYear} Lianel World. All rights reserved.</p>
        </footer>
      </div>
    </div>
  );
}
