import React, { useState, useEffect, useRef } from 'react';
import './UserDropdown.css';

function UserDropdown() {
  const [isOpen, setIsOpen] = useState(false);
  const [userInfo, setUserInfo] = useState(null);
  const dropdownRef = useRef(null);

  useEffect(() => {
    // Get user info from headers set by OAuth2-proxy
    const getUserInfo = async () => {
      try {
        // Fetch user info from profile service API
        const response = await fetch('/api/profile', {
          method: 'GET',
          credentials: 'include'
        });
        
        if (response.ok) {
          const data = await response.json();
          setUserInfo({
            username: data.username || 'User',
            email: data.email || '',
            name: data.name || data.firstName || data.username || 'User'
          });
        } else {
          // Fallback: use defaults
          setUserInfo({
            username: 'User',
            email: '',
            name: 'User'
          });
        }
      } catch (error) {
        // Fallback user info
        setUserInfo({
          username: 'User',
          email: '',
          name: 'User'
        });
      }
    };

    getUserInfo();
  }, []);

  useEffect(() => {
    const handleClickOutside = (event) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  const handleLogout = () => {
    // Clear OAuth2-proxy cookies first (can clear same-origin cookies)
    const cookies = document.cookie.split(';');
    cookies.forEach(cookie => {
      const eqPos = cookie.indexOf('=');
      const name = eqPos > -1 ? cookie.substr(0, eqPos).trim() : cookie.trim();
      // Clear cookies for current domain and parent domain
      document.cookie = `${name}=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.lianel.se`;
      document.cookie = `${name}=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/`;
    });
    
    // Redirect to OAuth2-proxy sign_out first (clears OAuth2-proxy session)
    // Then redirect to Keycloak logout with id_token_hint to properly clear Keycloak session
    // Use a new window/tab to ensure cookies are cleared, then redirect back
    window.location.href = '/oauth2/sign_out?rd=' + encodeURIComponent('https://auth.lianel.se/realms/lianel/protocol/openid-connect/logout?client_id=oauth2-proxy&post_logout_redirect_uri=' + encodeURIComponent('https://www.lianel.se/'));
  };

  const getInitials = () => {
    if (userInfo?.name) {
      return userInfo.name
        .split(' ')
        .map(n => n[0])
        .join('')
        .toUpperCase()
        .substring(0, 2);
    }
    if (userInfo?.username) {
      return userInfo.username.substring(0, 2).toUpperCase();
    }
    return 'U';
  };

  return (
    <div className="user-dropdown" ref={dropdownRef}>
      <button 
        className="user-dropdown-toggle"
        onClick={() => setIsOpen(!isOpen)}
        aria-label="User menu"
      >
        <div className="user-avatar">
          {getInitials()}
        </div>
      </button>
      
      {isOpen && (
        <div className="user-dropdown-menu">
          <div className="user-dropdown-header">
            <div className="user-avatar-large">
              {getInitials()}
            </div>
            <div className="user-info">
              <div className="user-name">{userInfo?.name || userInfo?.username || 'User'}</div>
              <div className="user-email">{userInfo?.email || ''}</div>
            </div>
          </div>
          
          <div className="user-dropdown-divider"></div>
          
          <a href="/profile" className="user-dropdown-item" onClick={() => setIsOpen(false)}>
            <span className="dropdown-icon">👤</span>
            Profile
          </a>
          
          <button className="user-dropdown-item" onClick={handleLogout}>
            <span className="dropdown-icon">🚪</span>
            Logout
          </button>
        </div>
      )}
    </div>
  );
}

export default UserDropdown;

