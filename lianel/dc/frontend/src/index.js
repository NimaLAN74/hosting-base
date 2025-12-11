import React from 'react';
import ReactDOM from 'react-dom/client';
import './index.css';
import App from './App';
import { KeycloakProvider } from './KeycloakProvider';

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(
  <React.StrictMode>
    <KeycloakProvider>
      <App />
    </KeycloakProvider>
  </React.StrictMode>
);
