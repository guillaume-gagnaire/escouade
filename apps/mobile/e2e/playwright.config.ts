import { defineConfig, devices } from '@playwright/test';
import { PAGE_PORT } from './harness';

// The phone's e2e (MO-1): the app's page built (apps/mobile/dist) and served by Vite's preview, in Chromium at the two
// sizes of the design (design/mobile-2-3: 360 × 780, an Android; 430 × 932, an iPhone), against an `escouade-server` of
// each test (harness.ts). From the repository's root, after `cargo build --bin escouade-server`:
//
//   npx playwright test -c apps/mobile/e2e/playwright.config.ts
//
// ESCOUADE_PORT_BASE gives the worktree's ports (the page on base + 5). ESCOUADE_MOBILE_SHOTS=1 writes the captures of
// the tabs in design/mobile-2-3/app/.
const phone = { isMobile: true, hasTouch: true, deviceScaleFactor: 2 };

export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.ts',
  timeout: 90_000,
  expect: { timeout: 15_000 },
  workers: 1,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  use: { baseURL: `http://127.0.0.1:${PAGE_PORT}`, trace: process.env.CI ? 'on-first-retry' : 'retain-on-failure', locale: 'fr-FR' },
  projects: [
    {
      name: 'android-360',
      use: { ...devices['Pixel 7'], ...phone, viewport: { width: 360, height: 780 }, screen: { width: 360, height: 780 } },
    },
    {
      name: 'ios-430',
      // An iPhone's size and user agent, in Chromium (the CI's and the machine's browser).
      use: {
        userAgent: devices['iPhone 15 Pro Max'].userAgent,
        ...phone,
        viewport: { width: 430, height: 932 },
        screen: { width: 430, height: 932 },
      },
    },
  ],
  webServer: {
    command: 'node ../../node_modules/vite/bin/vite.js build && node ../../node_modules/vite/bin/vite.js preview',
    cwd: '..',
    env: { PORT: String(PAGE_PORT) },
    url: `http://127.0.0.1:${PAGE_PORT}`,
    // Always the page of the code under test, built again.
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
