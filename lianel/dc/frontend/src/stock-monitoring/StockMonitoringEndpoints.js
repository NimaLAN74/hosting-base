import React from 'react';
import PageTemplate from '../PageTemplate';
import './StockMonitoring.css';

function StockMonitoringEndpoints() {
  return (
    <PageTemplate title="Stock Monitoring Endpoints" subtitle="Useful URLs and test commands">
      <div className="stock-monitoring-card">
        <h3>Primary URLs</h3>
        <div className="stock-monitoring-actions">
          <a href="/stock">/stock</a>
          <a href="/api/v1/stock-monitoring/health" className="secondary">
            /api/v1/stock-monitoring/health
          </a>
          <a href="/api/v1/stock-monitoring/status" className="secondary">
            /api/v1/stock-monitoring/status
          </a>
        </div>
      </div>

      <div className="stock-monitoring-status-box">
        <h3>cURL quick checks</h3>
        <pre>{`curl -i https://www.lianel.se/stock
curl -i https://www.lianel.se/api/v1/stock-monitoring/health
curl -i -H "Authorization: Bearer <token>" https://www.lianel.se/api/v1/stock-monitoring/status`}</pre>
      </div>
    </PageTemplate>
  );
}

export default StockMonitoringEndpoints;
