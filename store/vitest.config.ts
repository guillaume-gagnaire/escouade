import { defineConfig } from 'vitest/config';

// The store's own tests: the unit tests always, the database's when TEST_DATABASE_URL names one (they skip otherwise).
export default defineConfig({
  test: {
    include: ['tests/unit/**/*.test.ts', 'tests/db/**/*.test.ts'],
    environment: 'node',
    // The database's tests share one base: one file at a time.
    fileParallelism: false,
    testTimeout: 20_000,
  },
});
