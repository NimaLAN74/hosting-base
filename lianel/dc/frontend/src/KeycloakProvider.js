// Keycloak Context Provider for React
import React, { createContext, useContext, useEffect, useState } from 'react';
import { initKeycloak, login, logout, getToken, getUserInfo, isAuthenticated, hasRole, authenticatedFetch, keycloak } from './keycloak';

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
    // Check if we're returning from Keycloak login (has code parameter)
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get('code');
    const isCallback = !!code;

    // Initialize Keycloak
    initKeycloak()
      .then((auth) => {
        setAuthenticated(auth);
        if (auth) {
          setUserInfo(getUserInfo());
        }
        setKeycloakReady(true);
        
        // If we came from a callback and are now authenticated, update state
        if (isCallback && auth) {
          console.log('Login callback detected, user authenticated');
          // Force state update to ensure UI reflects authentication
          setAuthenticated(true);
          setUserInfo(getUserInfo());
        }
      })
      .catch((error) => {
        console.error('Keycloak initialization error:', error);
        setKeycloakReady(true);
      });

    // Listen for token updates and force refresh to get updated roles
    const updateToken = async () => {
      if (isAuthenticated()) {
        try {
          // Force token refresh to get updated roles
          const refreshed = await keycloak.updateToken(70);
          if (refreshed) {
            console.log('Token refreshed, updating user info');
            setUserInfo(getUserInfo());
            setAuthenticated(true); // Ensure state is updated
          } else {
            // Token still valid, just update user info
            setUserInfo(getUserInfo());
            setAuthenticated(true); // Ensure state is updated
          }
        } catch (error) {
          console.error('Token refresh failed:', error);
          setUserInfo(getUserInfo()); // Still update with current token
          // Check if still authenticated
          setAuthenticated(isAuthenticated());
        }
      } else {
        // Not authenticated - update state
        setAuthenticated(false);
      }
    };

    // Check token updates every 30 seconds and force refresh on mount
    updateToken(); // Refresh immediately on mount
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

