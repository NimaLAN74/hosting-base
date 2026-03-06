// Jest setup for react-scripts (CI and local)
import '@testing-library/jest-dom';

beforeEach(() => {
  global.fetch = jest.fn(async (input) => {
    const url = String(input || '');
    if (url.includes('/health')) {
      return { ok: true, status: 200, text: async () => 'ok', headers: { get: () => 'text/plain' } };
    }
    if (url.includes('/status')) {
      return {
        ok: true,
        status: 200,
        json: async () => ({ service: 'stock-monitoring', version: 'test', database: 'connected' }),
        headers: { get: () => 'application/json' },
      };
    }
    if (url.includes('/me')) {
      return {
        ok: true,
        status: 200,
        json: async () => ({ user_id: 'test-user' }),
        headers: { get: () => 'application/json' },
      };
    }
    return {
      ok: true,
      status: 200,
      json: async () => [],
      text: async () => 'ok',
      headers: { get: () => 'application/json' },
    };
  });
});

afterEach(() => {
  jest.clearAllMocks();
});
