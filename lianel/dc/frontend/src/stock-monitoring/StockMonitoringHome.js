import React from 'react';
import PageTemplate from '../PageTemplate';
import './StockMonitoring.css';

function StockMonitoringHome() {
  return (
    <PageTemplate title="Stock Monitoring" subtitle="EU markets service and quick access">
      <div className="stock-monitoring-grid">
        <div className="stock-monitoring-card">
          <h3>Stock UI</h3>
          <p>Open the deployed stock monitoring frontend for dashboards and watchlists.</p>
          <div className="stock-monitoring-actions">
            <a href="/stock">Open Stock UI</a>
          </div>
        </div>

        <div className="stock-monitoring-card">
          <h3>Service Health</h3>
          <p>Quick health endpoint for runtime checks.</p>
          <div className="stock-monitoring-actions">
            <a href="/api/v1/stock-monitoring/health" className="secondary">
              Open Health JSON
            </a>
          </div>
        </div>

        <div className="stock-monitoring-card">
          <h3>Subpages</h3>
          <p>Use the internal subpages for status diagnostics and endpoint reference.</p>
          <div className="stock-monitoring-actions">
            <a href="/stock-monitoring/status">Status</a>
            <a href="/stock-monitoring/endpoints" className="secondary">Endpoints</a>
          </div>
        </div>
      </div>
    </PageTemplate>
  );
}

export default StockMonitoringHome;
