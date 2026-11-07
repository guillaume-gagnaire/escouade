import { defineConfig } from '@playwright/test';

// The previews of ESC-65 (README.md here): the real app of the end-to-end tests, its window drawn with each direction of
// the « Pull requests » view. `npx playwright test -c design/esc-65/playwright.config.ts`, after
// `npx tauri build --debug --no-bundle`.
export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.ts',
  globalSetup: '../../tests/e2e/global-tmp.ts',
  timeout: 600_000,
  expect: { timeout: 30_000 },
  workers: 1,
  reporter: 'list',
});
