import React from 'react';
import { render, screen } from '@testing-library/react';
import App from './App';

test('renders Stock Exchange Monitoring heading', () => {
  render(<App />);
  const heading = screen.getByText(/Stock Exchange Monitoring/i);
  expect(heading).toBeTruthy();
  expect(heading.textContent).toMatch(/Stock Exchange Monitoring/i);
});

test('renders EU markets MVP text', () => {
  render(<App />);
  const mvp = screen.getByText(/EU markets MVP/i);
  expect(mvp).toBeTruthy();
});
