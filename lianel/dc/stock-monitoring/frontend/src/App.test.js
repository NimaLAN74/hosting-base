import React from 'react';
import { render, screen } from '@testing-library/react';
import App from './App';

test('App renders and shows Stock Exchange Monitoring', () => {
  render(<App />);
  const heading = screen.getByRole('heading', { name: /Stock Exchange Monitoring/i });
  expect(heading).toBeInTheDocument();
});

test('App shows EU markets MVP', () => {
  render(<App />);
  expect(screen.getByText(/EU markets MVP/i)).toBeInTheDocument();
});
