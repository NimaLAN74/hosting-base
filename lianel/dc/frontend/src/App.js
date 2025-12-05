import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import UserDropdown from './UserDropdown';
import Profile from './Profile';
import './App.css';

function Home() {
  return (
    <>
      <header className="header">
        <h1 className="logo">
          <div className="logo-icon">LW</div>
          Lianel World
        </h1>
        <div className="header-right">
          <UserDropdown />
        </div>
      </header>
        
      <main className="main">
        <div className="hero">
          <h2 className="title">Welcome to Lianel World</h2>
          <p className="subtitle">Building something amazing</p>
        </div>
        
        <div className="features">
          <div className="feature-card">
            <div className="icon">🚀</div>
            <h3>Fast</h3>
            <p>Lightning-fast performance</p>
          </div>
          
          <div className="feature-card">
            <div className="icon">🎨</div>
            <h3>Beautiful</h3>
            <p>Clean and modern design</p>
          </div>
          
          <div className="feature-card">
            <div className="icon">🔒</div>
            <h3>Secure</h3>
            <p>Built with security in mind</p>
          </div>
        </div>

        <div className="services">
          <a href="https://airflow.lianel.se" className="service-link" target="_blank" rel="noopener noreferrer">
            <div className="service-card">
              <div className="icon">⚙️</div>
              <h3>Airflow</h3>
              <p>Workflow Management</p>
            </div>
          </a>

          <a href="/monitoring/" className="service-link" target="_blank" rel="noopener noreferrer">
            <div className="service-card">
              <div className="icon">📊</div>
              <h3>Monitoring</h3>
              <p>Grafana Dashboard</p>
            </div>
          </a>
        </div>
      </main>
      
      <footer className="footer">
        <p>&copy; 2025 Lianel World. All rights reserved.</p>
      </footer>
    </>
  );
}

function App() {
  return (
    <Router>
      <div className="App">
        <div className="container">
          <Routes>
            <Route path="/profile" element={<Profile />} />
            <Route path="/" element={<Home />} />
          </Routes>
        </div>
      </div>
    </Router>
  );
}

export default App;
