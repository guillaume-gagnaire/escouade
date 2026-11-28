import { defineConfig, devices } from '@playwright/test';
import { PAGE_PORT } from './harness';

// The captures of the stores (MO-8, docs/MOBILE-PUBLISH.md): the app's page against an `escouade-server` of the
// test's and the fake `claude` (fictitious data alone), at the sizes the stores ask, in French and in English, into
// apps/mobile/store/screenshots/<store>/<locale>/. From the repository's root, after
// `cargo build --bin escouade-server`:
//
//   ESCOUADE_PORT_BASE=<base> npx playwright test -c apps/mobile/e2e/store-shots.config.ts
//
// - Google Play, a phone: 1080 × 2160 (9:18, its longest side twice the shortest at most), 360 × 720 at 3×.
// - App Store, iPhone 6.9" (1290 × 2796, 430 × 932 at 3×) and 6.5" (1242 × 2688, 414 × 896 at 3×). No iPad (M10).
const ios = devices['iPhone 15 Pro Max'].userAgent;
const phone = { isMobile: true, hasTouch: true, deviceScaleFactor: 3 };
const sizes = [
  { name: 'play', width: 360, height: 720, userAgent: devices['Pixel 7'].userAgent },
  { name: 'appstore-6.9', width: 430, height: 932, userAgent: ios },
  { name: 'appstore-6.5', width: 414, height: 896, userAgent: ios },
];
const locales = ['fr-FR', 'en-US'];

export default defineConfig({
  testDir: '.',
  testMatch: 'store-shots.ts',
  timeout: 240_000,
  expect: { timeout: 15_000 },
  workers: 1,
  reporter: 'list',
  use: { baseURL: `http://127.0.0.1:${PAGE_PORT}` },
  projects: sizes.flatMap((s) =>
    locales.map((locale) => ({
      name: `${s.name}@${locale}`,
      use: {
        ...phone,
        locale,
        userAgent: s.userAgent,
        viewport: { width: s.width, height: s.height },
        screen: { width: s.width, height: s.height },
      },
    })),
  ),
  webServer: {
    command: 'node ../../node_modules/vite/bin/vite.js build && node ../../node_modules/vite/bin/vite.js preview',
    cwd: '..',
    env: { PORT: String(PAGE_PORT) },
    url: `http://127.0.0.1:${PAGE_PORT}`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
