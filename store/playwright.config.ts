import { defineConfig, devices } from '@playwright/test';

// The store's end-to-end tests, on the built server (`npm run build` first): `node .output/server/index.mjs` on
// STORE_PORT (3000 by default), with the database of DATABASE_URL if there is one.
const port = Number(process.env.STORE_PORT ?? 3000);

export default defineConfig({
  testDir: 'tests/e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: { baseURL: `http://localhost:${port}`, ...devices['Desktop Chrome'] },
  webServer: {
    command: 'node .output/server/index.mjs',
    url: `http://localhost:${port}/`,
    reuseExistingServer: !process.env.CI,
    env: { PORT: String(port), ARCHIVE_DIR: process.env.ARCHIVE_DIR ?? './.data/archives' },
    stdout: 'pipe',
  },
});
