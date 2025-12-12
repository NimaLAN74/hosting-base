import React, { useState, useEffect } from 'react';
import { useKeycloak } from './KeycloakProvider';
import './LandingPage.css';

function LandingPage() {
  const { login } = useKeycloak();
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      setScrolled(window.scrollY > 50);
    };
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const features = [
    {
      icon: '🚀',
      title: 'Rapid Deployment',
      description: 'Launch your SaaS solutions in days, not months. Built on proven infrastructure.'
    },
    {
      icon: '🔐',
      title: 'Enterprise Security',
      description: 'Bank-grade security with SSO, RBAC, and compliance-ready architecture.'
    },
    {
      icon: '📊',
      title: 'Real-time Analytics',
      description: 'Built-in monitoring and analytics dashboards for data-driven decisions.'
    },
    {
      icon: '🌍',
      title: 'Global Scale',
      description: 'Cloud-native architecture designed to scale from startup to enterprise.'
    },
    {
      icon: '🔧',
      title: 'Customizable',
      description: 'Flexible architecture allowing full customization to your business needs.'
    },
    {
      icon: '💡',
      title: 'Expert Support',
      description: 'Technical consulting and support from experienced platform engineers.'
    }
  ];

  const solutions = [
    {
      title: 'Custom SaaS Solutions',
      description: 'Tailored platforms built on our proven infrastructure stack.',
      status: 'Available',
      statusColor: '#10b981',
      features: ['Rapid development', 'Your domain expertise', 'Enterprise-ready', 'Scalable architecture'],
      gradient: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
    },
    {
      title: 'Technical Consulting',
      description: 'Expert guidance for architecture, technology selection, and implementation.',
      status: 'Available',
      statusColor: '#10b981',
      features: ['Architecture design', 'Technology selection', 'Best practices', 'Performance optimization'],
      gradient: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)'
    }
  ];

  const techStack = [
    { name: 'Modern Frontend', category: 'User Interface' },
    { name: 'High-Performance Backend', category: 'Services' },
    { name: 'Container Orchestration', category: 'Infrastructure' },
    { name: 'Cloud-Native', category: 'Architecture' },
    { name: 'Scalable Databases', category: 'Data' },
    { name: 'Workflow Automation', category: 'Operations' },
    { name: 'Enterprise Security', category: 'Authentication' },
    { name: 'Real-time Monitoring', category: 'Observability' }
  ];

  return (
    <div className="landing-page">
      {/* Navigation */}
      <nav className={`navbar ${scrolled ? 'scrolled' : ''}`}>
        <div className="container">
          <div className="navbar-content">
            <div className="logo-section">
              <div className="logo-icon-landing">LW</div>
              <span className="logo-text">Lianel World</span>
            </div>
            <div className="nav-links">
              <a href="#solutions" className="nav-link">Solutions</a>
              <a href="#features" className="nav-link">Features</a>
              <a href="#technology" className="nav-link">Technology</a>
              <a href="#contact" className="nav-link">Contact</a>
              <button onClick={login} className="btn-primary">Sign In</button>
            </div>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="hero-section">
        <div className="hero-background"></div>
        <div className="container">
          <div className="hero-content">
            <div className="hero-badge">
              <span className="badge-dot"></span>
              Technical Platform & SaaS Consulting
            </div>
            <h1 className="hero-title">
              Build Your Next
              <span className="gradient-text"> SaaS Platform</span>
              <br />
              On Proven Infrastructure
            </h1>
            <p className="hero-subtitle">
              From consulting to fully managed SaaS solutions. We help businesses launch 
              and scale technical platforms with enterprise-grade architecture.
            </p>
            <div className="hero-actions">
              <button onClick={login} className="btn-hero-primary">
                Get Started
                <span className="arrow">→</span>
              </button>
              <a href="#solutions" className="btn-hero-secondary">
                Explore Solutions
              </a>
            </div>
            <div className="hero-stats">
              <div className="stat">
                <div className="stat-value">99.9%</div>
                <div className="stat-label">Uptime</div>
              </div>
              <div className="stat">
                <div className="stat-value">&lt;100ms</div>
                <div className="stat-label">Response Time</div>
              </div>
              <div className="stat">
                <div className="stat-value">24/7</div>
                <div className="stat-label">Support</div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Solutions Section */}
      <section id="solutions" className="solutions-section">
        <div className="container">
          <div className="section-header">
            <h2 className="section-title">Our Solutions</h2>
            <p className="section-subtitle">
              Proven platforms and custom solutions tailored to your business
            </p>
          </div>
          <div className="solutions-grid">
            {solutions.map((solution, index) => (
              <div key={index} className="solution-card">
                <div className="solution-header" style={{ background: solution.gradient }}>
                  <div className="solution-status" style={{ backgroundColor: solution.statusColor }}>
                    {solution.status}
                  </div>
                </div>
                <div className="solution-content">
                  <h3 className="solution-title">{solution.title}</h3>
                  <p className="solution-description">{solution.description}</p>
                  <ul className="solution-features">
                    {solution.features.map((feature, idx) => (
                      <li key={idx}>
                        <span className="check-icon">✓</span>
                        {feature}
                      </li>
                    ))}
                  </ul>
                  <button className="btn-solution" onClick={login}>
                    Learn More
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section id="features" className="features-section">
        <div className="container">
          <div className="section-header">
            <h2 className="section-title">Why Choose Lianel World</h2>
            <p className="section-subtitle">
              Enterprise-grade capabilities from day one
            </p>
          </div>
          <div className="features-grid">
            {features.map((feature, index) => (
              <div key={index} className="feature-card-modern">
                <div className="feature-icon">{feature.icon}</div>
                <h3 className="feature-title">{feature.title}</h3>
                <p className="feature-description">{feature.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Technology Section */}
      <section id="technology" className="tech-section">
        <div className="container">
          <div className="section-header">
            <h2 className="section-title">Built on Modern Technology</h2>
            <p className="section-subtitle">
              Leveraging the best tools for performance, security, and scalability
            </p>
          </div>
          <div className="tech-grid">
            {techStack.map((tech, index) => (
              <div key={index} className="tech-card">
                <div className="tech-name">{tech.name}</div>
                <div className="tech-category">{tech.category}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section id="contact" className="cta-section">
        <div className="container">
          <div className="cta-content">
            <h2 className="cta-title">Ready to Build Your Platform?</h2>
            <p className="cta-subtitle">
              Let's discuss how we can help you launch and scale your SaaS solution
            </p>
            <div className="cta-actions">
              <button onClick={login} className="btn-cta-primary">
                Start Now
              </button>
              <a href="mailto:contact@lianel.se" className="btn-cta-secondary">
                Contact Sales
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="footer-landing">
        <div className="container">
          <div className="footer-content">
            <div className="footer-section">
              <div className="footer-logo">
                <div className="logo-icon-landing">LW</div>
                <span className="logo-text">Lianel World</span>
              </div>
              <p className="footer-description">
                Technical platform and SaaS consulting for modern businesses
              </p>
            </div>
            <div className="footer-section">
              <h4 className="footer-heading">Solutions</h4>
              <ul className="footer-links">
                <li><a href="#solutions">Energy Intelligence</a></li>
                <li><a href="#solutions">Custom SaaS</a></li>
                <li><a href="#solutions">Consulting</a></li>
              </ul>
            </div>
            <div className="footer-section">
              <h4 className="footer-heading">Company</h4>
              <ul className="footer-links">
                <li><a href="#features">Features</a></li>
                <li><a href="#technology">Technology</a></li>
                <li><a href="#contact">Contact</a></li>
              </ul>
            </div>
            <div className="footer-section">
              <h4 className="footer-heading">Connect</h4>
              <ul className="footer-links">
                <li><a href="mailto:contact@lianel.se">Email</a></li>
                <li><a href="https://github.com/NimaLAN74">GitHub</a></li>
              </ul>
            </div>
          </div>
          <div className="footer-bottom">
            <p>&copy; 2025 Lianel World. All rights reserved.</p>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default LandingPage;
