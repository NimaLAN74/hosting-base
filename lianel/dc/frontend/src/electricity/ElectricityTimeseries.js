import React, { useState, useEffect } from 'react';
import { useKeycloak } from '../KeycloakProvider';
import PageTemplate from '../PageTemplate';
import './ElectricityTimeseries.css';

function ElectricityTimeseries() {
  const { authenticated, authenticatedFetch } = useKeycloak();
  const [data, setData] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [filters, setFilters] = useState({
    country_code: '',
    start_date: '',
    end_date: '',
    production_type: '',
    limit: 1000
  });

  const fetchData = React.useCallback(async () => {
    if (!authenticated) return;
    
    try {
      setLoading(true);
      setError('');

      const params = new URLSearchParams();
      if (filters.country_code && filters.country_code.trim()) {
        params.append('country_code', filters.country_code.trim());
      }
      if (filters.start_date && filters.start_date.trim()) {
        // Convert DDMMYYYY to YYYY-MM-DD format for API
        const dateStr = filters.start_date.trim();
        if (dateStr.length === 8 && /^\d{8}$/.test(dateStr)) {
          // DDMMYYYY format
          const day = dateStr.substring(0, 2);
          const month = dateStr.substring(2, 4);
          const year = dateStr.substring(4, 8);
          params.append('start_date', `${year}-${month}-${day}`);
        } else {
          // Already in YYYY-MM-DD format
          params.append('start_date', dateStr);
        }
      }
      if (filters.end_date && filters.end_date.trim()) {
        // Convert DDMMYYYY to YYYY-MM-DD format for API
        const dateStr = filters.end_date.trim();
        if (dateStr.length === 8 && /^\d{8}$/.test(dateStr)) {
          // DDMMYYYY format
          const day = dateStr.substring(0, 2);
          const month = dateStr.substring(2, 4);
          const year = dateStr.substring(4, 8);
          params.append('end_date', `${year}-${month}-${day}`);
        } else {
          // Already in YYYY-MM-DD format
          params.append('end_date', dateStr);
        }
      }
      if (filters.production_type && filters.production_type.trim()) {
        params.append('production_type', filters.production_type.trim());
      }
      params.append('limit', filters.limit || 1000);
      
      console.log('Fetching with params:', params.toString());

      const response = await authenticatedFetch(
        `/api/v1/electricity/timeseries?${params.toString()}`
      );

      if (response.ok) {
        const result = await response.json();
        setData(result.data || []);
        if (!result.data || result.data.length === 0) {
          setError('No data available. The electricity timeseries table may be empty. Please check if the ENTSO-E ingestion DAG has run successfully.');
        }
      } else {
        const errorData = await response.json().catch(() => ({}));
        const errorMessage = errorData.error || errorData.details || `Failed to load electricity data (${response.status})`;
        setError(errorMessage);
        console.error('Electricity API error:', errorData);
      }
    } catch (err) {
      console.error('Error fetching electricity data:', err);
      setError('Failed to load electricity data. Please try again.');
    } finally {
      setLoading(false);
    }
  }, [authenticated, filters, authenticatedFetch]);

  // Initial fetch when authenticated
  useEffect(() => {
    if (authenticated) {
      fetchData();
    }
  }, [authenticated, fetchData]);

  // Debounced fetch when filters change
  useEffect(() => {
    if (!authenticated) return;
    
    const timeoutId = setTimeout(() => {
      fetchData();
    }, 500); // Wait 500ms after last filter change

    return () => clearTimeout(timeoutId);
  }, [filters, authenticated, fetchData]);

  // Helper function to format date as DDMMYYYY
  const formatDateDDMMYYYY = (dateString) => {
    if (!dateString) return 'N/A';
    const date = new Date(dateString);
    const day = String(date.getDate()).padStart(2, '0');
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const year = date.getFullYear();
    return `${day}${month}${year}`;
  };

  // Helper function to format date as DDMMYYYY
  const formatDateDDMMYYYY = (dateString) => {
    if (!dateString) return 'N/A';
    try {
      const date = new Date(dateString);
      if (isNaN(date.getTime())) return 'N/A';
      const day = String(date.getDate()).padStart(2, '0');
      const month = String(date.getMonth() + 1).padStart(2, '0');
      const year = date.getFullYear();
      return `${day}${month}${year}`;
    } catch (e) {
      return 'N/A';
    }
  };

  const handleFilterChange = (field, value) => {
    setFilters(prev => ({ ...prev, [field]: value }));
  };

  // Aggregate data for visualization
  const aggregatedData = React.useMemo(() => {
    if (!data.length) return [];

    // Group by date and country
    const grouped = {};
    data.forEach(record => {
      const date = new Date(record.timestamp_utc).toISOString().split('T')[0];
      const key = `${date}_${record.country_code}`;
      
      if (!grouped[key]) {
        grouped[key] = {
          date,
          country_code: record.country_code,
          load_mw: 0,
          generation_mw: 0,
          renewable_generation: 0,
          fossil_generation: 0,
          count: 0
        };
      }

      if (record.load_mw) grouped[key].load_mw += record.load_mw;
      if (record.generation_mw) {
        grouped[key].generation_mw += record.generation_mw;
        if (record.production_type && ['B16', 'B18', 'B19', 'B11', 'B12', 'B13', 'B15', 'B01', 'B09'].includes(record.production_type)) {
          grouped[key].renewable_generation += record.generation_mw;
        }
        if (record.production_type && ['B02', 'B03', 'B04', 'B05', 'B06', 'B07', 'B08'].includes(record.production_type)) {
          grouped[key].fossil_generation += record.generation_mw;
        }
      }
      grouped[key].count++;
    });

    return Object.values(grouped).sort((a, b) => new Date(a.date) - new Date(b.date));
  }, [data]);

  if (!authenticated) {
    return (
      <PageTemplate title="Electricity Timeseries Data">
        <div className="electricity-container">Please log in to view electricity data.</div>
      </PageTemplate>
    );
  }

  return (
    <PageTemplate title="Electricity Timeseries Data">
      <div className="electricity-container">
        <p className="subtitle">High-frequency electricity load and generation data from ENTSO-E</p>

      {/* Filters */}
      <div className="filters">
        <div className="filter-group">
          <label>Country Code:</label>
          <input
            type="text"
            value={filters.country_code}
            onChange={(e) => handleFilterChange('country_code', e.target.value.toUpperCase())}
            placeholder="e.g., SE, DE, FR"
            maxLength={2}
          />
        </div>

        <div className="filter-group">
          <label>Start Date:</label>
          <input
            type="date"
            value={filters.start_date}
            onChange={(e) => handleFilterChange('start_date', e.target.value)}
          />
        </div>

        <div className="filter-group">
          <label>End Date:</label>
          <input
            type="date"
            value={filters.end_date}
            onChange={(e) => handleFilterChange('end_date', e.target.value)}
          />
        </div>

        <div className="filter-group">
          <label>Production Type:</label>
          <select
            value={filters.production_type}
            onChange={(e) => handleFilterChange('production_type', e.target.value)}
          >
            <option value="">All</option>
            <option value="B16">Solar</option>
            <option value="B18">Wind Offshore</option>
            <option value="B19">Wind Onshore</option>
            <option value="B14">Nuclear</option>
            <option value="B04">Fossil Gas</option>
            <option value="B05">Fossil Hard Coal</option>
          </select>
        </div>

        <div className="filter-group">
          <label>Limit:</label>
          <input
            type="number"
            value={filters.limit}
            onChange={(e) => handleFilterChange('limit', parseInt(e.target.value) || 1000)}
            min={1}
            max={10000}
          />
        </div>

        <button onClick={fetchData} className="btn-refresh">Refresh</button>
      </div>

      {error && <div className="error-message">{error}</div>}

      {loading ? (
        <div className="loading">Loading electricity data...</div>
      ) : (
        <>
          <div className="stats-summary">
            <div className="stat-card">
              <div className="stat-label">Total Records</div>
              <div className="stat-value">{data.length}</div>
            </div>
            <div className="stat-card">
              <div className="stat-label">Date Range</div>
              <div className="stat-value">
                {data.length > 0 ? (
                  <>
                    {new Date(data[data.length - 1].timestamp_utc).toLocaleDateString()} - {' '}
                    {new Date(data[0].timestamp_utc).toLocaleDateString()}
                  </>
                ) : 'N/A'}
              </div>
            </div>
            <div className="stat-card">
              <div className="stat-label">Countries</div>
              <div className="stat-value">
                {new Set(data.map(r => r.country_code)).size}
              </div>
            </div>
          </div>

          {/* Data Table */}
          <div className="data-table-container">
            <table className="data-table">
              <thead>
                <tr>
                  <th>Timestamp (UTC)</th>
                  <th>Country</th>
                  <th>Bidding Zone</th>
                  <th>Production Type</th>
                  <th>Load (MW)</th>
                  <th>Generation (MW)</th>
                  <th>Resolution</th>
                </tr>
              </thead>
              <tbody>
                {data.slice(0, 100).map((record, idx) => (
                  <tr key={idx}>
                    <td>{new Date(record.timestamp_utc).toLocaleString()}</td>
                    <td>{record.country_code}</td>
                    <td>{record.bidding_zone || 'N/A'}</td>
                    <td>{record.production_type || 'Load'}</td>
                    <td>{record.load_mw ? record.load_mw.toFixed(2) : '-'}</td>
                    <td>{record.generation_mw ? record.generation_mw.toFixed(2) : '-'}</td>
                    <td>{record.resolution}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {data.length > 100 && (
              <div className="table-note">
                Showing first 100 of {data.length} records
              </div>
            )}
          </div>
        </>
      )}
      </div>
    </PageTemplate>
  );
}

export default ElectricityTimeseries;
