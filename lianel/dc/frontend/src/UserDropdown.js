import React, { useState, useEffect, useRef } from 'react';
import { useKeycloak } from './KeycloakProvider';
import './UserDropdown.css';

function UserDropdown() {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef(null);
  const { userInfo, logout: keycloakLogout } = useKeycloak();

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
    // Keycloak handles logout and redirects to post_logout_redirect_uri
    keycloakLogout();
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

