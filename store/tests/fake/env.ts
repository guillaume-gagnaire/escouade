// The ports and the App's test values, shared by playwright.config.ts and the specs. None is a secret: the fake
// GitHub knows them, and the real App's live in Coolify only.
export const STORE_PORT = Number(process.env.STORE_PORT ?? 3000);
export const FAKE_GITHUB_PORT = Number(process.env.FAKE_GITHUB_PORT ?? 3010);
export const FAKE_SMTP_PORT = Number(process.env.FAKE_SMTP_PORT ?? 3025);
export const SITE = `http://localhost:${STORE_PORT}`;
export const FAKE_GITHUB = `http://127.0.0.1:${FAKE_GITHUB_PORT}`;

export const E2E_APP = {
  GITHUB_APP_ID: '1',
  GITHUB_APP_CLIENT_ID: 'Iv23liFakeStore',
  GITHUB_APP_CLIENT_SECRET: 'e2e-client-secret',
  GITHUB_WEBHOOK_SECRET: 'e2e-webhook-secret',
};
