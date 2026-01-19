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
      gradient: 'linear-gradient(135deg, #4facfe 0%, #00f2fe 100%)',
      category: 'Account'
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
      gradient: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
      category: 'Administration'
    },
    {
      name: 'Grafana Monitoring',
      description: 'Real-time system monitoring and analytics',
      icon: '📊',
      url: 'https://monitoring.lianel.se',
      status: 'active',
      gradient: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)',
      category: 'Administration'
    },
    {
      name: 'Profile Management',
      description: 'Admin tools to manage users',
      icon: '🧑‍💼',
      url: '/admin/users',
      status: 'active',
      gradient: 'linear-gradient(135deg, #fa709a 0%, #fee140 100%)',
      category: 'Administration'
    },
    {
      name: 'Profile Service API',
      description: 'User profile management service (Admin only)',
      icon: '🔧',
      url: '/swagger-ui',
      status: 'active',
      gradient: 'linear-gradient(135deg, #fa709a 0%, #fee140 100%)',
      category: 'Administration'
    },
    {
      name: 'Energy Service API',
      description: 'Energy data service API documentation (Admin only)',
      icon: '🔧',
      url: '/api/energy/swagger-ui',
      status: 'active',
      gradient: 'linear-gradient(135deg, #f6d365 0%, #fda085 100%)',
      category: 'Administration'
    },
    {
      name: 'Admin Console',
      description: 'Manage users and access',
      icon: '🛡️',
      url: '/admin/users',
      status: 'active',
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
            {allServices.map((service, index) => (
              <a 
                key={index} 
                href={service.url} 
                className="service-card"
                target={service.url.startsWith('http') ? '_blank' : '_self'}
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
