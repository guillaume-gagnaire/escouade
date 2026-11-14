import { defineConfig } from '@playwright/test';

// The black-box tests of the security audit of 2026-11 (ESC-418): the real app (debug build with its interface,
// `npx tauri build --debug --no-bundle`) started by the fixture of the end-to-end tests, so on a throwaway data folder,
// with the fake harness, a hidden window, and never the user's own Escouade, its folder, its ports or its sign-ins.
//
//   ESCOUADE_E2E_CDP_PORT=<port> AUDIT_PORT_BASE=<port> npx playwright test -c docs/security/audit-2026-11/poc/blackbox
//
// AUDIT_PORT_BASE is the first of four ports the tests use themselves (the MCP server of the test instance, the fake
// update server, a decoy web page): they are given, never written here. What each test saw goes to AUDIT_OUT (a folder),
// one JSON file per test, which the report quotes.
export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.ts',
  globalSetup: '../../../../../tests/e2e/global-tmp.ts',
  timeout: 240_000,
  expect: { timeout: 20_000 },
  workers: 1,
  retries: 0,
  reporter: 'list',
});
