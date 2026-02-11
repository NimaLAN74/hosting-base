import React, { useEffect, useState } from 'react';
import PageTemplate from '../PageTemplate';
import { useKeycloak } from '../KeycloakProvider';
import './StockMonitoring.css';

function StockMonitoringStatus() {
  const { authenticatedFetch } = useKeycloak();
  const [health, setHealth] = useState(null);
  const [status, setStatus] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setError(null);
      try {
        const [healthRes, statusRes] = await Promise.all([
          fetch('/api/v1/stock-monitoring/health'),
          authenticatedFetch('/api/v1/stock-monitoring/status'),
        ]);

        const healthJson = await healthRes.json().catch(() => null);
        const statusJson = await statusRes.json().catch(() => null);

        if (!cancelled) {
          setHealth(healthJson);
          setStatus(statusJson);
        }
      } catch (e) {
        if (!cancelled) {
          setError(e.message || 'Failed to fetch stock monitoring status.');
        }
      }
    }

    load();
    return () => {
      cancelled = true;
    };
  }, [authenticatedFetch]);

  return (
    <PageTemplate title="Stock Monitoring Status" subtitle="Live service health and status output">
      <div className="stock-monitoring-status-box">
        <h3>Health</h3>
        <pre>{JSON.stringify(health, null, 2)}</pre>
      </div>

      <div className="stock-monitoring-status-box">
        <h3>Status</h3>
        <pre>{JSON.stringify(status, null, 2)}</pre>
      </div>

      {error && <div className="stock-monitoring-error">{error}</div>}
    </PageTemplate>
  );
}

export default StockMonitoringStatus;
