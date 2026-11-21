// The refusals of the backend as the window says them.

import type { EngineId } from './types';

/**
 * A refusal for want of an account of a harness, as the backend sends it (`no-account:<harness>|<text>`, `commands::err`); for
 * a project no active account serves (ESC-556), `no-account:<harness>@<project>|<text>`.
 */
const NO_ACCOUNT = /^no-account:([a-z0-9._-]+)(?:@([^|]+))?\|/;

/** What a refusal says, without the mark of a missing account. */
export function errorText(e: unknown): string {
  return String(e).replace(NO_ACCOUNT, '');
}

/** The harness a refusal wants an account of (ESC-128), if it is one for want of an account. */
export function missingAccountOf(e: unknown): EngineId | null {
  const m = String(e).match(NO_ACCOUNT);
  return m ? (m[1] as EngineId) : null;
}

/** The project a refusal for want of an account is for, when its harness has accounts but none serves it (ESC-556). */
export function missingAccountProjectOf(e: unknown): string | null {
  return String(e).match(NO_ACCOUNT)?.[2] ?? null;
}
