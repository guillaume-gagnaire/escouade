import { defineConfig } from '@playwright/test';

// Le banc de l'app d'aujourd'hui pour l'étude GPUI (ESC-365) : hors des e2e de l'app, lancé à la main.
export default defineConfig({
  testDir: '.',
  testMatch: 'tauri.spec.ts',
  globalSetup: '../../../tests/e2e/global-tmp.ts',
  timeout: 30 * 60_000,
  expect: { timeout: 15_000 },
  workers: 1,
  reporter: 'list',
});
