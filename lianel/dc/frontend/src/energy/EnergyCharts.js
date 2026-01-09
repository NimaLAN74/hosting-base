import React from 'react';
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer
} from 'recharts';

const COLORS = ['#0088FE', '#00C49F', '#FFBB28', '#FF8042', '#8884d8', '#82ca9d', '#ffc658', '#ff7300'];

// Time Series Chart - Energy consumption over years
export const TimeSeriesChart = ({ data, countryCode }) => {
  if (!data || !data.data || data.data.length === 0) return null;

  // Group by year and sum values
  const yearData = data.data.reduce((acc, record) => {
    const year = record.year;
    if (!acc[year]) {
      acc[year] = { year, total: 0, count: 0 };
    }
    acc[year].total += parseFloat(record.value_gwh) || 0;
    acc[year].count += 1;
    return acc;
  }, {});

  const chartData = Object.values(yearData)
    .sort((a, b) => a.year - b.year)
    .map(item => ({
      year: item.year.toString(),
      'Total Energy (GWh)': Math.round(item.total * 100) / 100
    }));

  return (
    <div className="chart-container">
      <h3>Energy Consumption Over Time{countryCode ? ` - ${countryCode}` : ''}</h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="year" />
          <YAxis />
          <Tooltip formatter={(value) => `${value.toLocaleString()} GWh`} />
          <Legend />
          <Line 
            type="monotone" 
            dataKey="Total Energy (GWh)" 
            stroke="#8884d8" 
            strokeWidth={2}
            dot={{ r: 4 }}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
};

// Country Comparison Chart - Top countries by energy consumption
export const CountryComparisonChart = ({ summary }) => {
  if (!summary || !summary.summary || summary.summary.length === 0) return null;

  const chartData = summary.summary
    .slice(0, 10)
    .map(item => ({
      country: item.group,
      'Total Energy (GWh)': Math.round(parseFloat(item.total_gwh) * 100) / 100
    }));

  return (
    <div className="chart-container">
      <h3>Top Countries by Energy Consumption</h3>
      <ResponsiveContainer width="100%" height={300}>
        <BarChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="country" />
          <YAxis />
          <Tooltip formatter={(value) => `${value.toLocaleString()} GWh`} />
          <Legend />
          <Bar dataKey="Total Energy (GWh)" fill="#8884d8" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
};

// Product Distribution Chart - Energy by product type
export const ProductDistributionChart = ({ data }) => {
  if (!data || !data.data || data.data.length === 0) return null;

  // Group by product
  const productData = data.data.reduce((acc, record) => {
    const product = record.product_name || record.product_code || 'Unknown';
    if (!acc[product]) {
      acc[product] = 0;
    }
    acc[product] += parseFloat(record.value_gwh) || 0;
    return acc;
  }, {});

  const chartData = Object.entries(productData)
    .map(([name, value]) => ({
      name: name.length > 20 ? name.substring(0, 20) + '...' : name,
      value: Math.round(value * 100) / 100
    }))
    .sort((a, b) => b.value - a.value)
    .slice(0, 8);

  return (
    <div className="chart-container">
      <h3>Energy Distribution by Product Type</h3>
      <ResponsiveContainer width="100%" height={300}>
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={({ name, percent }) => `${name}: ${(percent * 100).toFixed(0)}%`}
            outerRadius={80}
            fill="#8884d8"
            dataKey="value"
          >
            {chartData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip formatter={(value) => `${value.toLocaleString()} GWh`} />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
};

// Flow Distribution Chart - Energy by flow type
export const FlowDistributionChart = ({ data }) => {
  if (!data || !data.data || data.data.length === 0) return null;

  // Group by flow
  const flowData = data.data.reduce((acc, record) => {
    const flow = record.flow_name || record.flow_code || 'Unknown';
    if (!acc[flow]) {
      acc[flow] = 0;
    }
    acc[flow] += parseFloat(record.value_gwh) || 0;
    return acc;
  }, {});

  const chartData = Object.entries(flowData)
    .map(([name, value]) => ({
      name: name.length > 20 ? name.substring(0, 20) + '...' : name,
      value: Math.round(value * 100) / 100
    }))
    .sort((a, b) => b.value - a.value)
    .slice(0, 8);

  return (
    <div className="chart-container">
      <h3>Energy Distribution by Flow Type</h3>
      <ResponsiveContainer width="100%" height={300}>
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={({ name, percent }) => `${name}: ${(percent * 100).toFixed(0)}%`}
            outerRadius={80}
            fill="#8884d8"
            dataKey="value"
          >
            {chartData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip formatter={(value) => `${value.toLocaleString()} GWh`} />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
};
