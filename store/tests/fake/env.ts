import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { testKey } from './minisign.mjs';

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

// The administrator of the review queue in the end-to-end tests (STORE-8): a login of the fake GitHub, whose id is
// drawn from it as the fake draws it, set in the store's STORE_ADMINS.
export const E2E_ADMIN_LOGIN = 'store-review-admin';
export const E2E_ADMIN_ID = 100_000 + (parseInt(createHash('sha256').update(E2E_ADMIN_LOGIN).digest('hex').slice(0, 8), 16) % 900_000_000);

/**
 * The signing key of the end-to-end tests: made once in the system's temporary folder (never in the repository),
 * the same for the runner and its workers; its public half is the store's STORE_KEYS_DIR during the run.
 */
export function e2eKey(): { dir: string; keys: string; secret: string } {
  const dir = join(tmpdir(), 'escouade-store-e2e-key');
  const keys = join(dir, 'keys');
  const secret = join(dir, 'store.key');
  if (!existsSync(secret)) {
    mkdirSync(keys, { recursive: true });
    const k = testKey();
    writeFileSync(join(keys, 'store.pub'), k.pub);
    writeFileSync(secret, k.secret);
  }
  return { dir, keys, secret };
}
