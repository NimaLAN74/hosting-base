import React, { useState, useEffect, useCallback, useRef } from 'react';
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
  const [fullFilteredData, setFullFilteredData] = useState(null); // Store full dataset for charts
  const [filters, setFilters] = useState({
    country_codes: [], // Changed to array for multi-select
    years: [], // Changed to array for multi-select
    limit: 50,
    offset: 0
  });
  const [summary, setSummary] = useState(null);
  const [availableCountries, setAvailableCountries] = useState([]);
  const [availableYears, setAvailableYears] = useState([]);
  
  // Use ref to always have access to latest filters
  const filtersRef = useRef(filters);
  useEffect(() => {
    filtersRef.current = filters;
  }, [filters]);

  // Extract unique countries and years from data
  const extractOptions = useCallback((data) => {
    if (!data || !data.data) return;
    
    const countries = new Set();
    const years = new Set();
    const countryMap = new Map(); // Store country code -> country name mapping
    
    data.data.forEach(record => {
      if (record.country_code) {
        countries.add(record.country_code);
        // Store country name if available
        if (record.country_name && !countryMap.has(record.country_code)) {
          countryMap.set(record.country_code, record.country_name);
        }
      }
      if (record.year) {
        years.add(record.year);
      }
    });
    
    // Merge with existing options to preserve all countries/years
    setAvailableCountries(prev => {
      const existingMap = new Map(prev.map(opt => [opt.value, opt]));
      // Add new countries
      countries.forEach(code => {
        if (!existingMap.has(code)) {
          const name = countryMap.get(code);
          existingMap.set(code, {
            value: code,
            label: name ? `${code} - ${name}` : code
          });
        }
      });
      return Array.from(existingMap.values()).sort((a, b) => a.value.localeCompare(b.value));
    });
    
    setAvailableYears(prev => {
      const existingSet = new Set(prev.map(opt => opt.value));
      // Add new years
      years.forEach(year => {
        if (!existingSet.has(year)) {
          existingSet.add(year);
        }
      });
      return Array.from(existingSet)
        .sort((a, b) => b - a)
        .map(year => ({ value: year, label: String(year) }));
    });
  }, []);

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
        // Batch requests to avoid overwhelming the service
        const countryList = activeFilters.country_codes.length > 0 ? activeFilters.country_codes : [null];
        const yearList = activeFilters.years.length > 0 ? activeFilters.years : [null];
        
        // Create all request parameters
        const requestParams = [];
        for (const country of countryList) {
          for (const year of yearList) {
            const params = {
              limit: 10000, // Get all matching records
              offset: 0
            };
            if (country) params.country_code = country;
            if (year) params.year = parseInt(year);
            requestParams.push(params);
          }
        }
        
        console.log(`Fetching ${requestParams.length} requests in batches...`);
        
        // Process requests in batches to avoid overwhelming the service
        const BATCH_SIZE = 5; // Process 5 requests at a time
        const results = [];
        
        for (let i = 0; i < requestParams.length; i += BATCH_SIZE) {
          const batch = requestParams.slice(i, i + BATCH_SIZE);
          console.log(`Processing batch ${Math.floor(i / BATCH_SIZE) + 1}/${Math.ceil(requestParams.length / BATCH_SIZE)} (${batch.length} requests)...`);
          
          try {
            const batchPromises = batch.map(params => energyApi.getEnergyAnnual(params));
            const batchResults = await Promise.all(batchPromises);
            results.push(...batchResults);
            
            // Small delay between batches to avoid rate limiting
            if (i + BATCH_SIZE < requestParams.length) {
              await new Promise(resolve => setTimeout(resolve, 200)); // 200ms delay between batches
            }
          } catch (error) {
            console.error(`Batch ${Math.floor(i / BATCH_SIZE) + 1} failed:`, error);
            // Continue with other batches even if one fails
            // Add empty results for failed batch
            batch.forEach(() => results.push(null));
          }
        }
        
        console.log('Received', results.length, 'API responses');
        
        // Combine and deduplicate results
        const dataMap = new Map();
        let successCount = 0;
        let failureCount = 0;
        
        results.forEach((result, index) => {
          if (result && result.data && Array.isArray(result.data)) {
            successCount++;
            console.log(`Result ${index + 1}/${results.length}: ${result.data.length} records`);
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
            failureCount++;
            console.warn(`Result ${index + 1}/${results.length} is invalid or failed:`, result);
          }
        });
        
        if (failureCount > 0) {
          console.warn(`${failureCount} out of ${results.length} requests failed`);
        }
        
        if (successCount === 0) {
          throw new Error('All API requests failed. Please try again with fewer filters.');
        }
        
        allData = Array.from(dataMap.values());
        console.log('Combined data:', allData.length, 'unique records');
        
        // Store full dataset for charts (before pagination)
        setFullFilteredData({ data: allData });
        
        // Apply pagination for table
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
        
        // Extract options from full dataset (but don't replace existing options)
        // Only update if we have new countries/years
        if (allData.length > 0) {
          extractOptions({ data: allData });
        }
      } else {
        // No filters - fetch normally
        const params = {
          limit: parseInt(activeFilters.limit) || 50,
          offset: activeFilters.offset || 0
        };
        
        const data = await energyApi.getEnergyAnnual(params);
        setEnergyData(data);
        // For charts, use the same data when no filters (or use full dataset if available)
        setFullFilteredData(data);
        
        // Fetch a much larger dataset to populate all available options (only on first load)
        // This ensures we get all countries and years, not just those in the first page
        if (availableCountries.length === 0 && availableYears.length === 0) {
          console.log('Fetching large dataset to populate all options...');
          // Fetch multiple pages to get all countries/years
          const allOptionsData = [];
          let offset = 0;
          const pageSize = 1000;
          let hasMore = true;
          
          // Fetch up to 10 pages (10,000 records) to get all options
          while (hasMore && offset < 10000) {
            const pageData = await energyApi.getEnergyAnnual({ limit: pageSize, offset });
            if (pageData.data && pageData.data.length > 0) {
              allOptionsData.push(...pageData.data);
              offset += pageSize;
              hasMore = pageData.data.length === pageSize;
            } else {
              hasMore = false;
            }
          }
          
          console.log(`Fetched ${allOptionsData.length} records for options`);
          extractOptions({ data: allOptionsData });
        } else {
          // Only update options if we have new data, don't replace existing options
          if (data && data.data && data.data.length > 0) {
            extractOptions(data);
          }
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
      const errorMessage = err.message || 'Failed to load energy data';
      
      // Provide more helpful error messages
      if (errorMessage.includes('503') || errorMessage.includes('Service Temporarily Unavailable')) {
        setError('Service is temporarily unavailable. This may be due to too many requests. Please try again in a moment or reduce the number of selected filters.');
      } else if (errorMessage.includes('All API requests failed')) {
        setError('All requests failed. Please try with fewer countries or years selected.');
      } else {
        setError(errorMessage);
      }
    } finally {
      setLoading(false);
    }
  }, [filters, availableCountries.length, availableYears.length, extractOptions]);

  const handleFilterChange = (field, value) => {
    console.log('Filter changed:', field, value, 'Type:', Array.isArray(value) ? 'array' : typeof value);
    setFilters(prev => {
      // Ensure value is an array for country_codes and years
      const arrayValue = Array.isArray(value) ? value : (value ? [value] : []);
      const newFilters = { ...prev, [field]: arrayValue, offset: 0 };
      console.log('New filters state:', newFilters);
      // Don't auto-fetch when filters change - user must click "Apply Filters"
      return newFilters;
    });
  };

  const handleApplyFilters = () => {
    // Get the latest filters from state and reset offset
    // Update both state and ref, then fetch data
    setFilters(prev => {
      const latestFilters = { ...prev, offset: 0 }; // Reset offset when applying filters
      filtersRef.current = latestFilters;
      console.log('Applying filters:', latestFilters);
      return latestFilters;
    });
    
    // Fetch data after state update completes
    // Use the filters from state (will be updated by the setFilters above)
    // We need to wait for the state update, so use the ref which we just updated
    const filtersToUse = filtersRef.current;
    console.log('Fetching data with filters:', filtersToUse);
    fetchData(filtersToUse);
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
      {/* Use fullFilteredData for charts (all filtered records), energyData for table (paginated) */}
      {fullFilteredData && fullFilteredData.data && fullFilteredData.data.length > 0 && (
        <div className="charts-section">
          <h2>Data Visualization</h2>
          <div className="charts-grid">
            <TimeSeriesChart 
              data={fullFilteredData} 
              countryCode={filters.country_codes.length === 1 ? filters.country_codes[0] : undefined}
              countryCodes={filters.country_codes.length > 1 ? filters.country_codes : undefined}
            />
            {summary && summary.summary && summary.summary.length > 0 && (
              <CountryComparisonChart summary={summary} />
            )}
            <ProductDistributionChart data={fullFilteredData} />
            <FlowDistributionChart data={fullFilteredData} />
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
