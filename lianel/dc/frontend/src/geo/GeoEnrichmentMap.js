import React, { useState, useEffect, useCallback } from 'react';
import { MapContainer, TileLayer, CircleMarker, Popup, useMap } from 'react-leaflet';
import { useKeycloak } from '../KeycloakProvider';
import PageTemplate from '../PageTemplate';
import 'leaflet/dist/leaflet.css';
import './GeoEnrichmentMap.css';

// Fix for default marker icons in React-Leaflet
import L from 'leaflet';
delete L.Icon.Default.prototype._getIconUrl;
L.Icon.Default.mergeOptions({
  iconRetinaUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/images/marker-icon-2x.png',
  iconUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/images/marker-icon.png',
  shadowUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/images/marker-shadow.png',
});

// Component to fit map bounds to data
function FitBounds({ bounds }) {
  const map = useMap();
  
  useEffect(() => {
    if (bounds && bounds.length === 2) {
      map.fitBounds(bounds, { padding: [50, 50] });
    }
  }, [map, bounds]);
  
  return null;
}

function GeoEnrichmentMap() {
  const { authenticated, authenticatedFetch } = useKeycloak();
  const [data, setData] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [filters, setFilters] = useState({
    cntr_code: '',
    year: new Date().getFullYear(),
    limit: 1000
  });
  const [mapBounds, setMapBounds] = useState(null);
  const [selectedMetric, setSelectedMetric] = useState('total_energy_gwh'); // total_energy_gwh, osm_feature_count, power_plant_count

  const fetchData = useCallback(async () => {
    if (!authenticated) {
      setLoading(false);
      return;
    }

    try {
      setLoading(true);
      setError('');

      const params = new URLSearchParams();
      if (filters.cntr_code && filters.cntr_code.trim()) {
        params.append('cntr_code', filters.cntr_code.trim());
      }
      if (filters.year) {
        params.append('year', filters.year);
      }
      params.append('limit', filters.limit || 1000);

      const response = await authenticatedFetch(
        `/api/v1/datasets/geo-enrichment?${params.toString()}`
      );

      if (response.ok) {
        const result = await response.json();
        const dataArray = result.data || [];
        console.log('Geo-enrichment data received:', dataArray.length, 'records');
        console.log('Sample record:', dataArray[0]);
        
        setData(dataArray);
        
        // Calculate map bounds from data with coordinates
        const dataWithCoords = dataArray.filter(d => d.latitude && d.longitude);
        console.log('Records with coordinates:', dataWithCoords.length);
        
        if (dataWithCoords.length > 0) {
          const lats = dataWithCoords.map(d => d.latitude).filter(Boolean);
          const lngs = dataWithCoords.map(d => d.longitude).filter(Boolean);
          if (lats.length > 0 && lngs.length > 0) {
            const minLat = Math.min(...lats);
            const maxLat = Math.max(...lats);
            const minLng = Math.min(...lngs);
            const maxLng = Math.max(...lngs);
            console.log('Map bounds:', [[minLat, minLng], [maxLat, maxLng]]);
            setMapBounds([[minLat, minLng], [maxLat, maxLng]]);
          } else {
            // Default bounds for Europe
            setMapBounds([[35, -10], [72, 40]]);
          }
        } else {
          console.warn('No records with coordinates found');
          setMapBounds([[35, -10], [72, 40]]);
        }
      } else {
        const errorData = await response.json().catch(() => ({}));
        setError(errorData.error || errorData.details || `Failed to load geo-enrichment data (${response.status})`);
      }
    } catch (err) {
      console.error('Error fetching geo-enrichment data:', err);
      setError('Failed to load geo-enrichment data. Please try again.');
    } finally {
      setLoading(false);
    }
  }, [authenticated, filters, authenticatedFetch]);

  useEffect(() => {
    if (authenticated) {
      fetchData();
    }
  }, [authenticated, fetchData]);

  // Get color based on selected metric value
  const getColor = (value, maxValue) => {
    if (!value || maxValue === 0) return '#808080'; // Gray for no data
    
    const ratio = value / maxValue;
    if (ratio > 0.8) return '#d73027'; // Red - high
    if (ratio > 0.6) return '#f46d43'; // Orange-red
    if (ratio > 0.4) return '#fdae61'; // Orange
    if (ratio > 0.2) return '#fee08b'; // Yellow
    return '#e6f598'; // Light green - low
  };

  // Get radius based on selected metric value
  const getRadius = (value, maxValue) => {
    if (!value || maxValue === 0) return 5;
    const ratio = value / maxValue;
    return Math.max(5, Math.min(20, 5 + (ratio * 15)));
  };

  // Calculate max value for selected metric
  const maxValue = data.length > 0
    ? Math.max(...data.map(d => {
        const val = d[selectedMetric];
        return val ? (typeof val === 'number' ? val : parseFloat(val)) : 0;
      }))
    : 1;

  if (!authenticated) {
    return (
      <PageTemplate title="Geo-Enrichment Map">
        <div className="geo-map-container">Please log in to view geo-enrichment map.</div>
      </PageTemplate>
    );
  }

  return (
    <PageTemplate title="Geo-Enrichment Map Visualization">
      <div className="geo-map-container">
        <p className="subtitle">Interactive map showing energy data and OSM features by region</p>

        {/* Filters */}
        <div className="map-filters">
          <div className="filter-group">
            <label>Country Code:</label>
            <input
              type="text"
              value={filters.cntr_code}
              onChange={(e) => setFilters({ ...filters, cntr_code: e.target.value.toUpperCase() })}
              placeholder="e.g., SE, DE, FR"
              maxLength={2}
            />
          </div>

          <div className="filter-group">
            <label>Year:</label>
            <input
              type="number"
              value={filters.year}
              onChange={(e) => setFilters({ ...filters, year: parseInt(e.target.value) || new Date().getFullYear() })}
              min={2015}
              max={2024}
            />
          </div>

          <div className="filter-group">
            <label>Visualize:</label>
            <select
              value={selectedMetric}
              onChange={(e) => setSelectedMetric(e.target.value)}
            >
              <option value="total_energy_gwh">Total Energy (GWh)</option>
              <option value="osm_feature_count">OSM Feature Count</option>
              <option value="power_plant_count">Power Plant Count</option>
              <option value="industrial_area_km2">Industrial Area (km²)</option>
              <option value="pct_renewable">Renewable Share (%)</option>
            </select>
          </div>

          <button onClick={fetchData} className="btn-refresh">Refresh</button>
        </div>

        {error && <div className="error-message">{error}</div>}

        {loading ? (
          <div className="loading">Loading geo-enrichment data...</div>
        ) : (
          <>
            <div className="map-stats">
              <div className="stat-item">
                <span className="stat-label">Total Regions:</span>
                <span className="stat-value">{data.length}</span>
              </div>
              <div className="stat-item">
                <span className="stat-label">With Coordinates:</span>
                <span className="stat-value">{data.filter(d => d.latitude && d.longitude).length}</span>
              </div>
              <div className="stat-item">
                <span className="stat-label">Max {selectedMetric.replace('_', ' ')}:</span>
                <span className="stat-value">
                  {maxValue > 0 ? maxValue.toLocaleString(undefined, { maximumFractionDigits: 2 }) : 'N/A'}
                </span>
              </div>
            </div>

            {/* Map Container */}
            <div className="map-wrapper">
              <MapContainer
                center={[55, 15]} // Center of Europe
                zoom={4}
                style={{ height: '600px', width: '100%' }}
                scrollWheelZoom={true}
              >
                <TileLayer
                  attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
                  url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
                />
                
                {mapBounds && <FitBounds bounds={mapBounds} />}

                {/* Plot markers for regions with coordinates */}
                {(() => {
                  const dataWithCoords = data.filter(d => d.latitude && d.longitude);
                  console.log('Rendering markers for', dataWithCoords.length, 'records with coordinates');
                  console.log('Selected metric:', selectedMetric, 'Max value:', maxValue);
                  
                  return dataWithCoords.map((record, idx) => {
                    const value = record[selectedMetric];
                    const numValue = value ? (typeof value === 'number' ? value : parseFloat(value)) : 0;
                    const color = getColor(numValue, maxValue);
                    const radius = getRadius(numValue, maxValue);
                    
                    if (idx < 3) {
                      console.log(`Marker ${idx}:`, {
                        region: record.region_id,
                        lat: record.latitude,
                        lng: record.longitude,
                        value: numValue,
                        color,
                        radius
                      });
                    }
                    
                    return (
                      <CircleMarker
                        key={`${record.region_id}-${record.year}-${idx}`}
                        center={[record.latitude, record.longitude]}
                        radius={radius}
                        pathOptions={{ color, fillColor: color, fillOpacity: 0.6, weight: 2 }}
                      >
                        <Popup>
                          <div className="popup-content">
                            <h4>{record.region_id}</h4>
                            <p><strong>Country:</strong> {record.cntr_code}</p>
                            <p><strong>Year:</strong> {record.year}</p>
                            <p><strong>Total Energy:</strong> {record.total_energy_gwh ? record.total_energy_gwh.toLocaleString(undefined, { maximumFractionDigits: 2 }) : 'N/A'} GWh</p>
                            <p><strong>Renewable:</strong> {record.pct_renewable ? `${record.pct_renewable.toFixed(2)}%` : 'N/A'}</p>
                            <p><strong>OSM Features:</strong> {record.osm_feature_count || 'N/A'}</p>
                            <p><strong>Power Plants:</strong> {record.power_plant_count || 'N/A'}</p>
                            <p><strong>Industrial Area:</strong> {record.industrial_area_km2 ? `${record.industrial_area_km2.toFixed(2)} km²` : 'N/A'}</p>
                          </div>
                        </Popup>
                      </CircleMarker>
                    );
                  });
                })()}
                
                {/* Show note if some regions don't have coordinates */}
                {data.length > 0 && data.filter(d => !d.latitude || !d.longitude).length > 0 && (
                  <div className="map-note">
                    <p>⚠️ {data.filter(d => !d.latitude || !d.longitude).length} regions without coordinates (not shown on map)</p>
                  </div>
                )}
              </MapContainer>
            </div>

            {/* Data Table */}
            <div className="data-table-container">
              <h3>Region Data</h3>
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Region ID</th>
                    <th>Country</th>
                    <th>Year</th>
                    <th>Total Energy (GWh)</th>
                    <th>Renewable %</th>
                    <th>OSM Features</th>
                    <th>Power Plants</th>
                    <th>Industrial Area (km²)</th>
                  </tr>
                </thead>
                <tbody>
                  {data.length === 0 ? (
                    <tr>
                      <td colSpan="8" style={{ textAlign: 'center', padding: '20px' }}>
                        No data available for the selected filters
                      </td>
                    </tr>
                  ) : (
                    data.slice(0, 50).map((record, idx) => (
                      <tr key={`${record.region_id}-${record.year}-${idx}`}>
                        <td>{record.region_id || 'N/A'}</td>
                        <td>{record.cntr_code || 'N/A'}</td>
                        <td>{record.year || 'N/A'}</td>
                        <td>{record.total_energy_gwh ? record.total_energy_gwh.toLocaleString(undefined, { maximumFractionDigits: 2 }) : '-'}</td>
                        <td>{record.pct_renewable ? `${record.pct_renewable.toFixed(2)}%` : '-'}</td>
                        <td>{record.osm_feature_count || '-'}</td>
                        <td>{record.power_plant_count || '-'}</td>
                        <td>{record.industrial_area_km2 ? record.industrial_area_km2.toFixed(2) : '-'}</td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
              {data.length > 50 && (
                <div className="table-note">
                  Showing first 50 of {data.length} regions
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </PageTemplate>
  );
}

export default GeoEnrichmentMap;
