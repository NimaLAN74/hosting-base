import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useKeycloak } from '../KeycloakProvider';
import PageTemplate from '../PageTemplate';
import { formatDateEU, formatDateTimeEU, toISODateFromInput, EU_DATE_INPUT_PLACEHOLDER, EU_DATE_INPUT_LABEL } from '../services/dateFormat';
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
        const apiDate = toISODateFromInput(filters.start_date);
        if (apiDate) {
          params.append('start_date', apiDate);
        } else {
          console.warn('Invalid start_date format:', filters.start_date);
        }
      }
      if (filters.end_date && filters.end_date.trim()) {
        const apiDate = toISODateFromInput(filters.end_date);
        if (apiDate) {
          params.append('end_date', apiDate);
        } else {
          console.warn('Invalid end_date format:', filters.end_date);
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

  // Helper function to format date as DD/MM/YYYY for display
  const formatDateDDMMYYYY = (dateString) => {
    return formatDateEU(dateString);
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

        <div className="filter-group eu-date-input-wrapper">
          <label>Start date ({EU_DATE_INPUT_LABEL}):</label>
          <input
            type="text"
            inputMode="text"
            autoComplete="off"
            data-date-format="eu"
            value={filters.start_date}
            onChange={(e) => handleFilterChange('start_date', e.target.value)}
            placeholder={EU_DATE_INPUT_PLACEHOLDER}
            title={`Use ${EU_DATE_INPUT_LABEL} or DDMMYYYY (e.g. 01/01/2024 or 01012024)`}
            aria-label={`Start date in ${EU_DATE_INPUT_LABEL} format`}
          />
        </div>

        <div className="filter-group eu-date-input-wrapper">
          <label>End date ({EU_DATE_INPUT_LABEL}):</label>
          <input
            type="text"
            inputMode="text"
            autoComplete="off"
            data-date-format="eu"
            value={filters.end_date}
            onChange={(e) => handleFilterChange('end_date', e.target.value)}
            placeholder={EU_DATE_INPUT_PLACEHOLDER}
            title={`Use ${EU_DATE_INPUT_LABEL} or DDMMYYYY (e.g. 31/12/2026 or 31122026)`}
            aria-label={`End date in ${EU_DATE_INPUT_LABEL} format`}
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
                    <td colSpan="7" className="data-table-empty">
                      No data to display for the selected filters
                    </td>
                  </tr>
                ) : (
                  data.slice(0, 100).map((record, idx) => {
                    console.log('Rendering record', idx, record);
                    if (!record) {
                      console.warn('Record is null/undefined at index', idx);
                      return null;
                    }
                    return (
                      <tr key={record.id || idx}>
                        <td>{formatDateTimeEU(record.timestamp_utc)}</td>
                        <td>{record.country_code || 'N/A'}</td>
                        <td>{record.bidding_zone || 'N/A'}</td>
                        <td>{record.production_type || 'Load'}</td>
                        <td>{record.load_mw != null ? Number(record.load_mw).toFixed(2) : '-'}</td>
                        <td>{record.generation_mw != null ? Number(record.generation_mw).toFixed(2) : '-'}</td>
                        <td>{record.resolution || 'N/A'}</td>
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
