import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
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

  const fetchData = useCallback(async () => {
    if (!authenticated) {
      console.log('Not authenticated, skipping fetch');
      return;
    }
    
    console.log('=== Starting fetchData ===');
    console.log('Current filters:', JSON.stringify(filters, null, 2));
    
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
          const apiDate = `${year}-${month}-${day}`;
          params.append('start_date', apiDate);
          console.log('Converted start_date:', dateStr, '->', apiDate);
        } else if (dateStr.match(/^\d{4}-\d{2}-\d{2}$/)) {
          // Already in YYYY-MM-DD format
          params.append('start_date', dateStr);
          console.log('Using start_date as-is:', dateStr);
        } else {
          console.warn('Invalid start_date format:', dateStr);
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
          const apiDate = `${year}-${month}-${day}`;
          params.append('end_date', apiDate);
          console.log('Converted end_date:', dateStr, '->', apiDate);
        } else if (dateStr.match(/^\d{4}-\d{2}-\d{2}$/)) {
          // Already in YYYY-MM-DD format
          params.append('end_date', dateStr);
          console.log('Using end_date as-is:', dateStr);
        } else {
          console.warn('Invalid end_date format:', dateStr);
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
        console.log('API response received:', {
          dataCount: result.data?.length || 0,
          total: result.total || 0,
          firstRecord: result.data?.[0] || null,
          fullData: result.data
        });
        const dataArray = result.data || [];
        console.log('Setting data state with', dataArray.length, 'records');
        console.log('First record in array:', dataArray[0]);
        setData(dataArray);
        if (!result.data || result.data.length === 0) {
          setError('No data available for the selected filters. Try adjusting the date range or country code.');
        } else {
          setError(''); // Clear error if we have data
          console.log('Data set successfully, should render', dataArray.length, 'records');
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

  // Debounced fetch when filters change or when authenticated
  useEffect(() => {
    if (!authenticated) return;
    
    const timeoutId = setTimeout(() => {
      console.log('Filters changed, fetching data with:', {
        country_code: filters.country_code,
        start_date: filters.start_date,
        end_date: filters.end_date,
        production_type: filters.production_type,
        limit: filters.limit
      });
      fetchData();
    }, 500); // Wait 500ms after last filter change

    return () => clearTimeout(timeoutId);
  }, [authenticated, filters.country_code, filters.start_date, filters.end_date, filters.production_type, filters.limit, fetchData]);

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
    console.log(`Filter changed: ${field} = ${value}`);
    setFilters(prev => {
      const newFilters = { ...prev, [field]: value };
      console.log('New filters state:', newFilters);
      return newFilters;
    });
  };

  // Aggregate data for visualization
  const aggregatedData = useMemo(() => {
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
            value={filters.start_date ? (() => {
              // Convert DDMMYYYY to YYYY-MM-DD for date input
              const dateStr = filters.start_date.trim();
              if (dateStr.length === 8 && /^\d{8}$/.test(dateStr)) {
                const day = dateStr.substring(0, 2);
                const month = dateStr.substring(2, 4);
                const year = dateStr.substring(4, 8);
                return `${year}-${month}-${day}`;
              }
              return dateStr; // Already in YYYY-MM-DD format
            })() : ''}
            onChange={(e) => {
              const value = e.target.value;
              if (value) {
                // Convert YYYY-MM-DD to DDMMYYYY for storage
                const [year, month, day] = value.split('-');
                handleFilterChange('start_date', `${day}${month}${year}`);
              } else {
                handleFilterChange('start_date', '');
              }
            }}
          />
          <input
            type="text"
            value={filters.start_date}
            onChange={(e) => {
              const value = e.target.value.replace(/\D/g, '').slice(0, 8);
              handleFilterChange('start_date', value);
            }}
            placeholder="Or enter DDMMYYYY"
            pattern="\d{8}"
            maxLength={8}
            style={{ marginTop: '5px', width: '100%' }}
          />
        </div>

        <div className="filter-group">
          <label>End Date:</label>
          <input
            type="date"
            value={filters.end_date ? (() => {
              // Convert DDMMYYYY to YYYY-MM-DD for date input
              const dateStr = filters.end_date.trim();
              if (dateStr.length === 8 && /^\d{8}$/.test(dateStr)) {
                const day = dateStr.substring(0, 2);
                const month = dateStr.substring(2, 4);
                const year = dateStr.substring(4, 8);
                return `${year}-${month}-${day}`;
              }
              return dateStr; // Already in YYYY-MM-DD format
            })() : ''}
            onChange={(e) => {
              const value = e.target.value;
              if (value) {
                // Convert YYYY-MM-DD to DDMMYYYY for storage
                const [year, month, day] = value.split('-');
                handleFilterChange('end_date', `${day}${month}${year}`);
              } else {
                handleFilterChange('end_date', '');
              }
            }}
          />
          <input
            type="text"
            value={filters.end_date}
            onChange={(e) => {
              const value = e.target.value.replace(/\D/g, '').slice(0, 8);
              handleFilterChange('end_date', value);
            }}
            placeholder="Or enter DDMMYYYY"
            pattern="\d{8}"
            maxLength={8}
            style={{ marginTop: '5px', width: '100%' }}
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
                {data.length > 0 ? (() => {
                  const dates = data.map(r => new Date(r.timestamp_utc)).sort((a, b) => a - b);
                  const minDate = dates[0];
                  const maxDate = dates[dates.length - 1];
                  return `${formatDateDDMMYYYY(minDate.toISOString())} - ${formatDateDDMMYYYY(maxDate.toISOString())}`;
                })() : 'N/A'}
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
                {data.length === 0 ? (
                  <tr>
                    <td colSpan="7" style={{ textAlign: 'center', padding: '20px' }}>
                      No data to display
                    </td>
                  </tr>
                ) : (
                  data.slice(0, 100).map((record, idx) => {
                    console.log('Rendering record', idx, record);
                    return (
                      <tr key={idx}>
                        <td>{formatDateDDMMYYYY(record.timestamp_utc)} {new Date(record.timestamp_utc).toLocaleTimeString()}</td>
                        <td>{record.country_code}</td>
                        <td>{record.bidding_zone || 'N/A'}</td>
                        <td>{record.production_type || 'Load'}</td>
                        <td>{record.load_mw ? record.load_mw.toFixed(2) : '-'}</td>
                        <td>{record.generation_mw ? record.generation_mw.toFixed(2) : '-'}</td>
                        <td>{record.resolution}</td>
                      </tr>
                    );
                  })
                )}
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
