import { defineConfig } from '@playwright/test';

// The captures of the settings as they are on main (ESC-375), taken in the real app with the tooling of the e2e tests:
// `npx playwright test -c design/settings-2-2/capture/playwright.config.ts` (ESCOUADE_E2E_EXE for a debug build made
// elsewhere). They go to design/settings-2-2/existant/, with the measures of each tab in mesures.json.
/** The demo's projects and the app's data at a path that tells nothing of the machine (as the site's screenshots). */
process.env.ESCOUADE_E2E_ROOT ??= 'C:\\escouade-375';

export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.ts',
  globalSetup: '../../../tests/e2e/global-tmp.ts',
  timeout: 600_000,
  expect: { timeout: 30_000 },
  workers: 1,
  reporter: 'list',
});
