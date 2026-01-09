import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import PageTemplate from '../PageTemplate';
import { energyApi } from './energyApi';
import { TimeSeriesChart, CountryComparisonChart, ProductDistributionChart, FlowDistributionChart } from './EnergyCharts';
import '../App.css';
import './Energy.css';

function Energy() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [serviceInfo, setServiceInfo] = useState(null);
  const [energyData, setEnergyData] = useState(null);
  const [filters, setFilters] = useState({
    country_code: '',
    year: '',
    limit: 50,
    offset: 0
  });
  const [summary, setSummary] = useState(null);

  useEffect(() => {
    fetchData();
  }, [filters.offset]);

  const fetchData = async () => {
    try {
      setLoading(true);
      setError('');

      // Fetch service info
      const info = await energyApi.getServiceInfo();
      setServiceInfo(info);

      // Fetch energy data
      const params = {
        limit: parseInt(filters.limit) || 50,
        offset: filters.offset || 0
      };
      if (filters.country_code) params.country_code = filters.country_code;
      if (filters.year) params.year = parseInt(filters.year);

      const data = await energyApi.getEnergyAnnual(params);
      setEnergyData(data);

      // Fetch summary
      const summaryData = await energyApi.getEnergySummary({
        country_code: filters.country_code || undefined,
        year: filters.year ? parseInt(filters.year) : undefined,
        group_by: 'country'
      });
      setSummary(summaryData);
    } catch (err) {
      console.error('Error fetching energy data:', err);
      setError(err.message || 'Failed to load energy data');
    } finally {
      setLoading(false);
    }
  };

  const handleFilterChange = (field, value) => {
    setFilters(prev => ({ ...prev, [field]: value, offset: 0 }));
  };

  const handleApplyFilters = () => {
    fetchData();
  };

  const handleResetFilters = () => {
    setFilters({ country_code: '', year: '', limit: 50, offset: 0 });
    setTimeout(fetchData, 100);
  };

  const formatNumber = (num) => {
    if (num === null || num === undefined) return 'N/A';
    return new Intl.NumberFormat('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(num);
  };

  // Calculate summary metrics
  const calculateMetrics = () => {
    if (!energyData || !energyData.data) return null;
    
    const data = energyData.data;
    const totalGWh = data.reduce((sum, record) => sum + (parseFloat(record.value_gwh) || 0), 0);
    const avgGWh = data.length > 0 ? totalGWh / data.length : 0;
    const countries = new Set(data.map(r => r.country_code)).size;
    const years = new Set(data.map(r => r.year)).size;
    
    return {
      totalGWh,
      avgGWh,
      countries,
      years,
      recordCount: data.length
    };
  };

  // Export to CSV
  const exportToCSV = () => {
    if (!energyData || !energyData.data || energyData.data.length === 0) {
      alert('No data to export');
      return;
    }

    const headers = ['Country Code', 'Country Name', 'Year', 'Product Code', 'Product Name', 'Flow Code', 'Flow Name', 'Value (GWh)', 'Unit', 'Source Table'];
    const rows = energyData.data.map(record => [
      record.country_code || '',
      record.country_name || '',
      record.year || '',
      record.product_code || '',
      record.product_name || '',
      record.flow_code || '',
      record.flow_name || '',
      record.value_gwh || '0',
      record.unit || '',
      record.source_table || ''
    ]);

    const csvContent = [
      headers.join(','),
      ...rows.map(row => row.map(cell => `"${String(cell).replace(/"/g, '""')}"`).join(','))
    ].join('\n');

    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    const url = URL.createObjectURL(blob);
    link.setAttribute('href', url);
    link.setAttribute('download', `energy-data-${new Date().toISOString().split('T')[0]}.csv`);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const metrics = calculateMetrics();

  if (loading && !energyData) {
    return (
      <PageTemplate title="EU Energy Data">
        <div className="loading">Loading energy data...</div>
      </PageTemplate>
    );
  }

  return (
    <PageTemplate title="EU Energy Data">
      <div className="energy-container">
        {/* Summary Cards */}
        {serviceInfo && (
          <div className="summary-cards">
            <div className="summary-card">
              <div className="card-icon">📊</div>
              <div className="card-content">
                <div className="card-label">Total Records</div>
                <div className="card-value">{serviceInfo.database.total_records.toLocaleString()}</div>
              </div>
            </div>
            <div className="summary-card">
              <div className="card-icon">🌍</div>
              <div className="card-content">
                <div className="card-label">Countries</div>
                <div className="card-value">{serviceInfo.database.countries}</div>
              </div>
            </div>
            <div className="summary-card">
              <div className="card-icon">📅</div>
              <div className="card-content">
                <div className="card-label">Years</div>
                <div className="card-value">{serviceInfo.database.years}</div>
              </div>
            </div>
            {metrics && (
              <>
                <div className="summary-card">
                  <div className="card-icon">⚡</div>
                  <div className="card-content">
                    <div className="card-label">Total Energy (GWh)</div>
                    <div className="card-value">{formatNumber(metrics.totalGWh)}</div>
                  </div>
                </div>
                <div className="summary-card">
                  <div className="card-icon">📈</div>
                  <div className="card-content">
                    <div className="card-label">Avg per Record</div>
                    <div className="card-value">{formatNumber(metrics.avgGWh)}</div>
                  </div>
                </div>
              </>
            )}
          </div>
        )}

            {error && <div className="error-message">{error}</div>}

      {/* Filters */}
      <div className="filters-panel">
        <h3>Filters</h3>
        <div className="filters-grid">
          <div className="filter-group">
            <label>Country Code (e.g., SE, DE, FR):</label>
            <input
              type="text"
              value={filters.country_code}
              onChange={(e) => handleFilterChange('country_code', e.target.value.toUpperCase())}
              placeholder="SE"
              maxLength="2"
            />
          </div>
          <div className="filter-group">
            <label>Year:</label>
            <input
              type="number"
              value={filters.year}
              onChange={(e) => handleFilterChange('year', e.target.value)}
              placeholder="2023"
              min="2015"
              max="2024"
            />
          </div>
          <div className="filter-group">
            <label>Records per page:</label>
            <select
              value={filters.limit}
              onChange={(e) => handleFilterChange('limit', e.target.value)}
            >
              <option value="25">25</option>
              <option value="50">50</option>
              <option value="100">100</option>
              <option value="200">200</option>
            </select>
          </div>
        </div>
        <div className="filter-actions">
          <button onClick={handleApplyFilters} className="btn-primary">Apply Filters</button>
          <button onClick={handleResetFilters} className="btn-secondary">Reset</button>
        </div>
      </div>

      {/* Charts Section */}
      {energyData && energyData.data && energyData.data.length > 0 && (
        <div className="charts-section">
          <h2>Data Visualization</h2>
          <div className="charts-grid">
            <TimeSeriesChart data={energyData} countryCode={filters.country_code} />
            {summary && summary.summary && summary.summary.length > 0 && (
              <CountryComparisonChart summary={summary} />
            )}
            <ProductDistributionChart data={energyData} />
            <FlowDistributionChart data={energyData} />
          </div>
        </div>
      )}

      {/* Summary */}
      {summary && summary.summary && summary.summary.length > 0 && (
        <div className="summary-panel">
          <h3>Summary by Country</h3>
          <div className="summary-grid">
            {summary.summary.slice(0, 10).map((item, idx) => (
              <div key={idx} className="summary-item">
                <div className="summary-label">{item.group}</div>
                <div className="summary-value">{formatNumber(item.total_gwh)} GWh</div>
                <div className="summary-count">{item.record_count} records</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Data Table */}
      {energyData && (
        <div className="data-panel">
          <div className="data-panel-header">
            <h3>
              Energy Records
              {energyData.total !== undefined && (
                <span className="total-count"> ({energyData.total.toLocaleString()} total)</span>
              )}
            </h3>
            <button onClick={exportToCSV} className="btn-export">
              📥 Export to CSV
            </button>
          </div>
          <div className="table-container">
            <table className="energy-table">
              <thead>
                <tr>
                  <th>Country</th>
                  <th>Year</th>
                  <th>Product</th>
                  <th>Flow</th>
                  <th>Value (GWh)</th>
                  <th>Source</th>
                </tr>
              </thead>
              <tbody>
                {energyData.data && energyData.data.length > 0 ? (
                  energyData.data.map((record, idx) => (
                    <tr key={idx}>
                      <td>
                        <strong>{record.country_code}</strong>
                        {record.country_name && <div className="country-name">{record.country_name}</div>}
                      </td>
                      <td>{record.year}</td>
                      <td>
                        {record.product_code || 'N/A'}
                        {record.product_name && <div className="detail-text">{record.product_name}</div>}
                      </td>
                      <td>
                        {record.flow_code || 'N/A'}
                        {record.flow_name && <div className="detail-text">{record.flow_name}</div>}
                      </td>
                      <td className="value-cell">{formatNumber(record.value_gwh)}</td>
                      <td>{record.source_table}</td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td colSpan="6" className="no-data">No data found</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          {/* Pagination */}
          {energyData.total > filters.limit && (
            <div className="pagination">
              <button
                onClick={() => setFilters(prev => ({ ...prev, offset: Math.max(0, prev.offset - prev.limit) }))}
                disabled={filters.offset === 0}
                className="btn-secondary"
              >
                Previous
              </button>
              <span className="page-info">
                Showing {filters.offset + 1} - {Math.min(filters.offset + filters.limit, energyData.total)} of {energyData.total}
              </span>
              <button
                onClick={() => setFilters(prev => ({ ...prev, offset: prev.offset + prev.limit }))}
                disabled={filters.offset + filters.limit >= energyData.total}
                className="btn-secondary"
              >
                Next
              </button>
            </div>
          )}
        </div>
      )}
      </div>
    </PageTemplate>
  );
}

export default Energy;
