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
// - Google Play, tablets (9:16, each side between 1080 and 7680): 7" (1080 × 1920, 720 × 1280 at 1.5×) and 10"
//   (1800 × 3200, 900 × 1600 at 2×); the phone's page, centred, 600 px at most (M10), in portrait as the app.
// - App Store, iPhone 6.9" (1290 × 2796, 430 × 932 at 3×) and 6.5" (1242 × 2688, 414 × 896 at 3×). No iPad (M10).
const ios = devices['iPhone 15 Pro Max'].userAgent;
const phone = { isMobile: true, hasTouch: true };
const tablet = devices['Galaxy Tab S4'].userAgent;
const sizes = [
  { name: 'play', width: 360, height: 720, scale: 3, userAgent: devices['Pixel 7'].userAgent },
  { name: 'play-tablet-7', width: 720, height: 1280, scale: 1.5, userAgent: tablet },
  { name: 'play-tablet-10', width: 900, height: 1600, scale: 2, userAgent: tablet },
  { name: 'appstore-6.9', width: 430, height: 932, scale: 3, userAgent: ios },
  { name: 'appstore-6.5', width: 414, height: 896, scale: 3, userAgent: ios },
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
        deviceScaleFactor: s.scale,
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
