// Minimal test that does not depend on DOM or testing-library (CI sanity check).
test('smoke', () => {
  expect(1 + 1).toBe(2);
});
