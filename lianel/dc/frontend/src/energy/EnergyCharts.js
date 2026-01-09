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

// Enhanced color palette for better visualization
const COLORS = [
  '#2c5aa0', // Primary blue
  '#4a8fe7', // Secondary blue
  '#f6c445', // Accent gold
  '#00C49F', // Teal
  '#FF8042', // Orange
  '#8884d8', // Purple
  '#82ca9d', // Green
  '#ffc658', // Yellow
  '#ff7300', // Red-orange
  '#0088FE', // Bright blue
  '#9c27b0', // Deep purple
  '#e91e63', // Pink
  '#00bcd4', // Cyan
  '#4caf50', // Green
  '#ff9800', // Orange
  '#795548'  // Brown
];

// Time Series Chart - Energy consumption over years
export const TimeSeriesChart = ({ data, countryCode, countryCodes }) => {
  if (!data || !data.data || data.data.length === 0) return null;

  // Filter by selected countries if multiple countries selected
  let filteredData = data.data;
  if (countryCodes && countryCodes.length > 1) {
    filteredData = data.data.filter(record => 
      countryCodes.includes(record.country_code)
    );
  } else if (countryCode) {
    filteredData = data.data.filter(record => record.country_code === countryCode);
  }

  // Group by year and sum values
  const yearData = filteredData.reduce((acc, record) => {
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

  const chartTitle = countryCodes && countryCodes.length > 1 
    ? `Energy Consumption Over Time - ${countryCodes.join(', ')}`
    : countryCode 
      ? `Energy Consumption Over Time - ${countryCode}`
      : 'Energy Consumption Over Time';

  return (
    <div className="chart-container">
      <h3>{chartTitle}</h3>
      <ResponsiveContainer width="100%" height={350}>
        <LineChart data={chartData} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="rgba(255, 255, 255, 0.1)" />
          <XAxis 
            dataKey="year" 
            stroke="var(--text-secondary)"
            style={{ fontSize: '12px' }}
          />
          <YAxis 
            stroke="var(--text-secondary)"
            style={{ fontSize: '12px' }}
            tickFormatter={(value) => `${(value / 1000).toFixed(0)}k`}
          />
          <Tooltip 
            formatter={(value) => [`${value.toLocaleString()} GWh`, 'Total Energy']}
            contentStyle={{
              backgroundColor: 'rgba(15, 23, 42, 0.95)',
              border: '1px solid var(--border-color)',
              borderRadius: '8px',
              color: 'var(--text-primary)'
            }}
            labelStyle={{ color: 'var(--text-primary)' }}
          />
          <Legend 
            wrapperStyle={{ color: 'var(--text-primary)' }}
          />
          <Line 
            type="monotone" 
            dataKey="Total Energy (GWh)" 
            stroke={COLORS[0]} 
            strokeWidth={3}
            dot={{ r: 5, fill: COLORS[0] }}
            activeDot={{ r: 7 }}
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
      <ResponsiveContainer width="100%" height={350}>
        <BarChart data={chartData} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="rgba(255, 255, 255, 0.1)" />
          <XAxis 
            dataKey="country" 
            stroke="var(--text-secondary)"
            style={{ fontSize: '12px' }}
            angle={-45}
            textAnchor="end"
            height={80}
          />
          <YAxis 
            stroke="var(--text-secondary)"
            style={{ fontSize: '12px' }}
            tickFormatter={(value) => `${(value / 1000).toFixed(0)}k`}
          />
          <Tooltip 
            formatter={(value) => [`${value.toLocaleString()} GWh`, 'Total Energy']}
            contentStyle={{
              backgroundColor: 'rgba(15, 23, 42, 0.95)',
              border: '1px solid var(--border-color)',
              borderRadius: '8px',
              color: 'var(--text-primary)'
            }}
            labelStyle={{ color: 'var(--text-primary)' }}
          />
          <Legend 
            wrapperStyle={{ color: 'var(--text-primary)' }}
          />
          <Bar 
            dataKey="Total Energy (GWh)" 
            fill={COLORS[1]}
            radius={[8, 8, 0, 0]}
          >
            {chartData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Bar>
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
      <ResponsiveContainer width="100%" height={350}>
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={({ name, percent }) => percent > 0.05 ? `${name}: ${(percent * 100).toFixed(0)}%` : ''}
            outerRadius={100}
            innerRadius={40}
            fill="#8884d8"
            dataKey="value"
            paddingAngle={2}
          >
            {chartData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip 
            formatter={(value, name, props) => [
              `${value.toLocaleString()} GWh (${(props.payload.percent * 100).toFixed(1)}%)`,
              props.payload.name
            ]}
            contentStyle={{
              backgroundColor: 'rgba(15, 23, 42, 0.95)',
              border: '1px solid var(--border-color)',
              borderRadius: '8px',
              color: 'var(--text-primary)'
            }}
          />
          <Legend 
            wrapperStyle={{ color: 'var(--text-primary)' }}
            formatter={(value, entry) => `${entry.payload.name}: ${(entry.payload.percent * 100).toFixed(1)}%`}
          />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
};

// Flow Distribution Chart - Energy by flow type
export const FlowDistributionChart = ({ data }) => {
  if (!data || !data.data || data.data.length === 0) return null;

  // Use all data (already filtered by country/year selection)
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
      <ResponsiveContainer width="100%" height={350}>
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={({ name, percent }) => percent > 0.05 ? `${name}: ${(percent * 100).toFixed(0)}%` : ''}
            outerRadius={100}
            innerRadius={40}
            fill="#8884d8"
            dataKey="value"
            paddingAngle={2}
          >
            {chartData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[(index + 4) % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip 
            formatter={(value, name, props) => [
              `${value.toLocaleString()} GWh (${(props.payload.percent * 100).toFixed(1)}%)`,
              props.payload.name
            ]}
            contentStyle={{
              backgroundColor: 'rgba(15, 23, 42, 0.95)',
              border: '1px solid var(--border-color)',
              borderRadius: '8px',
              color: 'var(--text-primary)'
            }}
          />
          <Legend 
            wrapperStyle={{ color: 'var(--text-primary)' }}
            formatter={(value, entry) => `${entry.payload.name}: ${(entry.payload.percent * 100).toFixed(1)}%`}
          />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
};
