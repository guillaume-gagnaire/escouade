import { createPrivateKey, createSign, type KeyObject } from 'node:crypto';
import type { Env } from './config';

// The GitHub App « Escouade Store » (slug escouade-store), as the user created it on 2026-10-29: sign-in by its
// user-to-server tokens (8 h), user authorization asked at installation, Contents and Metadata read-only, webhooks
// signed. Its addresses are GitHub's; GITHUB_URL and GITHUB_API_URL point the end-to-end tests at a fake GitHub
// (tests/fake/), never at the real App.

export const APP_SLUG = 'escouade-store';

export interface GithubApp {
  appId: string;
  clientId: string;
  clientSecret: string;
  privateKey: KeyObject;
  webhookSecret: string;
  web: string;
  api: string;
}

/**
 * The App's private key as Coolify holds it: pasted as is, on several lines, or on one with its line breaks escaped
 * (`\n`), in quotes or not. Null if it is not a private key.
 */
let lastKey: { raw: string; key: KeyObject | null } | null = null;

export function readPrivateKey(raw: string | undefined): KeyObject | null {
  if (lastKey && lastKey.raw === (raw ?? '')) return lastKey.key;
  const key = parseKey(raw ?? '');
  lastKey = { raw: raw ?? '', key };
  return key;
}

function parseKey(raw: string): KeyObject | null {
  let pem = (raw ?? '').trim();
  if (/^(["']).*\1$/s.test(pem)) pem = pem.slice(1, -1).trim();
  pem = pem.replace(/\\r/g, '').replace(/\\n/g, '\n').replace(/\r\n/g, '\n');
  if (!pem.includes('PRIVATE KEY-----')) return null;
  try {
    return createPrivateKey(pem);
  } catch {
    return null;
  }
}

const trimUrl = (u: string | undefined, fallback: string) => ((u ?? '').trim() || fallback).replace(/\/+$/, '');

/** The App from the environment, or null if a variable is missing or the key does not read (config.ts says which). */
export function githubApp(env: Env): GithubApp | null {
  const [appId, clientId, clientSecret, webhookSecret] = [
    'GITHUB_APP_ID',
    'GITHUB_APP_CLIENT_ID',
    'GITHUB_APP_CLIENT_SECRET',
    'GITHUB_WEBHOOK_SECRET',
  ].map((k) => (env[k] ?? '').trim());
  const privateKey = readPrivateKey(env.GITHUB_APP_PRIVATE_KEY);
  if (!appId || !clientId || !clientSecret || !webhookSecret || !privateKey) return null;
  return {
    appId,
    clientId,
    clientSecret,
    privateKey,
    webhookSecret,
    web: trimUrl(env.GITHUB_URL, 'https://github.com'),
    api: trimUrl(env.GITHUB_API_URL, 'https://api.github.com'),
  };
}

const b64 = (v: object | Buffer) => (Buffer.isBuffer(v) ? v : Buffer.from(JSON.stringify(v))).toString('base64url');

/** The App's own token (a JWT signed RS256 by its key, 9 minutes), to ask GitHub for an installation's token. */
export function appJwt(app: Pick<GithubApp, 'appId' | 'privateKey'>, now = Date.now()): string {
  const iat = Math.floor(now / 1000) - 60;
  const head = `${b64({ alg: 'RS256', typ: 'JWT' })}.${b64({ iat, exp: iat + 600, iss: app.appId })}`;
  const sign = createSign('RSA-SHA256');
  sign.update(head);
  return `${head}.${b64(sign.sign(app.privateKey))}`;
}

/** Where the sign-in sends the browser: GitHub's authorization of the App, with the state that comes back. */
export function authorizeUrl(app: GithubApp, redirectUri: string, state: string): string {
  const q = new URLSearchParams({ client_id: app.clientId, redirect_uri: redirectUri, state });
  return `${app.web}/login/oauth/authorize?${q}`;
}

/** Where installing the App starts; GitHub hands the state back with the authorization asked at installation. */
export const installUrl = (app: GithubApp, state: string) =>
  `${app.web}/apps/${APP_SLUG}/installations/new?${new URLSearchParams({ state })}`;

export class GithubError extends Error {
  constructor(
    message: string,
    readonly status?: number,
  ) {
    super(message);
  }
}

async function call<T>(url: string, init: RequestInit): Promise<T> {
  const res = await fetch(url, {
    ...init,
    headers: {
      accept: 'application/vnd.github+json',
      'user-agent': 'escouade-store',
      'x-github-api-version': '2022-11-28',
      ...init.headers,
    },
    signal: AbortSignal.timeout(15_000),
  });
  if (!res.ok) throw new GithubError(`GitHub answered ${res.status} to ${new URL(url).pathname}`, res.status);
  return (res.status === 204 ? undefined : await res.json()) as T;
}

export interface UserToken {
  token: string;
  /** When GitHub lets it go (8 h for the App's tokens), or null for a token that does not expire. */
  expiresAt: Date | null;
}

/** Trades the code GitHub sent back for the user's token. The refresh token is not kept: a session ends with it. */
export async function exchangeCode(app: GithubApp, code: string, redirectUri: string, now = Date.now()): Promise<UserToken> {
  const body = await call<{ access_token?: string; expires_in?: number; error?: string }>(`${app.web}/login/oauth/access_token`, {
    method: 'POST',
    // GitHub answers this endpoint in JSON only when asked for application/json (form-encoded otherwise).
    headers: { accept: 'application/json', 'content-type': 'application/json' },
    body: JSON.stringify({ client_id: app.clientId, client_secret: app.clientSecret, code, redirect_uri: redirectUri }),
  });
  if (!body.access_token) throw new GithubError(`GitHub refused the code (${body.error ?? 'no token'})`);
  return { token: body.access_token, expiresAt: body.expires_in ? new Date(now + body.expires_in * 1000) : null };
}

const bearer = (token: string) => ({ authorization: `Bearer ${token}` });

export interface GithubUser {
  id: number;
  login: string;
  name: string | null;
  avatar_url: string | null;
}

/** Who the token belongs to. */
export const getUser = (app: GithubApp, token: string) => call<GithubUser>(`${app.api}/user`, { headers: bearer(token) });

export interface GithubInstallation {
  id: number;
  account: { id: number; login: string; type: string };
  repository_selection?: string;
  suspended_at?: string | null;
}

export interface GithubRepo {
  id: number;
  full_name: string;
  private: boolean;
}

/** The App's installations the user can reach (they administer the account or organization), with their repositories. */
export async function userInstallations(
  app: GithubApp,
  token: string,
): Promise<{ installation: GithubInstallation; repos: GithubRepo[] }[]> {
  const { installations } = await call<{ installations: GithubInstallation[] }>(`${app.api}/user/installations?per_page=100`, {
    headers: bearer(token),
  });
  return Promise.all(
    installations.map(async (installation) => {
      const { repositories } = await call<{ repositories: GithubRepo[] }>(
        `${app.api}/user/installations/${installation.id}/repositories?per_page=100`,
        { headers: bearer(token) },
      );
      return { installation, repos: repositories };
    }),
  );
}

/** Revokes the user's token at sign-out or deletion; best effort, a failure is only logged. */
export async function revokeToken(app: GithubApp, token: string): Promise<void> {
  try {
    await call<void>(`${app.api}/applications/${encodeURIComponent(app.clientId)}/token`, {
      method: 'DELETE',
      headers: {
        authorization: `Basic ${Buffer.from(`${app.clientId}:${app.clientSecret}`).toString('base64')}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify({ access_token: token }),
    });
  } catch (e) {
    console.warn('[store] github: token not revoked:', e instanceof Error ? e.message : e);
  }
}
