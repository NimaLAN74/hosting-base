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
    const res = await authenticatedFetch('/api/energy/api/info');
    if (!res.ok) throw new Error('Failed to fetch service info');
    return res.json();
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

    const res = await authenticatedFetch(`/api/energy/annual${buildQuery(params)}`);
    if (!res.ok) throw new Error('Failed to fetch energy data');
    return res.json();
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

    const res = await authenticatedFetch(`/api/energy/annual/summary${buildQuery(params)}`);
    if (!res.ok) throw new Error('Failed to fetch energy summary');
    return res.json();
  }
};
