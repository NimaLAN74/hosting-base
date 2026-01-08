import { authenticatedFetch } from '../keycloak';

const buildQuery = (params = {}) => {
  const q = new URLSearchParams();
  Object.entries(params).forEach(([k, v]) => {
    if (v !== undefined && v !== null && String(v).trim() !== '') {
      q.set(k, String(v));
    }
  });
  const qs = q.toString();
  return qs ? `?${qs}` : '';
};

export const energyApi = {
  async getHealth() {
    const res = await authenticatedFetch('/api/energy/health');
    if (!res.ok) throw new Error('Health check failed');
    return res.json();
  },

  async getServiceInfo() {
    try {
      const res = await authenticatedFetch('/api/energy/api/info');
      if (!res.ok) {
        const errorText = await res.text().catch(() => 'Unknown error');
        throw new Error(`Failed to fetch service info: ${res.status} ${res.statusText} - ${errorText}`);
      }
      return res.json();
    } catch (err) {
      if (err.message.includes('Not authenticated')) {
        throw new Error('Authentication required. Please log in again.');
      }
      throw err;
    }
  },

  async getEnergyAnnual({
    country_code,
    year,
    product_code,
    flow_code,
    source_table,
    limit = 100,
    offset = 0
  } = {}) {
    const params = { limit, offset };
    if (country_code) params.country_code = country_code;
    if (year) params.year = year;
    if (product_code) params.product_code = product_code;
    if (flow_code) params.flow_code = flow_code;
    if (source_table) params.source_table = source_table;

    try {
      const res = await authenticatedFetch(`/api/energy/annual${buildQuery(params)}`);
      if (!res.ok) {
        const errorText = await res.text().catch(() => 'Unknown error');
        throw new Error(`Failed to fetch energy data: ${res.status} ${res.statusText} - ${errorText}`);
      }
      return res.json();
    } catch (err) {
      if (err.message.includes('Not authenticated')) {
        throw new Error('Authentication required. Please log in again.');
      }
      throw err;
    }
  },

  async getEnergyByCountry(countryCode, { limit = 100, offset = 0 } = {}) {
    const res = await authenticatedFetch(
      `/api/energy/annual/by-country/${encodeURIComponent(countryCode)}${buildQuery({ limit, offset })}`
    );
    if (!res.ok) throw new Error('Failed to fetch energy data by country');
    return res.json();
  },

  async getEnergyByYear(year, { limit = 100, offset = 0 } = {}) {
    const res = await authenticatedFetch(
      `/api/energy/annual/by-year/${encodeURIComponent(year)}${buildQuery({ limit, offset })}`
    );
    if (!res.ok) throw new Error('Failed to fetch energy data by year');
    return res.json();
  },

  async getEnergySummary({ country_code, year, group_by = 'country' } = {}) {
    const params = { group_by };
    if (country_code) params.country_code = country_code;
    if (year) params.year = year;

    try {
      const res = await authenticatedFetch(`/api/energy/annual/summary${buildQuery(params)}`);
      if (!res.ok) {
        const errorText = await res.text().catch(() => 'Unknown error');
        throw new Error(`Failed to fetch energy summary: ${res.status} ${res.statusText} - ${errorText}`);
      }
      return res.json();
    } catch (err) {
      if (err.message.includes('Not authenticated')) {
        throw new Error('Authentication required. Please log in again.');
      }
      throw err;
    }
  }
};
