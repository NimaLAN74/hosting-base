import React from 'react';
import { Link } from 'react-router-dom';
import UserDropdown from './UserDropdown';
import './App.css';

/**
 * Reusable Page Template Component
 * Provides consistent header and footer across all pages
 * 
 * @param {Object} props
 * @param {string} props.title - Page title
 * @param {ReactNode} props.children - Page content
 * @param {boolean} props.showBackButton - Show "Back to Home" button (default: true)
 * @param {string} props.backButtonText - Custom back button text (default: "← Back to Home")
 */
const PageTemplate = ({ 
  title, 
  children, 
  showBackButton = true, 
  backButtonText = "← Back to Home" 
}) => {
  return (
    <div className="App">
      <div className="container">
        <header className="header">
          <Link to="/" className="logo">
            <div className="logo-icon">LW</div>
            Lianel World
          </Link>
          <div className="header-right">
            <UserDropdown />
          </div>
        </header>
        
        <main className="main">
          <div className="page-header">
            {showBackButton && (
              <Link to="/" className="back-to-home-btn">
                {backButtonText}
              </Link>
            )}
            {title && <h1 className="page-title">{title}</h1>}
          </div>
          
          <div className="page-content">
            {children}
          </div>
        </main>
        
        <footer className="footer">
          <p>&copy; 2025 Lianel World. All rights reserved.</p>
        </footer>
      </div>
    </div>
  );
};

export default PageTemplate;
