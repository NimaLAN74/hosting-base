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

    // Function to update authentication state
    const updateAuthState = () => {
      const auth = isAuthenticated();
      setAuthenticated(auth);
      if (auth) {
        setUserInfo(getUserInfo());
      }
    };

    // Initialize Keycloak
    initKeycloak()
      .then((auth) => {
        console.log('Keycloak initialized, authenticated:', auth);
        updateAuthState();
        setKeycloakReady(true);
        
        // If we came from a callback, wait a bit for Keycloak to process it
        if (isCallback) {
          console.log('Login callback detected, waiting for token exchange...');
          // Give Keycloak time to exchange the code for a token
          setTimeout(() => {
            updateAuthState();
            // Double-check after a short delay
            setTimeout(() => {
              updateAuthState();
            }, 500);
          }, 100);
        }
      })
      .catch((error) => {
        console.error('Keycloak initialization error:', error);
        setKeycloakReady(true);
        setAuthenticated(false);
      });

    // Set up Keycloak event listeners for auth state changes
    const onAuthSuccess = () => {
      console.log('Keycloak onAuthSuccess event - updating state');
      // Force immediate state update
      setTimeout(() => {
        updateAuthState();
        // Double-check after a short delay to ensure state is correct
        setTimeout(() => {
          updateAuthState();
        }, 100);
      }, 50);
    };

    const onAuthError = () => {
      console.log('Keycloak onAuthError event');
      setAuthenticated(false);
      setUserInfo(null);
    };

    const onTokenExpired = () => {
      console.log('Keycloak onTokenExpired event');
      // Try to refresh token
      keycloak.updateToken(70)
        .then((refreshed) => {
          if (refreshed) {
            console.log('Token refreshed after expiry');
            updateAuthState();
          } else {
            setAuthenticated(false);
          }
        })
        .catch(() => {
          setAuthenticated(false);
        });
    };

    // Register event listeners
    keycloak.onAuthSuccess = onAuthSuccess;
    keycloak.onAuthError = onAuthError;
    keycloak.onTokenExpired = onTokenExpired;
    
    // Also listen for auth refresh success
    keycloak.onAuthRefreshSuccess = () => {
      console.log('Keycloak onAuthRefreshSuccess event');
      updateAuthState();
    };

    // Listen for token updates and force refresh to get updated roles
    const updateToken = async () => {
      if (isAuthenticated()) {
        try {
          // Force token refresh to get updated roles
          const refreshed = await keycloak.updateToken(70);
          if (refreshed) {
            console.log('Token refreshed, updating user info');
            updateAuthState();
          } else {
            // Token still valid, just update user info
            updateAuthState();
          }
        } catch (error) {
          console.error('Token refresh failed:', error);
          updateAuthState();
        }
      } else {
        // Not authenticated - update state
        setAuthenticated(false);
      }
    };

    // Check token updates every 30 seconds and force refresh on mount
    updateToken(); // Refresh immediately on mount
    const tokenInterval = setInterval(updateToken, 30000);

    // Also watch for URL changes (callback handling)
    const handleLocationChange = () => {
      const newParams = new URLSearchParams(window.location.search);
      const newCode = newParams.get('code');
      if (newCode && !isCallback) {
        // New callback detected, update state
        setTimeout(() => {
          updateAuthState();
        }, 200);
      }
    };

    // Listen for popstate (back/forward) and hashchange
    window.addEventListener('popstate', handleLocationChange);

    return () => {
      clearInterval(tokenInterval);
      window.removeEventListener('popstate', handleLocationChange);
      // Clean up event listeners
      keycloak.onAuthSuccess = null;
      keycloak.onAuthError = null;
      keycloak.onTokenExpired = null;
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

