import { generateKeyPairSync, randomBytes } from 'node:crypto';
import { defineConfig, devices } from '@playwright/test';
import {
  e2eKey,
  E2E_ADMIN_ID,
  E2E_APP as app,
  FAKE_GITHUB as fakeGithub,
  FAKE_GITHUB_PORT as githubPort,
  FAKE_SMTP_PORT as smtpPort,
  SITE as site,
  STORE_PORT as port,
} from './tests/fake/env';

// The store's end-to-end tests, on the built server (`npm run build` first): `node .output/server/index.mjs` on
// STORE_PORT (3000 by default), with the database of DATABASE_URL if there is one. The accounts go through a fake
// GitHub and a fake SMTP server (tests/fake/github.mjs, on FAKE_GITHUB_PORT and FAKE_SMTP_PORT), never the real App:
// its key is made here for the run, and its secrets are the tests' own (tests/fake/env.ts).

export default defineConfig({
  testDir: 'tests/e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: { baseURL: site, ...devices['Desktop Chrome'] },
  webServer: [
    {
      command: 'node tests/fake/github.mjs',
      url: `${fakeGithub}/_fake/state`,
      reuseExistingServer: !process.env.CI,
      env: { ...app, FAKE_GITHUB_PORT: String(githubPort), FAKE_SMTP_PORT: String(smtpPort), STORE_URL: site },
      stdout: 'pipe',
    },
    {
      command: 'node .output/server/index.mjs',
      url: `${site}/`,
      reuseExistingServer: !process.env.CI,
      env: {
        ...app,
        PORT: String(port),
        ARCHIVE_DIR: process.env.ARCHIVE_DIR ?? './.data/archives',
        NUXT_PUBLIC_SITE_URL: site,
        NUXT_SESSION_SECRET: randomBytes(32).toString('hex'),
        GITHUB_APP_PRIVATE_KEY: generateKeyPairSync('rsa', { modulusLength: 2048 })
          .privateKey.export({ type: 'pkcs1', format: 'pem' })
          .toString(),
        GITHUB_URL: fakeGithub,
        GITHUB_API_URL: fakeGithub,
        SMTP_URL: `smtp://127.0.0.1:${smtpPort}`,
        MAIL_FROM: 'Escouade Store <store@escouade.dev>',
        // The review queue (STORE-8): its administrator, and the public key of a key made for the run.
        STORE_ADMINS: String(E2E_ADMIN_ID),
        STORE_KEYS_DIR: e2eKey().keys,
      },
      stdout: 'pipe',
    },
  ],
});
