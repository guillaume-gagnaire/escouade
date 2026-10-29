import { createError, deleteCookie, getCookie, setCookie, type H3Event } from 'h3';
import { sessionAccount, SESSION_DAYS, type Account } from './accounts';
import type { Db } from './db';
import { githubApp, type GithubApp } from './github';
import { service } from './service';

// The session's cookie and what a route needs to act for an author. One cookie, strictly needed (docs/STORE.md,
// §2.3): `__Host-` (no domain, the whole site, HTTPS only), HttpOnly, SameSite=Lax. The sign-in's state rides in a
// second one for ten minutes, the time of a trip to GitHub.

export const SESSION_COOKIE = '__Host-store_session';
export const STATE_COOKIE = '__Host-store_state';

const cookie = { httpOnly: true, secure: true, sameSite: 'lax', path: '/' } as const;

export function setSessionCookie(event: H3Event, token: string): void {
  setCookie(event, SESSION_COOKIE, token, { ...cookie, maxAge: SESSION_DAYS * 86_400 });
}

export const clearSessionCookie = (event: H3Event) => deleteCookie(event, SESSION_COOKIE, cookie);

export const sessionToken = (event: H3Event) => getCookie(event, SESSION_COOKIE);

/** Where the store is reached from outside (NUXT_PUBLIC_SITE_URL), without its last slash. */
export const siteUrl = () => String(useRuntimeConfig().public.siteUrl).replace(/\/+$/, '');

/** The address GitHub sends the browser back to. */
export const callbackUrl = () => `${siteUrl()}/api/auth/github/callback`;

/** A page of the store to come back to: a path of this site, never another site's address. */
export function localPath(raw: unknown, fallback = '/account'): string {
  return typeof raw === 'string' && /^\/(?![/\\])[\w\-./?=&%]*$/.test(raw) ? raw : fallback;
}

export interface Ready {
  db: Db;
  app: GithubApp;
  secret: string;
}

/** The base, the App and the session secret, or a 503 that says which part of the store is off. */
export function accountsReady(): Ready {
  const secret = (process.env.NUXT_SESSION_SECRET ?? '').trim();
  const app = githubApp(process.env);
  if (!service.db || !secret || !app) throw createError({ statusCode: 503, statusMessage: 'Signing in is off on this store' });
  return { db: service.db, app, secret };
}

/** The account of the session, or null. */
export async function currentAccount(event: H3Event): Promise<Account | null> {
  if (!service.db) return null;
  return sessionAccount(service.db, sessionToken(event));
}

/** The account of the session, or a 401. */
export async function requireAccount(event: H3Event): Promise<{ account: Account } & Ready> {
  const ready = accountsReady();
  const account = await sessionAccount(ready.db, sessionToken(event));
  if (!account) throw createError({ statusCode: 401, statusMessage: 'Not signed in' });
  return { account, ...ready };
}
