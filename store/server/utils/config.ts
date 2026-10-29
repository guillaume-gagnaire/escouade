// What the store reads from its environment (Coolify's variables, store/README.md). Nothing secret is in the
// repository: a variable that is missing turns its feature off and says so in the log, the server still starts.

import { readPrivateKey } from './github';

export type Env = Record<string, string | undefined>;

/** A feature and the variables it needs, all of them. */
export interface Feature {
  name: string;
  vars: readonly string[];
  /** What the store does without it, for the log. */
  without: string;
}

export const FEATURES: readonly Feature[] = [
  {
    name: 'database',
    vars: ['DATABASE_URL'],
    without: 'no database: /api/health answers 503 and nothing is stored',
  },
  {
    name: 'sessions',
    vars: ['NUXT_SESSION_SECRET'],
    without: 'nobody can sign in',
  },
  {
    name: 'github',
    vars: ['GITHUB_APP_ID', 'GITHUB_APP_CLIENT_ID', 'GITHUB_APP_CLIENT_SECRET', 'GITHUB_APP_PRIVATE_KEY', 'GITHUB_WEBHOOK_SECRET'],
    without: 'signing in with GitHub, submissions and webhooks are off',
  },
  {
    name: 'mail',
    vars: ['SMTP_URL', 'MAIL_FROM'],
    without: 'no e-mail is sent',
  },
  {
    name: 'review',
    vars: ['STORE_ADMINS'],
    without: 'the review queue has no administrator',
  },
];

const set = (env: Env, name: string) => (env[name] ?? '').trim() !== '';

/** The features turned off for want of a variable, each with the variables it lacks (never their values). */
export function missingFeatures(env: Env): { feature: Feature; missing: string[] }[] {
  return FEATURES.flatMap((feature) => {
    const missing = feature.vars.filter((v) => !set(env, v));
    return missing.length ? [{ feature, missing }] : [];
  });
}

/** One line per feature turned off: what the log says at start. */
export function missingLines(env: Env): string[] {
  return missingFeatures(env).map(
    ({ feature, missing }) => `[store] ${feature.name} off (missing ${missing.join(', ')}): ${feature.without}`,
  );
}

/** What is set but does not hold: the App's key that does not read, a session secret too short. */
export function invalidLines(env: Env): string[] {
  const lines: string[] = [];
  if (set(env, 'GITHUB_APP_PRIVATE_KEY') && !readPrivateKey(env.GITHUB_APP_PRIVATE_KEY))
    lines.push(
      '[store] github off (GITHUB_APP_PRIVATE_KEY does not read as a private key): signing in with GitHub, submissions and webhooks are off',
    );
  if (set(env, 'NUXT_SESSION_SECRET') && env.NUXT_SESSION_SECRET!.trim().length < 32)
    lines.push('[store] sessions: NUXT_SESSION_SECRET is shorter than 32 characters; use `openssl rand -hex 32`');
  return lines;
}

/** Where the archives live: the volume in production (/data/archives), a folder of the checkout in development. */
export const archiveDir = (env: Env) => (env.ARCHIVE_DIR ?? '').trim() || './.data/archives';

/** The administrators: GitHub's numeric ids, separated by commas or spaces. */
export function admins(env: Env): number[] {
  return (env.STORE_ADMINS ?? '')
    .split(/[\s,]+/)
    .filter(Boolean)
    .map(Number)
    .filter((n) => Number.isSafeInteger(n) && n > 0);
}

/**
 * Where the public keys of the store's signatures are read (`*.pub`, the ones the app embeds): `store/keys/` of the
 * checkout, `/app/keys` in the image. The private key is never here (docs/STORE.md, §4.5).
 */
export const keysDir = (env: Env) => (env.STORE_KEYS_DIR ?? '').trim() || './keys';
