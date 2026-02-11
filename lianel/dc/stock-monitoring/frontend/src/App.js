import React from 'react';

function App() {
  return (
    <div style={{ padding: '2rem', fontFamily: 'sans-serif' }}>
      <h1>Stock Exchange Monitoring</h1>
      <p><strong>EU markets MVP</strong> – placeholder. Connect to shared Keycloak and stock-monitoring backend when ready.</p>
      <p style={{ color: '#666' }}>Build refresh: 2026-02-11 hotfix.</p>
      <ul>
        <li>Watchlists, alerts, dashboard to be implemented per function list.</li>
        <li>Auth: Keycloak (shared).</li>
      </ul>
    </div>
  );
}

export default App;
