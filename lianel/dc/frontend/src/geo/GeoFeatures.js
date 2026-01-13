import React, { useState, useEffect } from 'react';
import { useKeycloak } from '../KeycloakProvider';
import './GeoFeatures.css';

function GeoFeatures() {
  const { authenticated, authenticatedFetch } = useKeycloak();
  const [data, setData] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [filters, setFilters] = useState({
    region_id: '',
    feature_name: '',
    snapshot_year: new Date().getFullYear(),
    limit: 1000
  });

  useEffect(() => {
    if (authenticated) {
      fetchData();
    }
  }, [authenticated, filters]);

  const fetchData = async () => {
    try {
      setLoading(true);
      setError('');

      const params = new URLSearchParams();
      if (filters.region_id) params.append('region_id', filters.region_id);
      if (filters.feature_name) params.append('feature_name', filters.feature_name);
      if (filters.snapshot_year) params.append('snapshot_year', filters.snapshot_year);
      params.append('limit', filters.limit);

      const response = await authenticatedFetch(
        `/api/v1/geo/features?${params.toString()}`
      );

      if (response.ok) {
        const result = await response.json();
        setData(result.data || []);
      } else {
        const errorData = await response.json().catch(() => ({}));
        setError(errorData.error || 'Failed to load geo features');
      }
    } catch (err) {
      console.error('Error fetching geo features:', err);
      setError('Failed to load geo features. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const handleFilterChange = (field, value) => {
    setFilters(prev => ({ ...prev, [field]: value }));
  };

  // Group features by region
  const featuresByRegion = React.useMemo(() => {
    const grouped = {};
    data.forEach(record => {
      if (!grouped[record.region_id]) {
        grouped[record.region_id] = {
          region_id: record.region_id,
          snapshot_year: record.snapshot_year,
          features: {}
        };
      }
      grouped[record.region_id].features[record.feature_name] = record.feature_value;
    });
    return Object.values(grouped);
  }, [data]);

  // Feature categories for better organization
  const featureCategories = {
    'Energy Infrastructure': ['power_plant_count', 'power_generator_count', 'power_substation_count'],
    'Industrial': ['industrial_area_km2'],
    'Buildings': ['residential_building_count', 'commercial_building_count'],
    'Transport': ['railway_station_count', 'airport_count'],
    'Densities': ['power_plant_density_per_km2', 'industrial_density_per_km2']
  };

  if (!authenticated) {
    return <div className="geo-container">Please log in to view geo features.</div>;
  }

  return (
    <div className="geo-container">
      <h2>Geospatial Features (OSM)</h2>
      <p className="subtitle">OpenStreetMap features aggregated to NUTS2 regions</p>

      {/* Filters */}
      <div className="filters">
        <div className="filter-group">
          <label>Region ID (NUTS):</label>
          <input
            type="text"
            value={filters.region_id}
            onChange={(e) => handleFilterChange('region_id', e.target.value.toUpperCase())}
            placeholder="e.g., SE11, DE11, FR10"
          />
        </div>

        <div className="filter-group">
          <label>Feature Name:</label>
          <select
            value={filters.feature_name}
            onChange={(e) => handleFilterChange('feature_name', e.target.value)}
          >
            <option value="">All Features</option>
            <optgroup label="Energy Infrastructure">
              <option value="power_plant_count">Power Plant Count</option>
              <option value="power_generator_count">Power Generator Count</option>
              <option value="power_substation_count">Power Substation Count</option>
            </optgroup>
            <optgroup label="Industrial">
              <option value="industrial_area_km2">Industrial Area (km²)</option>
            </optgroup>
            <optgroup label="Buildings">
              <option value="residential_building_count">Residential Buildings</option>
              <option value="commercial_building_count">Commercial Buildings</option>
            </optgroup>
            <optgroup label="Transport">
              <option value="railway_station_count">Railway Stations</option>
              <option value="airport_count">Airports</option>
            </optgroup>
          </select>
        </div>

        <div className="filter-group">
          <label>Snapshot Year:</label>
          <input
            type="number"
            value={filters.snapshot_year}
            onChange={(e) => handleFilterChange('snapshot_year', parseInt(e.target.value) || new Date().getFullYear())}
            min={2000}
            max={new Date().getFullYear()}
          />
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
        <div className="loading">Loading geo features...</div>
      ) : (
        <>
          <div className="stats-summary">
            <div className="stat-card">
              <div className="stat-label">Total Records</div>
              <div className="stat-value">{data.length}</div>
            </div>
            <div className="stat-card">
              <div className="stat-label">Unique Regions</div>
              <div className="stat-value">
                {new Set(data.map(r => r.region_id)).size}
              </div>
            </div>
            <div className="stat-card">
              <div className="stat-label">Feature Types</div>
              <div className="stat-value">
                {new Set(data.map(r => r.feature_name)).size}
              </div>
            </div>
          </div>

          {/* Features by Region */}
          <div className="regions-grid">
            {featuresByRegion.slice(0, 50).map((region, idx) => (
              <div key={idx} className="region-card">
                <div className="region-header">
                  <h3>{region.region_id}</h3>
                  <span className="snapshot-year">Year: {region.snapshot_year}</span>
                </div>
                <div className="features-list">
                  {Object.entries(region.features).map(([name, value]) => (
                    <div key={name} className="feature-item">
                      <span className="feature-name">{name.replace(/_/g, ' ')}:</span>
                      <span className="feature-value">
                        {typeof value === 'number' 
                          ? value.toLocaleString(undefined, { maximumFractionDigits: 2 })
                          : value}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>

          {featuresByRegion.length > 50 && (
            <div className="table-note">
              Showing first 50 of {featuresByRegion.length} regions
            </div>
          )}

          {/* Raw Data Table */}
          <details className="raw-data-section">
            <summary>Raw Data Table</summary>
            <div className="data-table-container">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Region ID</th>
                    <th>Feature Name</th>
                    <th>Feature Value</th>
                    <th>Snapshot Year</th>
                  </tr>
                </thead>
                <tbody>
                  {data.slice(0, 100).map((record, idx) => (
                    <tr key={idx}>
                      <td>{record.region_id}</td>
                      <td>{record.feature_name}</td>
                      <td>
                        {typeof record.feature_value === 'number'
                          ? record.feature_value.toLocaleString(undefined, { maximumFractionDigits: 2 })
                          : record.feature_value}
                      </td>
                      <td>{record.snapshot_year}</td>
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
          </details>
        </>
      )}
    </div>
  );
}

export default GeoFeatures;
