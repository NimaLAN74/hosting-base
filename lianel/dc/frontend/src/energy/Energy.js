import React, { useState, useEffect, useCallback } from 'react';
import { Link } from 'react-router-dom';
import PageTemplate from '../PageTemplate';
import { energyApi } from './energyApi';
import { TimeSeriesChart, CountryComparisonChart, ProductDistributionChart, FlowDistributionChart } from './EnergyCharts';
import MultiSelect from './MultiSelect';
import '../App.css';
import './Energy.css';

function Energy() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [serviceInfo, setServiceInfo] = useState(null);
  const [energyData, setEnergyData] = useState(null);
  const [filters, setFilters] = useState({
    country_codes: [], // Changed to array for multi-select
    years: [], // Changed to array for multi-select
    limit: 50,
    offset: 0
  });
  const [summary, setSummary] = useState(null);
  const [availableCountries, setAvailableCountries] = useState([]);
  const [availableYears, setAvailableYears] = useState([]);

  // Extract unique countries and years from data
  const extractOptions = (data) => {
    if (!data || !data.data) return;
    
    const countries = new Set();
    const years = new Set();
    
    data.data.forEach(record => {
      if (record.country_code) {
        countries.add(record.country_code);
      }
      if (record.year) {
        years.add(record.year);
      }
    });
    
    const countryOptions = Array.from(countries)
      .sort()
      .map(code => {
        const record = data.data.find(r => r.country_code === code);
        return {
          value: code,
          label: record?.country_name ? `${code} - ${record.country_name}` : code
        };
      });
    
    const yearOptions = Array.from(years)
      .sort((a, b) => b - a) // Sort descending
      .map(year => ({ value: year, label: String(year) }));
    
    setAvailableCountries(countryOptions);
    setAvailableYears(yearOptions);
  };

  const fetchData = useCallback(async (currentFilters = null) => {
    try {
      setLoading(true);
      setError('');
      
      // Use provided filters or current state
      const activeFilters = currentFilters || filters;

      // Fetch service info
      const info = await energyApi.getServiceInfo();
      setServiceInfo(info);

      // Fetch energy data - handle multiple countries/years
      let allData = [];
      let totalCount = 0;
      
      if (activeFilters.country_codes.length > 0 || activeFilters.years.length > 0) {
        // If filters are selected, fetch for each combination
        const countryList = activeFilters.country_codes.length > 0 ? activeFilters.country_codes : [null];
        const yearList = activeFilters.years.length > 0 ? activeFilters.years : [null];
        
        const promises = [];
        for (const country of countryList) {
          for (const year of yearList) {
            const params = {
              limit: 10000, // Get all matching records
              offset: 0
            };
            if (country) params.country_code = country;
            if (year) params.year = parseInt(year);
            
            promises.push(energyApi.getEnergyAnnual(params));
          }
        }
        
        const results = await Promise.all(promises);
        
        console.log('Received', results.length, 'API responses');
        
        // Combine and deduplicate results
        const dataMap = new Map();
        results.forEach((result, index) => {
          if (result && result.data && Array.isArray(result.data)) {
            console.log(`Result ${index}: ${result.data.length} records`);
            result.data.forEach(record => {
              if (record) {
                const key = `${record.country_code}-${record.year}-${record.product_code}-${record.flow_code}`;
                if (!dataMap.has(key)) {
                  dataMap.set(key, record);
                }
              }
            });
            totalCount = Math.max(totalCount, result.total || 0);
          } else {
            console.warn(`Result ${index} is invalid:`, result);
          }
        });
        
        allData = Array.from(dataMap.values());
        console.log('Combined data:', allData.length, 'unique records');
        
        // Apply pagination
        const start = activeFilters.offset || 0;
        const limit = parseInt(activeFilters.limit) || 50;
        const paginatedData = allData.slice(start, start + limit);
        
        console.log('Setting energy data:', { 
          total: allData.length, 
          paginated: paginatedData.length,
          start,
          limit
        });
        
        setEnergyData({
          data: paginatedData,
          total: allData.length,
          limit: limit,
          offset: start
        });
        
        // Extract options from full dataset
        extractOptions({ data: allData });
      } else {
        // No filters - fetch normally
        const params = {
          limit: parseInt(activeFilters.limit) || 50,
          offset: activeFilters.offset || 0
        };
        
        const data = await energyApi.getEnergyAnnual(params);
        setEnergyData(data);
        
        // Fetch a larger dataset to populate all available options (only on first load)
        if (availableCountries.length === 0 && availableYears.length === 0) {
          const optionsData = await energyApi.getEnergyAnnual({ limit: 1000, offset: 0 });
          extractOptions(optionsData);
        } else {
          extractOptions(data);
        }
      }

      // Fetch summary (use first selected country/year if multiple)
      const summaryData = await energyApi.getEnergySummary({
        country_code: activeFilters.country_codes.length > 0 ? activeFilters.country_codes[0] : undefined,
        year: activeFilters.years.length > 0 ? parseInt(activeFilters.years[0]) : undefined,
        group_by: 'country'
      });
      setSummary(summaryData);
    } catch (err) {
      console.error('Error fetching energy data:', err);
      setError(err.message || 'Failed to load energy data');
    } finally {
      setLoading(false);
    }
  }, [filters, availableCountries.length, availableYears.length]);

  const handleFilterChange = (field, value) => {
    console.log('Filter changed:', field, value);
    setFilters(prev => {
      const newFilters = { ...prev, [field]: value, offset: 0 };
      console.log('New filters state:', newFilters);
      return newFilters;
    });
  };

  const handleApplyFilters = () => {
    // Use a ref-like pattern to get the latest filters
    // Since we can't use refs easily here, we'll use the filters from the closure
    // which should be up-to-date due to the dependency array in useCallback
    console.log('Applying filters:', filters);
    fetchData(filters);
  };

  const handleResetFilters = () => {
    const resetFilters = { country_codes: [], years: [], limit: 50, offset: 0 };
    setFilters(resetFilters);
    fetchData(resetFilters);
  };

  // Auto-fetch when offset changes (for pagination)
  useEffect(() => {
    fetchData();
  }, [filters.offset, fetchData]);

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
            <MultiSelect
              label="Countries"
              options={availableCountries}
              selected={filters.country_codes}
              onChange={(selected) => handleFilterChange('country_codes', selected)}
              placeholder="Select countries..."
              searchable={true}
            />
          </div>
          <div className="filter-group">
            <MultiSelect
              label="Years"
              options={availableYears}
              selected={filters.years}
              onChange={(selected) => handleFilterChange('years', selected)}
              placeholder="Select years..."
              searchable={true}
            />
          </div>
          <div className="filter-group">
            <label>Records per page:</label>
            <select
              value={filters.limit}
              onChange={(e) => handleFilterChange('limit', e.target.value)}
              style={{
                padding: '10px 12px',
                background: 'rgba(255, 255, 255, 0.1)',
                border: '1px solid var(--border-color)',
                borderRadius: '8px',
                color: 'var(--text-primary)',
                fontSize: '1em',
                width: '100%'
              }}
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
        {(filters.country_codes.length > 0 || filters.years.length > 0) && (
          <div className="active-filters">
            <strong>Active filters:</strong>
            {filters.country_codes.length > 0 && (
              <span className="filter-tag">
                Countries: {filters.country_codes.join(', ')}
              </span>
            )}
            {filters.years.length > 0 && (
              <span className="filter-tag">
                Years: {filters.years.join(', ')}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Charts Section */}
      {energyData && energyData.data && energyData.data.length > 0 && (
        <div className="charts-section">
          <h2>Data Visualization</h2>
            <div className="charts-grid">
            <TimeSeriesChart data={energyData} countryCode={filters.country_codes.length === 1 ? filters.country_codes[0] : undefined} />
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
          {energyData && energyData.total && energyData.total > filters.limit && (
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
