import { defineConfig } from '@playwright/test';

// The captures of ESC-561 (the automatic fetch: its settings and its line in the Git panel), taken in the real app with
// the tooling of the e2e tests: `npx playwright test -c design/auto-fetch/capture/playwright.config.ts`
// (ESCOUADE_E2E_EXE for a debug build made elsewhere). They go to design/auto-fetch/captures/.
/** The demo's projects and the app's data at a path that tells nothing of the machine (as the site's screenshots). */
process.env.ESCOUADE_E2E_ROOT ??= 'C:\\escouade-561';

export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.ts',
  globalSetup: '../../../tests/e2e/global-tmp.ts',
  timeout: 600_000,
  expect: { timeout: 30_000 },
  workers: 1,
  reporter: 'list',
});
