import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Profile from './Profile';
import LandingPage from './LandingPage';
import Dashboard from './Dashboard';
import Energy from './energy/Energy';
import { useKeycloak } from './KeycloakProvider';
import UsersList from './admin/UsersList';
import UserDetails from './admin/UserDetails';
import UserForm from './admin/UserForm';
import AdminGuard from './admin/AdminGuard';
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
            <Route path="/energy" element={<Energy />} />
            <Route
              path="/admin/users"
              element={
                <AdminGuard>
                  <UsersList />
                </AdminGuard>
              }
            />
            <Route
              path="/admin/users/new"
              element={
                <AdminGuard>
                  <UserForm mode="create" />
                </AdminGuard>
              }
            />
            <Route
              path="/admin/users/:id"
              element={
                <AdminGuard>
                  <UserDetails />
                </AdminGuard>
              }
            />
            <Route path="/" element={<Home />} />
          </Routes>
        </div>
      </div>
    </Router>
  );
}

export default App;

