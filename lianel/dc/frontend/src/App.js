import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Profile from './Profile';
import LandingPage from './LandingPage';
import Dashboard from './Dashboard';
import Energy from './energy/Energy';
import ElectricityTimeseries from './electricity/ElectricityTimeseries';
import GeoFeatures from './geo/GeoFeatures';
import GeoEnrichmentMap from './geo/GeoEnrichmentMap';
import Monitoring from './monitoring/Monitoring';
import Services from './services/Services';
import CompAI from './comp-ai/CompAI';
import CompAIHistory from './comp-ai/CompAIHistory';
import CompAIMonitoring from './comp-ai/CompAIMonitoring';
import CompAIControls from './comp-ai/CompAIControls';
import CompAIAuditDocs from './comp-ai/CompAIAuditDocs';
import CompAIScanDocuments from './comp-ai/CompAIScanDocuments';
import StockMonitoringHome from './stock-monitoring/StockMonitoringHome';
import StockMonitoringStatus from './stock-monitoring/StockMonitoringStatus';
import StockMonitoringEndpoints from './stock-monitoring/StockMonitoringEndpoints';
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
            <Route path="/electricity" element={<ElectricityTimeseries />} />
            <Route path="/geo" element={<GeoFeatures />} />
            <Route path="/geo/map" element={<GeoEnrichmentMap />} />
            <Route path="/monitoring" element={<Monitoring />} />
            <Route path="/services" element={<Services />} />
            <Route path="/comp-ai" element={<CompAI />} />
            <Route path="/comp-ai/controls" element={<CompAIControls />} />
            <Route path="/comp-ai/history" element={<CompAIHistory />} />
            <Route path="/comp-ai/monitoring" element={<CompAIMonitoring />} />
            <Route path="/comp-ai/audit-docs" element={<CompAIAuditDocs />} />
            <Route path="/comp-ai/scan" element={<CompAIScanDocuments />} />
            <Route path="/stock-monitoring" element={<StockMonitoringHome />} />
            <Route path="/stock-monitoring/status" element={<StockMonitoringStatus />} />
            <Route path="/stock-monitoring/endpoints" element={<StockMonitoringEndpoints />} />
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

