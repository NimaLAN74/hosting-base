// Keycloak Context Provider for React
import React, { createContext, useContext, useEffect, useState } from 'react';
import { initKeycloak, login, logout, getToken, getUserInfo, isAuthenticated, hasRole, authenticatedFetch } from './keycloak';

const KeycloakContext = createContext(null);

export const useKeycloak = () => {
  const context = useContext(KeycloakContext);
  if (!context) {
    throw new Error('useKeycloak must be used within KeycloakProvider');
  }
  return context;
};

export const KeycloakProvider = ({ children }) => {
  const [keycloakReady, setKeycloakReady] = useState(false);
  const [authenticated, setAuthenticated] = useState(false);
  const [userInfo, setUserInfo] = useState(null);

  useEffect(() => {
    // Initialize Keycloak
    initKeycloak()
      .then((auth) => {
        setAuthenticated(auth);
        if (auth) {
          setUserInfo(getUserInfo());
        }
        setKeycloakReady(true);
      })
      .catch((error) => {
        console.error('Keycloak initialization error:', error);
        setKeycloakReady(true);
      });

    // Listen for token updates
    const updateToken = () => {
      if (isAuthenticated()) {
        setUserInfo(getUserInfo());
      }
    };

    // Check token updates every 30 seconds
    const tokenInterval = setInterval(updateToken, 30000);

    return () => {
      clearInterval(tokenInterval);
    };
  }, []);

  const value = {
    keycloakReady,
    authenticated,
    userInfo,
    login,
    logout,
    getToken,
    getUserInfo,
    hasRole,
    authenticatedFetch
  };

  // Show loading state while Keycloak initializes
  if (!keycloakReady) {
    return (
      <div style={{ 
        display: 'flex', 
        justifyContent: 'center', 
        alignItems: 'center', 
        height: '100vh',
        flexDirection: 'column'
      }}>
        <div style={{ fontSize: '18px', marginBottom: '10px' }}>Loading...</div>
        <div style={{ fontSize: '14px', color: '#666' }}>Initializing authentication</div>
      </div>
    );
  }

  return (
    <KeycloakContext.Provider value={value}>
      {children}
    </KeycloakContext.Provider>
  );
};

