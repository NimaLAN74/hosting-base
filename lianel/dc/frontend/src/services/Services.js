import React from 'react';
import PageTemplate from '../PageTemplate';
import { useKeycloak } from '../KeycloakProvider';
import keycloak from '../keycloak';
import './Services.css';

function Services() {
  const { authenticated, userInfo, hasRole, authenticatedFetch } = useKeycloak();
  const [isAdminFromAPI, setIsAdminFromAPI] = React.useState(false);
  const [adminCheckLoading, setAdminCheckLoading] = React.useState(true);

  // Check admin status from backend API
  React.useEffect(() => {
    if (!authenticated) {
      setIsAdminFromAPI(false);
      setAdminCheckLoading(false);
      return;
    }

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
  
  const finalIsAdmin = isAdminFromAPI || (authenticated && !adminCheckLoading && (hasRoleResult || hasRoleInToken));

  // Base services - moved from main page
  const baseServices = [
    {
      name: 'User Profile',
      description: 'View and edit your user profile',
      icon: '👤',
      url: '/profile',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #4facfe 0%, #00f2fe 100%)',
      category: 'Account'
    },
    {
      name: 'Stock Monitoring',
      description: 'Entry page for stock monitoring service and diagnostics',
      icon: '📈',
      url: '/stock-monitoring',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #0ea5e9 0%, #2563eb 100%)',
      category: 'Analytics'
    },
    {
      name: 'Stock Monitoring Status',
      description: 'Service status and health output for stock-monitoring backend',
      icon: '🩺',
      url: '/stock-monitoring/status',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #14b8a6 0%, #0ea5e9 100%)',
      category: 'Analytics'
    },
    {
      name: 'Stock Monitoring Endpoints',
      description: 'Quick test links and endpoint reference',
      icon: '🧪',
      url: '/stock-monitoring/endpoints',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
      category: 'Analytics'
    }
  ];

  // Admin-only services
  const adminServices = finalIsAdmin ? [
    {
      name: 'Apache Airflow',
      description: 'Workflow orchestration and management',
      icon: '⚙️',
      url: 'https://airflow.lianel.se',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
      category: 'Administration'
    },
    {
      name: 'Grafana Monitoring',
      description: 'Real-time system monitoring and analytics',
      icon: '📊',
      url: 'https://monitoring.lianel.se',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)',
      category: 'Administration'
    },
    {
      name: 'Profile Management',
      description: 'Admin tools to manage users',
      icon: '🧑‍💼',
      url: '/admin/users',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #fa709a 0%, #fee140 100%)',
      category: 'Administration'
    },
    {
      name: 'Profile Service API',
      description: 'User profile management service (Admin only)',
      icon: '🔧',
      url: '/swagger-ui',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #fa709a 0%, #fee140 100%)',
      category: 'Administration'
    },
    {
      name: 'Energy Service API',
      description: 'Energy data service API documentation (currently paused)',
      icon: '🔧',
      url: '/api/energy/swagger-ui',
      status: 'paused',
      enabled: false,
      gradient: 'linear-gradient(135deg, #f6d365 0%, #fda085 100%)',
      category: 'Administration'
    },
    {
      name: 'Admin Console',
      description: 'Manage users and access',
      icon: '🛡️',
      url: '/admin/users',
      status: 'active',
      enabled: true,
      gradient: 'linear-gradient(135deg, #43e97b 0%, #38f9d7 100%)',
      category: 'Administration'
    }
  ] : [];

  const allServices = [...baseServices, ...adminServices];

  if (!authenticated) {
    return (
      <PageTemplate title="Services">
        <div className="services-container">
          <p>Please log in to view services.</p>
        </div>
      </PageTemplate>
    );
  }

  return (
    <PageTemplate title="Services">
      <div className="services-container">
        <div className="services-header">
          <p className="services-description">
            Access all available services and tools. Administrative services are only visible to admin users.
          </p>
        </div>

        {allServices.length > 0 ? (
          <div className="services-grid">
            {allServices.map((service, index) => {
              const isEnabled = service.enabled !== false;
              const className = `service-card ${isEnabled ? '' : 'service-card-disabled'}`.trim();
              const actionLabel = isEnabled ? 'Open Service' : 'Service Paused';

              const content = (
                <>
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
                      {actionLabel}
                      <span className="arrow">→</span>
                    </div>
                  </div>
                </>
              );

              if (!isEnabled) {
                return (
                  <div key={index} className={className} aria-disabled="true">
                    {content}
                  </div>
                );
              }

              return (
                <a
                  key={index}
                  href={service.url}
                  className={className}
                  target={service.url.startsWith('http') ? '_blank' : '_self'}
                  rel={service.url.startsWith('http') ? 'noopener noreferrer' : ''}
                >
                  {content}
                </a>
              );
            })}
          </div>
        ) : (
          <div className="no-services">
            <p>No services available at this time.</p>
          </div>
        )}
      </div>
    </PageTemplate>
  );
}

export default Services;
