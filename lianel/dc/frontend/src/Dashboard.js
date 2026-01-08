import React from 'react';
import { useKeycloak } from './KeycloakProvider';
import UserDropdown from './UserDropdown';
import './Dashboard.css';

function Dashboard() {
  const { keycloak, hasRole } = useKeycloak();
  const userName = keycloak?.tokenParsed?.preferred_username || keycloak?.tokenParsed?.name || 'User';

  // Base services available to all users
  const baseServices = [
    {
      name: 'User Profile',
      description: 'View and edit your user profile',
      icon: '👤',
      url: '/profile',
      status: 'active',
      gradient: 'linear-gradient(135deg, #4facfe 0%, #00f2fe 100%)'
    },
    {
      name: 'EU Energy Data',
      description: 'Explore EU energy statistics and analytics',
      icon: '⚡',
      url: '/energy',
      status: 'active',
      gradient: 'linear-gradient(135deg, #f6d365 0%, #fda085 100%)'
    }
  ];

  // Admin-only services
  const adminServices = hasRole && hasRole('admin') ? [
    {
      name: 'Apache Airflow',
      description: 'Workflow orchestration and management',
      icon: '⚙️',
      url: 'https://airflow.lianel.se',
      status: 'active',
      gradient: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
    },
    {
      name: 'Grafana Monitoring',
      description: 'Real-time system monitoring and analytics',
      icon: '📊',
      url: 'https://monitoring.lianel.se',
      status: 'active',
      gradient: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)'
    },
    {
      name: 'Profile Management',
      description: 'Admin tools to manage users',
      icon: '🧑‍💼',
      url: '/admin/users',
      status: 'active',
      gradient: 'linear-gradient(135deg, #fa709a 0%, #fee140 100%)'
    },
    {
      name: 'Profile Service API',
      description: 'User profile management service (Admin only)',
      icon: '🔧',
      url: '/swagger-ui',
      status: 'active',
      gradient: 'linear-gradient(135deg, #fa709a 0%, #fee140 100%)'
    },
    {
      name: 'Energy Service API',
      description: 'Energy data service API documentation (Admin only)',
      icon: '🔧',
      url: '/api/energy/swagger-ui',
      status: 'active',
      gradient: 'linear-gradient(135deg, #f6d365 0%, #fda085 100%)'
    },
    {
      name: 'Admin Console',
      description: 'Manage users and access',
      icon: '🛡️',
      url: '/admin/users',
      status: 'active',
      gradient: 'linear-gradient(135deg, #43e97b 0%, #38f9d7 100%)'
    }
  ] : [];

  // Combine services: base services for everyone, admin services for admins
  const services = [...baseServices, ...adminServices];

  // Recent activity - show different activities based on role
  const recentActivity = [
    { action: 'Energy data accessed', service: 'Energy Service', time: '2 hours ago', status: 'success' },
    { action: 'Profile viewed', service: 'Profile Service', time: '5 hours ago', status: 'info' },
    ...(hasRole && hasRole('admin') ? [
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

          {/* Services Grid */}
          <section className="services-section">
            <h2 className="section-title">Your Services</h2>
            <div className="services-grid">
              {services.map((service, index) => (
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
                <div className="stat-value">{services.length}</div>
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
