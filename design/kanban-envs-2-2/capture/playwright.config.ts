import { defineConfig } from '@playwright/test';

// The captures « après » of ESC-390, taken in the real app with the tooling of the e2e tests:
// `npx playwright test -c design/kanban-envs-2-2/capture/playwright.config.ts` (ESCOUADE_E2E_EXE for a debug build made
// elsewhere). They go to design/kanban-envs-2-2/apres/.
/** The demo's projects and the app's data at a path that tells nothing of the machine (as the site's screenshots). */
process.env.ESCOUADE_E2E_ROOT ??= 'C:\\escouade-390';

export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.ts',
  globalSetup: '../../../tests/e2e/global-tmp.ts',
  timeout: 600_000,
  expect: { timeout: 30_000 },
  workers: 1,
  reporter: 'list',
});
