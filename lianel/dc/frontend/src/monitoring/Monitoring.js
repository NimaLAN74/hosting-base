import React from 'react';
import PageTemplate from '../PageTemplate';
import { useKeycloak } from '../KeycloakProvider';
import './Monitoring.css';

function Monitoring() {
  const { authenticated, login, keycloakReady } = useKeycloak();

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
      category: 'Infrastructure'
    },
    {
      id: 'pipeline-status',
      name: 'Pipeline Status',
      description: 'Airflow DAG execution status, success rates, and task performance',
      uid: 'pipeline-status',
      url: `${grafanaUrl}/d/pipeline-status`,
      category: 'Data Pipelines'
    },
    {
      id: 'database-performance',
      name: 'Database Performance',
      description: 'PostgreSQL performance metrics, query times, and connection stats',
      uid: 'database-performance',
      url: `${grafanaUrl}/d/database-performance`,
      category: 'Infrastructure'
    },
    {
      id: 'error-tracking',
      name: 'Error Tracking',
      description: 'System errors, OOM kills, HTTP errors, and active alerts',
      uid: 'error-tracking',
      url: `${grafanaUrl}/d/error-tracking`,
      category: 'Operations'
    },
    {
      id: 'sla-monitoring',
      name: 'SLA Monitoring',
      description: 'Service level agreements, API response times, and uptime metrics',
      uid: 'sla-monitoring',
      url: `${grafanaUrl}/d/sla-monitoring`,
      category: 'Operations'
    },
    {
      id: 'data-quality',
      name: 'Data Quality',
      description: 'Data freshness, completeness, and anomaly detection',
      uid: 'data-quality',
      url: `${grafanaUrl}/d/data-quality`,
      category: 'Data Quality'
    },
    {
      id: 'energy-osm-features',
      name: 'Energy & OSM Features',
      description: 'Energy data vs OpenStreetMap features analysis',
      uid: 'energy-osm-features',
      url: `${grafanaUrl}/d/energy-osm-features`,
      category: 'Analytics'
    },
    {
      id: 'web-analytics',
      name: 'Web Analytics',
      description: 'User access logs, unique visitors, and request statistics',
      uid: 'web-analytics',
      url: `${grafanaUrl}/d/web-analytics`,
      category: 'Analytics'
    },
    {
      id: 'energy-metrics',
      name: 'Energy Metrics',
      description: 'Energy consumption and production metrics',
      uid: 'energy-metrics',
      url: `${grafanaUrl}/d/energy-metrics`,
      category: 'Analytics'
    }
  ];

  const categories = [...new Set(grafanaDashboards.map(d => d.category))];

  const handleOpenInGrafana = (url) => {
    window.open(url, '_blank');
  };

  // Wait for Keycloak to be ready before checking authentication
  // This prevents redirect loops when Keycloak is still initializing
  if (!keycloakReady) {
    return (
      <div className="monitoring-container" style={{ padding: '40px', textAlign: 'center' }}>
        <p style={{ fontSize: '18px', marginBottom: '20px' }}>Loading authentication...</p>
      </div>
    );
  }

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
    <PageTemplate
      title="Monitoring & Dashboards"
      subtitle="System monitoring, dashboards, and operational visibility"
    >
      <div className="monitoring-container" style={{ minHeight: '400px' }}>
        <div className="monitoring-header">
          <p className="monitoring-description">
            Access Grafana dashboards for system monitoring, data quality, and analytics.
            Click on a dashboard card or button to open it in a new tab.
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
                    onClick={() => handleOpenInGrafana(dashboard.url)}
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
                          handleOpenInGrafana(dashboard.url);
                        }}
                      >
                        View in New Tab
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

      </div>
    </PageTemplate>
  );
}

export default Monitoring;
