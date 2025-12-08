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
    // Clear all cookies for this domain and Keycloak domain
    // This ensures both OAuth2-proxy and Keycloak sessions are cleared
    const cookies = document.cookie.split(';');
    cookies.forEach(cookie => {
      const eqPos = cookie.indexOf('=');
      const name = eqPos > -1 ? cookie.substr(0, eqPos).trim() : cookie.trim();
      // Clear cookies for current domain
      document.cookie = `${name}=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.lianel.se`;
      document.cookie = `${name}=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/`;
    });
    
    // Clear Keycloak cookies explicitly
    document.cookie = 'AUTH_SESSION_ID=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.lianel.se';
    document.cookie = 'KEYCLOAK_SESSION=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.lianel.se';
    document.cookie = 'KEYCLOAK_IDENTITY=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.lianel.se';
    document.cookie = 'KEYCLOAK_SESSION_LEGACY=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.lianel.se';
    
    // Clear OAuth2-proxy cookies
    document.cookie = '_oauth2_proxy=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.lianel.se';
    document.cookie = '_oauth2_proxy_csrf=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.lianel.se';
    
    // Redirect to OAuth2-proxy sign_out, then Keycloak logout
    window.location.href = '/oauth2/sign_out?rd=https://auth.lianel.se/realms/lianel/protocol/openid-connect/logout?client_id=oauth2-proxy&redirect_uri=' + encodeURIComponent('https://www.lianel.se/');
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

