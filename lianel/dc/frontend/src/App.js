import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Profile from './Profile';
import LandingPage from './LandingPage';
import Dashboard from './Dashboard';
import { useKeycloak } from './KeycloakProvider';
import './App.css';

function Home() {
  const { authenticated } = useKeycloak();

  // If authenticated, show the dashboard
  if (authenticated) {
    return <Dashboard />;
  }

  // If not authenticated, show the landing page
  return <LandingPage />;
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

