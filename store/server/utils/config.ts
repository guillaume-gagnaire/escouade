// What the store reads from its environment (Coolify's variables, store/README.md). Nothing secret is in the
// repository: a variable that is missing turns its feature off and says so in the log, the server still starts.

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
