import React from 'react';
import { useKeycloak } from './KeycloakProvider';
import keycloak from './keycloak';
import UserDropdown from './UserDropdown';
import './Dashboard.css';

function Dashboard() {
  const { authenticated, userInfo, hasRole, authenticatedFetch } = useKeycloak();
  const [isAdminFromAPI, setIsAdminFromAPI] = React.useState(false);
  const [adminCheckLoading, setAdminCheckLoading] = React.useState(true);
  const userName = userInfo?.username || userInfo?.name || 'User';

  // Check admin status from backend API (more reliable than token parsing)
  React.useEffect(() => {
    console.log('Dashboard useEffect - authenticated:', authenticated, 'keycloak:', !!keycloak);
    
    if (!authenticated) {
      console.log('Dashboard - User not authenticated, skipping admin check');
      setIsAdminFromAPI(false);
      setAdminCheckLoading(false);
      return;
    }

    // Check admin status via backend API
    authenticatedFetch('/api/admin/check')
      .then(res => {
        if (!res.ok) {
          throw new Error(`API returned ${res.status}`);
        }
        return res.json();
      })
      .then(data => {
        setIsAdminFromAPI(!!data?.isAdmin);
        setAdminCheckLoading(false);
      })
      .catch(err => {
        console.error('Failed to check admin status via API:', err);
        setIsAdminFromAPI(false);
        setAdminCheckLoading(false);
      });
  }, [authenticated, authenticatedFetch]);

  // Also check token directly as fallback
  const roles = userInfo?.roles || keycloak?.tokenParsed?.realm_access?.roles || [];
  const hasRoleResult = hasRole && typeof hasRole === 'function' ? hasRole('admin') : false;
  const hasRoleInToken = roles.some(role => {
    const roleLower = role.toLowerCase();
    return roleLower === 'admin' || roleLower === 'realm-admin';
  });
  
  // Use backend API result, fallback to token check if API fails or is loading
  const finalIsAdmin = isAdminFromAPI || (authenticated && !adminCheckLoading && (hasRoleResult || hasRoleInToken));
  
  // Debug logging (ALWAYS log for troubleshooting)
  React.useEffect(() => {
    if (authenticated) {
      console.log('=== DASHBOARD ADMIN ROLE DEBUG ===');
      console.log('Dashboard - authenticated:', authenticated);
      console.log('Dashboard - userInfo:', userInfo);
      console.log('Dashboard - Backend API isAdmin:', isAdminFromAPI);
      console.log('Dashboard - API check loading:', adminCheckLoading);
      console.log('Dashboard - Token roles:', roles);
      console.log('Dashboard - hasRole("admin"):', hasRoleResult);
      console.log('Dashboard - roles check (admin/realm-admin):', hasRoleInToken);
      console.log('Dashboard - keycloak instance:', !!keycloak);
      console.log('Dashboard - keycloak.authenticated:', keycloak?.authenticated);
      console.log('Dashboard - realm_access:', keycloak?.tokenParsed?.realm_access);
      console.log('Dashboard - Final isAdmin (API + fallback):', finalIsAdmin);
      console.log('Dashboard - adminServices will be:', finalIsAdmin ? 'SHOWN' : 'HIDDEN');
      if (keycloak?.tokenParsed) {
        console.log('Dashboard - Full tokenParsed:', JSON.stringify(keycloak.tokenParsed, null, 2));
      }
      console.log('===================================');
    }
  }, [authenticated, isAdminFromAPI, adminCheckLoading, roles, hasRoleResult, hasRoleInToken, finalIsAdmin, keycloak, userInfo]);

  // Main analytics and visualizations - shown prominently on main page
  const mainAnalytics = [
    {
      name: 'EU Energy Data',
      description: 'Explore EU energy statistics and analytics',
      icon: '⚡',
      url: '/energy',
      status: 'active',
      gradient: 'linear-gradient(135deg, #f6d365 0%, #fda085 100%)',
      category: 'Analytics'
    },
    {
      name: 'Electricity Timeseries',
      description: 'High-frequency electricity load and generation data (ENTSO-E)',
      icon: '🔌',
      url: '/electricity',
      status: 'active',
      gradient: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
      category: 'Analytics'
    },
    {
      name: 'Geospatial Features',
      description: 'OpenStreetMap features aggregated to NUTS regions',
      icon: '🗺️',
      url: '/geo',
      status: 'active',
      gradient: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)',
      category: 'Analytics'
    },
    {
      name: 'Monitoring & Dashboards',
      description: 'System monitoring, data quality, and Grafana dashboards',
      icon: '📊',
      url: '/monitoring',
      status: 'active',
      gradient: 'linear-gradient(135deg, #43e97b 0%, #38f9d7 100%)',
      category: 'Monitoring'
    }
  ];

  // All services combined for stats (only main analytics on main page)
  const allServices = [...mainAnalytics];

  // Recent activity - show different activities based on role
  const recentActivity = [
    { action: 'Energy data accessed', service: 'Energy Service', time: '2 hours ago', status: 'success' },
    { action: 'Profile viewed', service: 'Profile Service', time: '5 hours ago', status: 'info' },
    ...(finalIsAdmin ? [
      { action: 'Workflow executed', service: 'Airflow', time: '1 day ago', status: 'success' }
    ] : [])
  ];

  return (
    <div className="dashboard">
      {/* Header */}
      <header className="dashboard-header">
        <div className="dashboard-header-content">
          <div className="logo-section">
            <div className="logo-icon-dashboard">LW</div>
            <span className="logo-text">Lianel World</span>
          </div>
          <div className="header-right">
            <UserDropdown />
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="dashboard-main">
        <div className="dashboard-container">
          {/* Welcome Section */}
          <div className="welcome-section">
            <h1 className="welcome-title">Welcome back, {userName}! 👋</h1>
            <p className="welcome-subtitle">Here's what's happening with your services</p>
          </div>

          {/* Main Analytics & Visualizations */}
          <section className="services-section">
            <h2 className="section-title">Analytics & Data Visualizations</h2>
            <p className="section-subtitle">Explore energy data, timeseries, geospatial features, and monitoring dashboards</p>
            <div className="services-grid">
              {mainAnalytics.map((service, index) => (
                <a 
                  key={index} 
                  href={service.url} 
                  className="service-card"
                  target={service.url.startsWith('http') ? '_self' : '_self'}
                  rel={service.url.startsWith('http') ? 'noopener noreferrer' : ''}
                >
                  <div className="service-header" style={{ background: service.gradient }}>
                    <div className="service-icon">{service.icon}</div>
                    <div className={`service-status status-${service.status}`}>
                      <span className="status-dot"></span>
                      {service.status}
                    </div>
                  </div>
                  <div className="service-content">
                    <h3 className="service-name">{service.name}</h3>
                    <p className="service-description">{service.description}</p>
                    <div className="service-action">
                      Open Service
                      <span className="arrow">→</span>
                    </div>
                  </div>
                </a>
              ))}
            </div>
          </section>

          {/* Link to Services Page */}
          <section className="services-section">
            <div className="services-link-card">
              <h2 className="section-title">Other Services</h2>
              <p className="section-subtitle">Access user profile, admin tools, and API documentation</p>
              <a href="/services" className="btn-view-services">
                View All Services →
              </a>
            </div>
          </section>

          {/* Recent Activity */}
          <section className="activity-section">
            <h2 className="section-title">Recent Activity</h2>
            <div className="activity-list">
              {recentActivity.map((activity, index) => (
                <div key={index} className="activity-item">
                  <div className={`activity-icon activity-${activity.status}`}>
                    {activity.status === 'success' ? '✓' : 'ℹ'}
                  </div>
                  <div className="activity-content">
                    <div className="activity-action">{activity.action}</div>
                    <div className="activity-meta">
                      <span className="activity-service">{activity.service}</span>
                      <span className="activity-time">{activity.time}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Quick Stats */}
          <section className="stats-section">
            <div className="stats-grid">
              <div className="stat-card">
                <div className="stat-icon">🚀</div>
                <div className="stat-value">{allServices.length}</div>
                <div className="stat-label">Active Services</div>
              </div>
              <div className="stat-card">
                <div className="stat-icon">✓</div>
                <div className="stat-value">99.9%</div>
                <div className="stat-label">Uptime</div>
              </div>
              <div className="stat-card">
                <div className="stat-icon">⚡</div>
                <div className="stat-value">&lt;50ms</div>
                <div className="stat-label">Response Time</div>
              </div>
            </div>
          </section>
        </div>
      </main>

      {/* Footer */}
      <footer className="dashboard-footer">
        <div className="dashboard-container">
          <p>&copy; 2025 Lianel World. All rights reserved.</p>
        </div>
      </footer>
    </div>
  );
}

export default Dashboard;
