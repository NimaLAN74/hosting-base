import React, { useState, useEffect } from 'react';
import { compAiApi } from './compAiApi';
import PageTemplate from '../PageTemplate';
import { formatDateTimeEU } from '../services/dateFormat';
import './CompAI.css';

function CompAIMonitoring() {
  const [health, setHealth] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [lastChecked, setLastChecked] = useState(null);

  useEffect(() => {
    checkHealth();
    // Auto-refresh every 30 seconds
    const interval = setInterval(checkHealth, 30000);
    return () => clearInterval(interval);
  }, []);

  const checkHealth = async () => {
    try {
      const result = await compAiApi.getHealth();
      setHealth(result);
      setError(null);
      setLastChecked(new Date());
    } catch (err) {
      setError(err.message || 'Health check failed');
      setHealth(null);
      setLastChecked(new Date());
    } finally {
      setLoading(false);
    }
  };

  return (
    <PageTemplate title="Comp AI Monitoring" subtitle="Service status and health monitoring">
      <div className="comp-ai-container">
        <div className="comp-ai-monitoring">
          <div className="comp-ai-monitoring-header">
            <h2>Service Health Status</h2>
            <button
              onClick={checkHealth}
              disabled={loading}
              className="comp-ai-refresh-btn"
            >
              {loading ? 'Checking...' : 'Refresh'}
            </button>
          </div>

          {lastChecked && (
            <div className="comp-ai-last-checked">
              Last checked: {formatDateTimeEU(lastChecked)}
            </div>
          )}

          {loading && !health && (
            <div className="comp-ai-loading">Checking service health...</div>
          )}

          {error && (
            <div className="comp-ai-error">
              <strong>Error:</strong> {error}
            </div>
          )}

          {health && (
            <div className="comp-ai-health-cards">
              <div className={`comp-ai-health-card ${health.status === 'healthy' ? 'healthy' : 'unhealthy'}`}>
                <div className="comp-ai-health-card-header">
                  <h3>Service Status</h3>
                  <span className={`comp-ai-status-badge ${health.status === 'healthy' ? 'healthy' : 'unhealthy'}`}>
                    {health.status || 'Unknown'}
                  </span>
                </div>
                <div className="comp-ai-health-card-content">
                  <p><strong>Service:</strong> {health.service || 'comp-ai-service'}</p>
                  {health.version && <p><strong>Version:</strong> {health.version}</p>}
                </div>
              </div>

              <div className="comp-ai-health-card">
                <div className="comp-ai-health-card-header">
                  <h3>System Information</h3>
                </div>
                <div className="comp-ai-health-card-content">
                  <p><strong>Status:</strong> {health.status === 'healthy' ? '✅ Operational' : '⚠️ Issues Detected'}</p>
                  {health.uptime && <p><strong>Uptime:</strong> {health.uptime}</p>}
                  {health.timestamp && (
                    <p><strong>Last Update:</strong> {formatDateTimeEU(health.timestamp)}</p>
                  )}
                </div>
              </div>
            </div>
          )}

          <div className="comp-ai-monitoring-info">
            <h3>Monitoring Information</h3>
            <p>
              This page displays the real-time health status of the Comp AI service.
              The service is automatically checked every 30 seconds.
            </p>
            <ul>
              <li><strong>Healthy:</strong> Service is operational and responding to requests</li>
              <li><strong>Unhealthy:</strong> Service may be experiencing issues</li>
              <li>Check the request history for recent activity</li>
              <li>Contact support if issues persist</li>
            </ul>
          </div>
        </div>
      </div>
    </PageTemplate>
  );
}

export default CompAIMonitoring;
