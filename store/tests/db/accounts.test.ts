import pg from 'pg';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import {
  accountView,
  applyWebhook,
  askEmail,
  deleteAccount,
  EMAIL_LINKS_PER_HOUR,
  exportAccount,
  purge,
  removeEmail,
  sessionAccount,
  signIn,
  signOut,
  verifyEmail,
} from '../../server/utils/accounts';
import { migrateDb, openDb, type Db } from '../../server/utils/db';
import type { GithubInstallation } from '../../server/utils/github';
import { tokenHash } from '../../server/utils/tokens';

// The accounts against a real PostgreSQL, as tests/db/migrations.test.ts: the base of TEST_DATABASE_URL alone,
// emptied first. Without it, skipped.
const url = process.env.TEST_DATABASE_URL;
const SECRET = 's'.repeat(64);
const T0 = new Date('2026-10-29T10:00:00Z');
const later = (ms: number) => new Date(T0.getTime() + ms);
const HOUR = 3600_000;
const DAY = 24 * HOUR;

const lea = { id: 9001, login: 'lea-martin', name: 'Léa Martin', avatar_url: 'https://avatars.githubusercontent.com/u/9001' };
const leaInstall: GithubInstallation = {
  id: 501,
  account: { id: 9001, login: 'lea-martin', type: 'User' },
  repository_selection: 'selected',
};
const repo = (id: number, full_name: string) => ({ id, full_name, private: false });

describe.skipIf(!url)('the accounts', () => {
  let pool: pg.Pool;
  let db: Db;
  const q = async (sql: string, params: unknown[] = []) => (await pool.query(sql, params)).rows;

  beforeAll(async () => {
    const admin = new pg.Client({ connectionString: url });
    await admin.connect();
    await admin.query(
      'drop schema if exists drizzle cascade; drop schema if exists pgboss cascade; drop schema public cascade; create schema public;',
    );
    await admin.end();
    ({ pool, db } = openDb(url!));
    await migrateDb(db, './server/db/migrations');
  });
  beforeEach(async () => {
    await pool.query(
      'truncate accounts, sessions, email_tokens, installations, installation_repos, account_installations, webhook_deliveries, audit cascade',
    );
  });
  afterAll(async () => {
    await pool?.end();
  });

  const signLea = (now = T0, expiresIn: number | null = 8 * HOUR) =>
    signIn(
      db,
      {
        user: lea,
        token: { token: 'ghu_lea', expiresAt: expiresIn === null ? null : new Date(now.getTime() + expiresIn) },
        installations: [{ installation: leaInstall, repos: [repo(1, 'lea-martin/escouade-slack')] }],
      },
      SECRET,
      now,
    );

  it('signs in: the account by its GitHub id, a session of 30 days whose base keeps only the hash', async () => {
    const { account, session } = await signLea();
    expect(account).toMatchObject({ githubId: 9001, login: 'lea-martin', name: 'Léa Martin', email: null });
    const [row] = await q('select * from sessions');
    expect(row.id).toBe(tokenHash(session));
    expect(row.id).not.toBe(session);
    expect(row.expires_at.toISOString()).toBe(later(30 * DAY).toISOString());
    expect(row.github_token).not.toContain('ghu_lea');
    expect(await sessionAccount(db, session, later(29 * DAY))).toMatchObject({ login: 'lea-martin' });
    expect(await sessionAccount(db, session, later(30 * DAY + 1))).toBeNull();
    expect(await sessionAccount(db, 'not-a-session', T0)).toBeNull();
    expect(await sessionAccount(db, undefined, T0)).toBeNull();
  });

  it('keeps the GitHub token 8 hours at most, whatever GitHub says', async () => {
    await signLea(T0, null);
    await signLea(T0, 24 * HOUR);
    await signLea(T0, 2 * HOUR);
    const rows = await q('select github_token_expires_at as e from sessions order by e');
    expect(rows.map((r) => r.e.toISOString())).toEqual([later(2 * HOUR), later(8 * HOUR), later(8 * HOUR)].map((d) => d.toISOString()));
  });

  it('follows the login GitHub gives at the next sign-in, on the same account', async () => {
    const first = await signLea();
    const again = await signIn(
      db,
      { user: { ...lea, login: 'lea-m' }, token: { token: 't', expiresAt: null }, installations: [] },
      SECRET,
      later(DAY),
    );
    expect(again.account.id).toBe(first.account.id);
    expect(again.account.login).toBe('lea-m');
    // The installations are GitHub's list at sign-in: an empty list unlinks them.
    expect((await accountView(db, again.account)).installations).toEqual([]);
  });

  it('shows the installations the account manages, with their repositories', async () => {
    const { account } = await signLea();
    expect((await accountView(db, account, T0)).installations).toEqual([
      { id: 501, owner: 'lea-martin', ownerType: 'User', all: false, suspended: false, repos: ['lea-martin/escouade-slack'] },
    ]);
  });

  it('signs out: the session goes, the valid token comes back for GitHub to revoke it', async () => {
    const { session } = await signLea();
    expect(await signOut(db, session, SECRET, later(HOUR))).toBe('ghu_lea');
    expect(await sessionAccount(db, session, later(HOUR))).toBeNull();
    expect(await signOut(db, session, SECRET, later(HOUR))).toBeNull();
    const { session: old } = await signLea();
    expect(await signOut(db, old, SECRET, later(9 * HOUR))).toBeNull();
    const actions = (await q('select action from audit order by id')).map((r) => r.action);
    expect(actions).toEqual(['account.signed_in', 'account.signed_out', 'account.signed_in', 'account.signed_out']);
  });

  it('keeps an e-mail address once its link is followed, once, within 24 hours', async () => {
    const { account } = await signLea();
    const asked = await askEmail(db, account, ' lea@Exemple.dev ', T0);
    if ('error' in asked) throw new Error(asked.error);
    expect(asked.email).toBe('lea@exemple.dev');
    expect((await q('select id from email_tokens'))[0].id).toBe(tokenHash(asked.token));
    let view = await accountView(db, account, T0);
    expect(view).toMatchObject({ email: null, pendingEmail: 'lea@exemple.dev' });

    expect(await verifyEmail(db, 'wrong', T0)).toBeNull();
    const done = await verifyEmail(db, asked.token, later(HOUR));
    expect(done).toMatchObject({ email: 'lea@exemple.dev' });
    expect(done!.emailVerifiedAt?.toISOString()).toBe(later(HOUR).toISOString());
    expect(await verifyEmail(db, asked.token, later(HOUR))).toBeNull();
    view = await accountView(db, done!, later(HOUR));
    expect(view).toMatchObject({ email: 'lea@exemple.dev', pendingEmail: null });
    expect(await askEmail(db, done!, 'lea@exemple.dev', later(HOUR))).toEqual({ error: 'same' });

    const old = await askEmail(db, done!, 'lea@ailleurs.dev', later(2 * HOUR));
    if ('error' in old) throw new Error(old.error);
    expect(await verifyEmail(db, old.token, later(26 * HOUR + 1))).toBeNull();
    // A link not followed changes nothing: the address checked stays.
    expect((await accountView(db, done!, later(27 * HOUR))).email).toBe('lea@exemple.dev');
  });

  it('refuses a wrong address, and more than five links an hour', async () => {
    const { account } = await signLea();
    expect(await askEmail(db, account, 'lea@', T0)).toEqual({ error: 'invalid' });
    for (let i = 0; i < EMAIL_LINKS_PER_HOUR; i++)
      expect(await askEmail(db, account, `lea${i}@exemple.dev`, later(i * 1000))).toHaveProperty('token');
    expect(await askEmail(db, account, 'lea9@exemple.dev', later(10_000))).toEqual({ error: 'too-many' });
    expect(await askEmail(db, account, 'lea9@exemple.dev', later(HOUR + 10_000))).toHaveProperty('token');
  });

  it('forgets the address and its pending links when asked', async () => {
    const { account } = await signLea();
    const asked = await askEmail(db, account, 'lea@exemple.dev', T0);
    await verifyEmail(db, (asked as { token: string }).token, T0);
    await askEmail(db, account, 'lea@ailleurs.dev', T0);
    await removeEmail(db, account, T0);
    expect(await accountView(db, account, T0)).toMatchObject({ email: null, pendingEmail: null });
  });

  it('exports what it keeps of the account, without any token', async () => {
    const { account, session } = await signLea();
    const asked = await askEmail(db, account, 'lea@exemple.dev', T0);
    await verifyEmail(db, (asked as { token: string }).token, T0);
    const data = await exportAccount(db, (await sessionAccount(db, session, T0))!, later(HOUR));
    expect(data.account).toMatchObject({ githubId: 9001, login: 'lea-martin', email: 'lea@exemple.dev' });
    expect(data.sessions).toHaveLength(1);
    expect(data.installations[0]!.repos).toEqual(['lea-martin/escouade-slack']);
    expect(data.journal.map((e) => e.action)).toEqual(['account.signed_in', 'account.email_asked', 'account.email_verified']);
    const text = JSON.stringify(data);
    expect(text).not.toContain(session);
    expect(text).not.toContain(tokenHash(session));
    expect(text).not.toContain('ghu_lea');
    expect(text).not.toContain((await q('select github_token from sessions'))[0].github_token);
  });

  it('deletes the account and all that is its own; the journal keeps the deletion, by GitHub id', async () => {
    const { account, session } = await signLea();
    await askEmail(db, account, 'lea@exemple.dev', T0);
    await signLea(T0, HOUR);
    const tokens = await deleteAccount(db, account, SECRET, later(2 * HOUR));
    // Only the token still valid comes back, for GitHub to revoke it.
    expect(tokens).toEqual(['ghu_lea']);
    expect(await sessionAccount(db, session, later(2 * HOUR))).toBeNull();
    for (const table of ['accounts', 'sessions', 'email_tokens', 'account_installations'])
      expect(await q(`select * from ${table}`)).toEqual([]);
    // The installation is GitHub's, not the account's: it stays until GitHub says it is gone.
    expect(await q('select id from installations')).toHaveLength(1);
    const journal = await q('select account_id, github_id, action from audit order by id');
    expect(journal.at(-1)).toEqual({ account_id: null, github_id: '9001', action: 'account.deleted' });
    expect(journal.every((e) => e.account_id === null)).toBe(true);
  });

  it('applies the installation webhooks once each, as GitHub says them', async () => {
    const sender = { id: 9001 };
    const created = {
      action: 'created',
      sender,
      installation: leaInstall,
      repositories: [repo(1, 'lea-martin/a'), repo(2, 'lea-martin/b')],
    };
    expect(await applyWebhook(db, 'installation', 'd1', created, T0)).toBe('handled');
    expect(await applyWebhook(db, 'installation', 'd1', created, T0)).toBe('duplicate');
    const repos = async () => (await q('select full_name from installation_repos order by 1')).map((r) => r.full_name);
    expect(await repos()).toEqual(['lea-martin/a', 'lea-martin/b']);

    await applyWebhook(db, 'installation_repositories', 'd2', {
      action: 'added',
      sender,
      installation: leaInstall,
      repositories_added: [repo(3, 'lea-martin/c')],
      repositories_removed: [{ id: 1 }],
    });
    expect(await repos()).toEqual(['lea-martin/b', 'lea-martin/c']);

    await applyWebhook(db, 'repository', 'd3', { action: 'renamed', sender, repository: repo(2, 'lea-martin/b2') });
    expect(await repos()).toEqual(['lea-martin/b2', 'lea-martin/c']);
    await applyWebhook(db, 'repository', 'd4', { action: 'transferred', sender, repository: repo(3, 'kpetit/c') });
    expect(await repos()).toEqual(['lea-martin/b2']);

    await applyWebhook(db, 'installation', 'd5', { action: 'suspend', sender, installation: leaInstall });
    expect((await q('select suspended_at from installations'))[0].suspended_at).not.toBeNull();
    await applyWebhook(db, 'installation', 'd6', { action: 'unsuspend', sender, installation: leaInstall });
    expect((await q('select suspended_at from installations'))[0].suspended_at).toBeNull();

    expect(await applyWebhook(db, 'create', 'd7', { ref: 'v1.0.0', ref_type: 'tag', sender, repository: repo(2, 'lea-martin/b2') })).toBe(
      'handled',
    );
    expect(await applyWebhook(db, 'star', 'd8', { action: 'created', sender })).toBe('ignored');

    await applyWebhook(db, 'installation', 'd9', { action: 'deleted', sender, installation: leaInstall });
    expect(await q('select * from installations')).toEqual([]);
    expect(await repos()).toEqual([]);
    const actions = (await q('select action from audit order by id')).map((r) => r.action);
    expect(actions).toEqual([
      'installation.created',
      'installation.repositories_added',
      'repository.renamed',
      'repository.transferred',
      'installation.suspend',
      'installation.unsuspend',
      'create.tag',
      'installation.deleted',
    ]);
  });

  it('purges what has outlived its time, and nothing else', async () => {
    const { account } = await signLea(T0);
    await signLea(later(20 * DAY));
    await askEmail(db, account, 'lea@exemple.dev', T0);
    await applyWebhook(db, 'star', 'old', { action: 'created' }, T0);
    await applyWebhook(db, 'star', 'new', { action: 'created' }, later(25 * DAY));
    await pool.query("insert into audit (at, action) values ($1, 'old.entry')", [later(-366 * DAY)]);

    const done = await purge(db, later(31 * DAY));
    expect(done).toEqual({ sessions: 1, emailLinks: 1, deliveries: 1, journal: 1 });
    const [left] = await q('select github_token, github_token_expires_at from sessions');
    // The session of day 20 lives on; its GitHub token, 8 hours long, is gone.
    expect(left).toEqual({ github_token: null, github_token_expires_at: null });
    expect((await q('select id from webhook_deliveries')).map((r) => r.id)).toEqual(['new']);
  });
});
