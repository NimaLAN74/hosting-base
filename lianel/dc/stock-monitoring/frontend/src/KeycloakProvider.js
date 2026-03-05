import React, { createContext, useContext, useEffect, useState } from 'react';
import {
  initKeycloak,
  login,
  logout,
  getToken,
  getUserInfo,
  isAuthenticated,
  authenticatedFetch,
  keycloak,
} from './keycloak';

const KeycloakContext = createContext(null);

export const useKeycloak = () => {
  const context = useContext(KeycloakContext);
  if (!context) {
    throw new Error('useKeycloak must be used within KeycloakProvider');
  }
  return context;
};

export const KeycloakProvider = ({ children }) => {
  const [ready, setReady] = useState(false);
  const [authenticated, setAuthenticated] = useState(false);
  const [userInfo, setUserInfo] = useState(null);

  useEffect(() => {
    const updateAuth = () => {
      setAuthenticated(isAuthenticated());
      setUserInfo(isAuthenticated() ? getUserInfo() : null);
    };

    initKeycloak()
      .then((auth) => {
        updateAuth();
        setReady(true);
      })
      .catch(() => {
        setReady(true);
        setAuthenticated(false);
      });

    const onAuthSuccess = () => {
      setTimeout(updateAuth, 50);
    };
    const onAuthError = () => {
      setAuthenticated(false);
      setUserInfo(null);
    };
    const onTokenExpired = () => {
      keycloak.updateToken(70).then((refreshed) => {
        if (refreshed) updateAuth();
        else setAuthenticated(false);
      }).catch(() => setAuthenticated(false));
    };

    keycloak.onAuthSuccess = onAuthSuccess;
    keycloak.onAuthError = onAuthError;
    keycloak.onTokenExpired = onTokenExpired;

    return () => {
      keycloak.onAuthSuccess = undefined;
      keycloak.onAuthError = undefined;
      keycloak.onTokenExpired = undefined;
    };
  }, []);

  const value = {
    keycloakReady: ready,
    authenticated,
    userInfo,
    getToken,
    login,
    logout,
    authenticatedFetch,
  };

  return (
    <KeycloakContext.Provider value={value}>
      {children}
    </KeycloakContext.Provider>
  );
};
