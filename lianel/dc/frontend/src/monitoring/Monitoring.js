import React, { useState, useEffect } from 'react';
import PageTemplate from '../PageTemplate';
import { useKeycloak } from '../KeycloakProvider';
import './Monitoring.css';

function Monitoring() {
  const { authenticated, login } = useKeycloak();
  const [selectedDashboard, setSelectedDashboard] = useState(null);

  // Get Grafana URL from environment variable
  const grafanaUrl = process.env.REACT_APP_GRAFANA_URL || 'https://monitoring.lianel.se';
  
  // Grafana dashboards configuration
  // UIDs match the dashboard JSON files in monitoring/grafana/provisioning/dashboards/
  const grafanaDashboards = [
    {
      id: 'system-health',
      name: 'System Health',
      description: 'Overall system health, CPU, memory, disk, and network metrics',
      uid: 'system-health',
      url: `${grafanaUrl}/d/system-health`,
      embedUrl: `${grafanaUrl}/d/system-health?kiosk=tv&orgId=1`,
      category: 'Infrastructure'
    },
    {
      id: 'pipeline-status',
      name: 'Pipeline Status',
      description: 'Airflow DAG execution status, success rates, and task performance',
      uid: 'pipeline-status',
      url: `${grafanaUrl}/d/pipeline-status`,
      embedUrl: `${grafanaUrl}/d/pipeline-status?kiosk=tv&orgId=1`,
      category: 'Data Pipelines'
    },
    {
      id: 'database-performance',
      name: 'Database Performance',
      description: 'PostgreSQL performance metrics, query times, and connection stats',
      uid: 'database-performance',
      url: `${grafanaUrl}/d/database-performance`,
      embedUrl: `${grafanaUrl}/d/database-performance?kiosk=tv&orgId=1`,
      category: 'Infrastructure'
    },
    {
      id: 'error-tracking',
      name: 'Error Tracking',
      description: 'System errors, OOM kills, HTTP errors, and active alerts',
      uid: 'error-tracking',
      url: `${grafanaUrl}/d/error-tracking`,
      embedUrl: `${grafanaUrl}/d/error-tracking?kiosk=tv&orgId=1`,
      category: 'Operations'
    },
    {
      id: 'sla-monitoring',
      name: 'SLA Monitoring',
      description: 'Service level agreements, API response times, and uptime metrics',
      uid: 'sla-monitoring',
      url: `${grafanaUrl}/d/sla-monitoring`,
      embedUrl: `${grafanaUrl}/d/sla-monitoring?kiosk=tv&orgId=1`,
      category: 'Operations'
    },
    {
      id: 'data-quality',
      name: 'Data Quality',
      description: 'Data freshness, completeness, and anomaly detection',
      uid: 'data-quality',
      url: `${grafanaUrl}/d/data-quality`,
      embedUrl: `${grafanaUrl}/d/data-quality?kiosk=tv&orgId=1`,
      category: 'Data Quality'
    },
    {
      id: 'energy-osm-features',
      name: 'Energy & OSM Features',
      description: 'Energy data vs OpenStreetMap features analysis',
      uid: 'energy-osm-features',
      url: `${grafanaUrl}/d/energy-osm-features`,
      embedUrl: `${grafanaUrl}/d/energy-osm-features?kiosk=tv&orgId=1`,
      category: 'Analytics'
    },
    {
      id: 'web-analytics',
      name: 'Web Analytics',
      description: 'User access logs, unique visitors, and request statistics',
      uid: 'web-analytics',
      url: `${grafanaUrl}/d/web-analytics`,
      embedUrl: `${grafanaUrl}/d/web-analytics?kiosk=tv&orgId=1`,
      category: 'Analytics'
    },
    {
      id: 'energy-metrics',
      name: 'Energy Metrics',
      description: 'Energy consumption and production metrics',
      uid: 'energy-metrics',
      url: `${grafanaUrl}/d/energy-metrics`,
      embedUrl: `${grafanaUrl}/d/energy-metrics?kiosk=tv&orgId=1`,
      category: 'Analytics'
    }
  ];

  const categories = [...new Set(grafanaDashboards.map(d => d.category))];

  const handleDashboardClick = (dashboard) => {
    setSelectedDashboard(dashboard);
  };

  const handleOpenInGrafana = (url) => {
    window.open(url, '_blank');
  };

  // Show login message if not authenticated (same pattern as /electricity and /geo)
  if (!authenticated) {
    return (
      <div className="monitoring-container" style={{ padding: '40px', textAlign: 'center' }}>
        <p style={{ fontSize: '18px', marginBottom: '20px' }}>Please log in to view monitoring dashboards.</p>
        <button
          onClick={() => login(true)}
          style={{
            padding: '12px 24px',
            background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
            color: 'white',
            border: 'none',
            borderRadius: '8px',
            fontSize: '16px',
            fontWeight: '600',
            cursor: 'pointer'
          }}
        >
          Log In
        </button>
      </div>
    );
  }

  return (
    <PageTemplate title="Monitoring & Dashboards">
      <div className="monitoring-container" style={{ minHeight: '400px' }}>
        <div className="monitoring-header">
          <p className="monitoring-description">
            Access Grafana dashboards for system monitoring, data quality, and analytics.
            Click on a dashboard to view it embedded, or open it in Grafana for full functionality.
          </p>
        </div>

        {/* Dashboard Categories */}
        {categories.map(category => (
          <div key={category} className="dashboard-category">
            <h2 className="category-title">{category}</h2>
            <div className="dashboards-grid">
              {grafanaDashboards
                .filter(d => d.category === category)
                .map(dashboard => (
                  <div
                    key={dashboard.id}
                    className="dashboard-card"
                    onClick={() => handleDashboardClick(dashboard)}
                  >
                    <div className="dashboard-card-header">
                      <h3 className="dashboard-name">{dashboard.name}</h3>
                      <span className="dashboard-badge">{dashboard.category}</span>
                    </div>
                    <p className="dashboard-description">{dashboard.description}</p>
                    <div className="dashboard-actions">
                      <button
                        className="btn-view"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDashboardClick(dashboard);
                        }}
                      >
                        View Embedded
                      </button>
                      <button
                        className="btn-open"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleOpenInGrafana(dashboard.url);
                        }}
                      >
                        Open in Grafana
                      </button>
                    </div>
                  </div>
                ))}
            </div>
          </div>
        ))}

        {/* Embedded Dashboard Modal/View */}
        {selectedDashboard && (
          <div className="dashboard-modal" onClick={() => setSelectedDashboard(null)}>
            <div className="dashboard-modal-content" onClick={(e) => e.stopPropagation()}>
              <div className="dashboard-modal-header">
                <h2>{selectedDashboard.name}</h2>
                <div className="dashboard-modal-actions">
                  <button
                    className="btn-open-full"
                    onClick={() => handleOpenInGrafana(selectedDashboard.url)}
                  >
                    Open Full Dashboard
                  </button>
                  <button
                    className="btn-close"
                    onClick={() => setSelectedDashboard(null)}
                  >
                    Close
                  </button>
                </div>
              </div>
              <div className="dashboard-embed-container">
                <iframe
                  src={selectedDashboard.embedUrl}
                  title={selectedDashboard.name}
                  className="dashboard-iframe"
                  frameBorder="0"
                  allowFullScreen
                />
              </div>
            </div>
          </div>
        )}
      </div>
    </PageTemplate>
  );
}

export default Monitoring;
