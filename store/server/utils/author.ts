import { unlink } from 'node:fs/promises';
import { join } from 'node:path';
import { and, desc, eq, gte, inArray, isNotNull, isNull, lt, ne, sql } from 'drizzle-orm';
import { accountInstallations, accounts, installationRepos, installsDaily, plugins, versions } from '../db/schema';
import { note, type Account } from './accounts';
import { forgetCatalog } from './catalog';
import { CHECK_NAMES, DOCS, textOf, type CheckResult, type Text } from './checks';
import type { Env } from './config';
import type { Db } from './db';
import { authorMail, sendMail, type AuthorEvent } from './mail';
import type { Plugin, Version } from './submissions';
import { compareVersions } from './versions';
import type { Icon } from '../../shared/tile';

// The author's space (docs/STORE.md, §2.2; the preview's #account): their plugins with their state, a version found
// on a new tag that waits for their click, the versions of a plugin and their installs, and what they may do of it —
// take a version back, take the plugin off the list, hand it to another account (whose publications then freeze 7
// days, the time of a report), delete it (its page and archives go 30 days later, its id stays theirs). Each action
// lands in the journal and is told by e-mail to the author who gave an address: an account in other hands shows.

const DAY = 86_400_000;
/** A transfer freezes the plugin's publications this long: the time for a report (docs/STORE.md, §7). */
export const FREEZE_DAYS = 7;
/** A plugin deleted keeps its page and archives this long: the time to think again, and the backups'. */
export const DELETE_DAYS = 30;
/** The weeks the installs chart shows. */
export const WEEKS = 12;

/** Where an action's e-mail goes from: the store's mail and its address. */
export interface Tell {
  env: Env;
  site: string;
}

/** Tells `accountId` by e-mail of `event`, if they gave an address that was checked. */
async function tell(
  db: Db,
  to: number | null,
  how: Tell | null,
  event: AuthorEvent,
  what: { plugin: string; version?: string; other?: string; until?: Date },
) {
  if (!to || !how) return;
  const [a] = await db.select({ email: accounts.email, at: accounts.emailVerifiedAt }).from(accounts).where(eq(accounts.id, to));
  if (a?.email && a.at)
    await sendMail(how.env, authorMail(a.email, event, { ...what, page: `${how.site}/account?plugin=${encodeURIComponent(what.plugin)}` }));
}

const day = (d: Date) => d.toISOString().slice(0, 10);
const ago = (now: Date, days: number) => day(new Date(now.getTime() - days * DAY));

/** What a plugin is, in the words of the account's table (the preview's tags). */
export type PluginState =
  'deleted' | 'unlisted' | 'verified' | 'review' | 'published' | 'yanked' | 'ready' | 'held' | 'checking' | 'error' | 'rejected';

/** The first check that failed, to say why a version was refused right in the table. */
function firstFailure(
  checks: unknown[],
): { check: string; name: Text; detail: Text; path: string | null; why: Text | null; fix: Text | null; docs: Text } | null {
  const r = (checks as CheckResult[]).find((c) => c?.status === 'fail');
  if (!r) return null;
  const p = r.problems?.[0];
  return {
    check: r.key,
    name: CHECK_NAMES[r.key],
    detail: r.detail,
    path: p?.path ?? null,
    why: p?.why ?? null,
    fix: p?.fix ?? null,
    docs: DOCS,
  };
}

/** The permissions of a manifest as flat words (`network:hooks.slack.com`, `events:agent.created`, `decide`). */
export function permissionWords(manifest: Record<string, unknown> | null | undefined): string[] {
  const p = (manifest?.permissions ?? {}) as Record<string, unknown>;
  const out: string[] = [];
  for (const [k, v] of Object.entries(p)) {
    if (Array.isArray(v)) for (const x of v) if (typeof x === 'string') out.push(`${k}:${x}`);
    if (v === true) out.push(k);
  }
  return out.sort();
}

/** What `next` asks that `previous` did not. */
export const newPermissions = (previous: Record<string, unknown> | null | undefined, next: Record<string, unknown> | null | undefined) => {
  const had = new Set(permissionWords(previous));
  return permissionWords(next).filter((w) => !had.has(w));
};

interface Counts {
  total: number;
  recent: number;
}

/** The installs and updates counted for `ids`: in all, and in the last 30 days. */
async function countsOf(db: Db, ids: string[], now: Date): Promise<Map<string, Counts>> {
  if (!ids.length) return new Map();
  const rows = await db
    .select({
      id: installsDaily.pluginId,
      total: sql<number>`sum(${installsDaily.count})::int`,
      recent: sql<number>`coalesce(sum(${installsDaily.count}) filter (where ${installsDaily.day} >= ${ago(now, 30)}), 0)::int`,
    })
    .from(installsDaily)
    .where(and(inArray(installsDaily.pluginId, ids), eq(installsDaily.event, 'install')))
    .groupBy(installsDaily.pluginId);
  return new Map(rows.map((r) => [r.id, { total: Number(r.total), recent: Number(r.recent) }]));
}

/** The newest of `list` by semver, a release before its pre-releases. */
const newest = (list: Version[]) =>
  [...list].sort((a, b) => {
    const pa = /-/.test(a.version ?? '');
    const pb = /-/.test(b.version ?? '');
    return pa !== pb ? (pa ? 1 : -1) : compareVersions(b.version ?? '0.0.0', a.version ?? '0.0.0');
  })[0];

/** The icons of `ids` (PLG-13), each its newest published version's: the tiles of the offers and claims. */
async function iconsOf(db: Db, ids: string[]): Promise<Map<string, Icon>> {
  if (!ids.length) return new Map();
  const rows = await db
    .select()
    .from(versions)
    .where(and(inArray(versions.pluginId, ids), eq(versions.state, 'published')));
  const out = new Map<string, Icon>();
  for (const id of ids) {
    const icon = newest(rows.filter((v) => v.pluginId === id))?.icon;
    if (icon) out.set(id, icon);
  }
  return out;
}

/** A row of the account's table. */
export interface PluginRow {
  id: string;
  name: string;
  /** Its icon, by the version its name is taken from; null draws its letters. */
  icon: Icon | null;
  state: PluginState;
  /** The version the state speaks of: the latest published, else the last submitted. */
  version: string | null;
  installs: number;
  recent: number;
  /** Why the last version was refused, for a plugin never published. */
  rejection: ReturnType<typeof firstFailure>;
  /** The submission's page of a version never published (first versions are published there, with their categories). */
  submission: number | null;
  repository: string;
  unlistedAt: Date | null;
  deletedAt: Date | null;
  purgeOn: Date | null;
  frozenUntil: Date | null;
  transferredAt: Date | null;
  transferredFrom: string | null;
  transferTo: string | null;
  repoMovedAt: Date | null;
  autoPublish: boolean;
}

function stateOf(plugin: Plugin | null, mine: Version[]): PluginState {
  if (plugin?.deletedAt) return 'deleted';
  if (plugin?.unlistedAt) return 'unlisted';
  const published = mine.filter((v) => v.state === 'published');
  const offered = published.filter((v) => !v.yankedAt);
  if (offered.length) {
    const v = newest(offered)!;
    return v.reviewState === 'verified' ? 'verified' : v.review ? 'review' : 'published';
  }
  if (published.length) return 'yanked';
  const last = mine[0];
  if (!last) return 'checking';
  return last.state === 'dismissed' ? 'rejected' : (last.state as PluginState);
}

/** The account's plugins, and the plugins they submitted that were never published, newest change first. */
export async function authorPlugins(db: Db, account: Account, now = new Date()): Promise<PluginRow[]> {
  const own = await db
    .select()
    .from(plugins)
    .where(and(eq(plugins.accountId, account.id), isNull(plugins.purgedAt)))
    .orderBy(plugins.id);
  const ownIds = own.map((p) => p.id);
  // A first version, refused or waiting: a plugin with no row yet, the account's own submission.
  const firsts = await db
    .select()
    .from(versions)
    .where(
      and(
        eq(versions.accountId, account.id),
        isNotNull(versions.pluginId),
        sql`not exists (select 1 from ${plugins} where ${plugins.id} = ${versions.pluginId})`,
      ),
    )
    .orderBy(desc(versions.id));
  const all = ownIds.length ? await db.select().from(versions).where(inArray(versions.pluginId, ownIds)).orderBy(desc(versions.id)) : [];
  const byId = new Map<string, Version[]>();
  for (const v of [...all, ...firsts]) byId.set(v.pluginId!, [...(byId.get(v.pluginId!) ?? []), v]);
  const counts = await countsOf(db, ownIds, now);
  const transferIds = own.map((p) => p.transferTo).filter((x): x is number => !!x);
  const logins = transferIds.length
    ? new Map(
        (await db.select({ id: accounts.id, login: accounts.login }).from(accounts).where(inArray(accounts.id, transferIds))).map((a) => [
          a.id,
          a.login,
        ]),
      )
    : new Map<number, string>();

  const rows: PluginRow[] = [];
  const row = (id: string, plugin: Plugin | null): PluginRow => {
    const mine = byId.get(id) ?? [];
    const state = stateOf(plugin, mine);
    const published = mine.filter((v) => v.state === 'published');
    const shown = newest(published.filter((v) => !v.yankedAt)) ?? newest(published) ?? mine[0] ?? null;
    const named = newest(published) ?? mine[0];
    const c = counts.get(id) ?? { total: 0, recent: 0 };
    return {
      id,
      name: textOf(named?.manifest?.name) || id,
      icon: named?.icon ?? null,
      state,
      version: shown?.version ?? null,
      installs: c.total,
      recent: c.recent,
      rejection: !plugin && mine[0]?.state === 'rejected' ? firstFailure(mine[0].checks) : null,
      submission: !plugin && mine[0] ? mine[0].id : null,
      repository: plugin?.repoFullName ?? mine[0]?.repoFullName ?? '',
      unlistedAt: plugin?.unlistedAt ?? null,
      deletedAt: plugin?.deletedAt ?? null,
      purgeOn: plugin?.deletedAt ? new Date(plugin.deletedAt.getTime() + DELETE_DAYS * DAY) : null,
      frozenUntil: plugin?.frozenUntil && plugin.frozenUntil > now ? plugin.frozenUntil : null,
      transferredAt: plugin?.transferredAt ?? null,
      transferredFrom: plugin?.transferredFrom ?? null,
      transferTo: plugin?.transferTo ? (logins.get(plugin.transferTo) ?? null) : null,
      repoMovedAt: plugin?.repoMovedAt ?? null,
      autoPublish: plugin?.autoPublish ?? false,
    };
  };
  for (const p of own) rows.push(row(p.id, p));
  for (const id of new Set(firsts.map((v) => v.pluginId!))) rows.push(row(id, null));
  return rows;
}

/** A version found on a new tag, checked, that waits for its author's click: the banner of the account's page. */
export interface Pending {
  id: number;
  plugin: string;
  name: string;
  /** The icon of the version waiting. */
  icon: Icon | null;
  version: string;
  tag: string;
  commit: string;
  createdAt: Date;
  /** The commit of the latest version published, to show « 3f9c2e1 → 81d0a4b ». */
  previousCommit: string | null;
  /** What this version asks that the latest published did not. */
  added: string[];
  frozenUntil: Date | null;
}

export async function pendingVersions(db: Db, account: Account, now = new Date()): Promise<Pending[]> {
  const rows = await db
    .select({ v: versions, plugin: plugins })
    .from(versions)
    .innerJoin(plugins, eq(plugins.id, versions.pluginId))
    .where(and(eq(plugins.accountId, account.id), eq(versions.state, 'ready'), isNull(plugins.deletedAt)))
    .orderBy(desc(versions.id));
  const out: Pending[] = [];
  for (const { v, plugin } of rows) {
    const published = await db
      .select()
      .from(versions)
      .where(and(eq(versions.pluginId, plugin.id), eq(versions.state, 'published')));
    const last = newest(published);
    out.push({
      id: v.id,
      plugin: plugin.id,
      name: textOf(v.manifest?.name) || plugin.id,
      icon: v.icon ?? null,
      version: v.version!,
      tag: v.tag,
      commit: v.commit,
      createdAt: v.checkedAt ?? v.createdAt,
      previousCommit: last?.commit ?? null,
      added: newPermissions(last?.manifest, v.manifest),
      frozenUntil: plugin.frozenUntil && plugin.frozenUntil > now ? plugin.frozenUntil : null,
    });
  }
  return out;
}

/** A plugin another account offers to this one, waiting for their answer. */
export async function transferOffers(db: Db, account: Account) {
  const rows = await db
    .select({ id: plugins.id, from: accounts.login, at: plugins.transferAskedAt })
    .from(plugins)
    .leftJoin(accounts, eq(accounts.id, plugins.accountId))
    .where(and(eq(plugins.transferTo, account.id), isNull(plugins.deletedAt)));
  const icons = await iconsOf(
    db,
    rows.map((r) => r.id),
  );
  return rows.map((r) => ({ id: r.id, from: r.from, at: r.at, icon: icons.get(r.id) ?? null }));
}

/**
 * The plugins whose repository moved, on GitHub, into the hands of this account: GitHub said `repository.transferred`
 * and an installation of the App this account manages reaches the repository now. They may take the plugin over.
 */
export async function repoClaims(db: Db, account: Account) {
  const rows = await db
    .selectDistinct({ id: plugins.id, repository: installationRepos.fullName, at: plugins.repoMovedAt })
    .from(plugins)
    .innerJoin(installationRepos, eq(installationRepos.repoId, plugins.repoId))
    .innerJoin(accountInstallations, eq(accountInstallations.installationId, installationRepos.installationId))
    .where(
      and(
        eq(accountInstallations.accountId, account.id),
        isNotNull(plugins.repoMovedAt),
        isNull(plugins.deletedAt),
        sql`${plugins.accountId} is distinct from ${account.id}`,
      ),
    );
  const icons = await iconsOf(
    db,
    rows.map((r) => r.id),
  );
  return rows.map((r) => ({ ...r, icon: icons.get(r.id) ?? null }));
}

/** A plugin of the account, as its management shows it: its versions, their installs, the weeks' chart. */
export async function pluginDetail(db: Db, account: Account, id: string, now = new Date()) {
  const [plugin] = await db
    .select()
    .from(plugins)
    .where(and(eq(plugins.id, id), eq(plugins.accountId, account.id), isNull(plugins.purgedAt)));
  if (!plugin) return null;
  const list = await db
    .select()
    .from(versions)
    .where(and(eq(versions.pluginId, id), ne(versions.state, 'dismissed')))
    .orderBy(desc(versions.id));
  const byVersion = await db
    .select({ version: installsDaily.version, event: installsDaily.event, n: sql<number>`sum(${installsDaily.count})::int` })
    .from(installsDaily)
    .where(eq(installsDaily.pluginId, id))
    .groupBy(installsDaily.version, installsDaily.event);
  const counted = (version: string, event: string) => Number(byVersion.find((r) => r.version === version && r.event === event)?.n ?? 0);
  // The weeks end today: the last bar is the seven days up to now, the first twelve weeks ago.
  const since = ago(now, WEEKS * 7 - 1);
  const days = await db
    .select({ day: installsDaily.day, event: installsDaily.event, n: sql<number>`sum(${installsDaily.count})::int` })
    .from(installsDaily)
    .where(and(eq(installsDaily.pluginId, id), gte(installsDaily.day, since)))
    .groupBy(installsDaily.day, installsDaily.event);
  const weeks = Array.from({ length: WEEKS }, () => ({ installs: 0, updates: 0 }));
  const today = Date.parse(`${day(now)}T00:00:00Z`);
  for (const d of days) {
    const back = Math.floor((today - Date.parse(`${d.day}T00:00:00Z`)) / DAY / 7);
    const w = weeks[WEEKS - 1 - back];
    if (!w) continue;
    if (d.event === 'install') w.installs += Number(d.n);
    else w.updates += Number(d.n);
  }
  const total = (event: string) => byVersion.filter((r) => r.event === event).reduce((s, r) => s + Number(r.n), 0);
  return {
    id,
    repository: plugin.repoFullName,
    autoPublish: plugin.autoPublish,
    installs: total('install'),
    updates: total('update'),
    recent: (await countsOf(db, [id], now)).get(id)?.recent ?? 0,
    weeks,
    versions: list.map((v) => ({
      id: v.id,
      version: v.version,
      tag: v.tag,
      commit: v.commit,
      state: v.state,
      review: v.review,
      // The review's answer (STORE-8): verified, changes asked or refused, and what it said.
      reviewState: v.reviewState,
      reviewMessage: v.reviewMessage,
      createdAt: v.createdAt,
      publishedAt: v.publishedAt,
      yankedAt: v.yankedAt,
      tagMovedAt: v.tagMovedAt,
      fingerprint: v.versionSha256,
      installs: v.version ? counted(v.version, 'install') : 0,
      updates: v.version ? counted(v.version, 'update') : 0,
      rejection: v.state === 'rejected' ? firstFailure(v.checks) : null,
    })),
  };
}

// ---- What the author does ----

export type Done<E extends string> = { ok: true } | { error: E };

/** The account's own plugin, still on the store (not purged). */
async function ownPlugin(db: Db, account: Account, id: string): Promise<Plugin | null> {
  const [p] = await db
    .select()
    .from(plugins)
    .where(and(eq(plugins.id, id), eq(plugins.accountId, account.id), isNull(plugins.purgedAt)));
  return p ?? null;
}

const who = (a: Account) => ({ accountId: a.id, githubId: a.githubId });

/** Whether a plugin's publications are frozen at `now` (a transfer less than 7 days old). */
export const frozen = (p: Pick<Plugin, 'frozenUntil'>, now: Date) => !!p.frozenUntil && p.frozenUntil > now;

/**
 * Takes a published version back (« yank »): it is no longer offered to install nor as an update; those who have it
 * keep it, with a band « taken back by its author ». Its number stays used. It does not come back.
 */
export async function yankVersion(
  db: Db,
  account: Account,
  versionId: number,
  how: Tell | null,
  now = new Date(),
): Promise<Done<'not-found' | 'not-published' | 'already'>> {
  const [v] = await db.select().from(versions).where(eq(versions.id, versionId));
  if (!v?.pluginId || !(await ownPlugin(db, account, v.pluginId))) return { error: 'not-found' };
  if (v.state !== 'published') return { error: 'not-published' };
  if (v.yankedAt) return { error: 'already' };
  await db.update(versions).set({ yankedAt: now }).where(eq(versions.id, v.id));
  forgetCatalog();
  await note(db, 'version.yanked', who(account), { plugin: v.pluginId, version: v.version, sha256: v.versionSha256 }, now);
  await tell(db, account.id, how, 'yanked', { plugin: v.pluginId, version: v.version! });
  return { ok: true };
}

/** Sets aside a version waiting for the author's click: it is not published, its tag may be pushed again. */
export async function dismissVersion(
  db: Db,
  account: Account,
  versionId: number,
  now = new Date(),
): Promise<Done<'not-found' | 'not-ready'>> {
  const [v] = await db.select().from(versions).where(eq(versions.id, versionId));
  if (!v?.pluginId || !(await ownPlugin(db, account, v.pluginId))) return { error: 'not-found' };
  if (v.state !== 'ready') return { error: 'not-ready' };
  await db
    .update(versions)
    .set({ state: 'dismissed' })
    .where(and(eq(versions.id, v.id), eq(versions.state, 'ready')));
  await note(db, 'version.dismissed', who(account), { plugin: v.pluginId, version: v.version, tag: v.tag }, now);
  return { ok: true };
}

/** Takes the plugin off the list (out of the search and the catalogue, its page still readable), or puts it back. */
export async function setListed(
  db: Db,
  account: Account,
  id: string,
  listed: boolean,
  how: Tell | null,
  now = new Date(),
): Promise<Done<'not-found' | 'deleted'>> {
  const p = await ownPlugin(db, account, id);
  if (!p) return { error: 'not-found' };
  if (p.deletedAt) return { error: 'deleted' };
  if (listed === !p.unlistedAt) return { ok: true };
  await db
    .update(plugins)
    .set({ unlistedAt: listed ? null : now })
    .where(eq(plugins.id, id));
  forgetCatalog();
  await note(db, listed ? 'plugin.relisted' : 'plugin.unlisted', who(account), { plugin: id }, now);
  await tell(db, account.id, how, listed ? 'relisted' : 'unlisted', { plugin: id });
  return { ok: true };
}

/** Lets a new tag of the plugin be published without a click, or asks for the click again. */
export async function setAutoPublish(db: Db, account: Account, id: string, on: boolean, now = new Date()): Promise<Done<'not-found'>> {
  const p = await ownPlugin(db, account, id);
  if (!p) return { error: 'not-found' };
  await db.update(plugins).set({ autoPublish: on }).where(eq(plugins.id, id));
  await note(db, on ? 'plugin.auto_publish_on' : 'plugin.auto_publish_off', who(account), { plugin: id }, now);
  return { ok: true };
}

/** Offers the plugin to the account of `login` on the store; it is theirs once they accept. */
export async function askTransfer(
  db: Db,
  account: Account,
  id: string,
  login: unknown,
  how: Tell | null,
  now = new Date(),
): Promise<Done<'not-found' | 'deleted' | 'no-account' | 'self'>> {
  const p = await ownPlugin(db, account, id);
  if (!p) return { error: 'not-found' };
  if (p.deletedAt) return { error: 'deleted' };
  const name = typeof login === 'string' ? login.trim().replace(/^@/, '') : '';
  if (!name || name.length > 39) return { error: 'no-account' };
  const [to] = await db
    .select()
    .from(accounts)
    .where(sql`lower(${accounts.login}) = lower(${name})`);
  if (!to) return { error: 'no-account' };
  if (to.id === account.id) return { error: 'self' };
  await db.update(plugins).set({ transferTo: to.id, transferAskedAt: now }).where(eq(plugins.id, id));
  await note(db, 'plugin.transfer_asked', who(account), { plugin: id, to: to.login, toGithubId: to.githubId }, now);
  await tell(db, to.id, how, 'transfer-offered', { plugin: id, other: account.login });
  await tell(db, account.id, how, 'transfer-asked', { plugin: id, other: to.login });
  return { ok: true };
}

/** Takes back an offer not yet accepted. */
export async function cancelTransfer(db: Db, account: Account, id: string, now = new Date()): Promise<Done<'not-found'>> {
  const p = await ownPlugin(db, account, id);
  if (!p) return { error: 'not-found' };
  if (!p.transferTo) return { ok: true };
  await db.update(plugins).set({ transferTo: null, transferAskedAt: null }).where(eq(plugins.id, id));
  await note(db, 'plugin.transfer_cancelled', who(account), { plugin: id }, now);
  return { ok: true };
}

/** Hands the plugin to `to`: its publications freeze 7 days; its signatures stay valid (they are on the archives). */
async function handOver(db: Db, p: Plugin, to: Account, how: Tell | null, via: 'accepted' | 'repository', now: Date) {
  const [from] = p.accountId ? await db.select().from(accounts).where(eq(accounts.id, p.accountId)) : [];
  const until = new Date(now.getTime() + FREEZE_DAYS * DAY);
  await db
    .update(plugins)
    .set({
      accountId: to.id,
      transferTo: null,
      transferAskedAt: null,
      transferredAt: now,
      transferredFrom: from?.login ?? null,
      frozenUntil: until,
      repoMovedAt: null,
    })
    .where(eq(plugins.id, p.id));
  // The versions still waiting are the new owner's to publish, once the freeze is over.
  await db
    .update(versions)
    .set({ accountId: to.id })
    .where(and(eq(versions.pluginId, p.id), inArray(versions.state, ['checking', 'held', 'ready'])));
  forgetCatalog();
  await note(db, 'plugin.transferred', who(to), { plugin: p.id, from: from?.login ?? null, via, frozenUntil: until.toISOString() }, now);
  await tell(db, to.id, how, 'transferred-in', { plugin: p.id, other: from?.login ?? '', until });
  if (from) await tell(db, from.id, how, 'transferred-out', { plugin: p.id, other: to.login, until });
}

/** The answer of the account a plugin is offered to: accepted, it is theirs and frozen 7 days; refused, it stays. */
export async function answerTransfer(
  db: Db,
  account: Account,
  id: string,
  accept: boolean,
  how: Tell | null,
  now = new Date(),
): Promise<Done<'not-found'>> {
  const [p] = await db
    .select()
    .from(plugins)
    .where(and(eq(plugins.id, id), eq(plugins.transferTo, account.id), isNull(plugins.deletedAt)));
  if (!p) return { error: 'not-found' };
  if (accept) {
    await handOver(db, p, account, how, 'accepted', now);
    return { ok: true };
  }
  await db.update(plugins).set({ transferTo: null, transferAskedAt: null }).where(eq(plugins.id, id));
  await note(db, 'plugin.transfer_refused', who(account), { plugin: id }, now);
  await tell(db, p.accountId, how, 'transfer-refused', { plugin: id, other: account.login });
  return { ok: true };
}

/** Takes over a plugin whose repository moved on GitHub to an owner whose installation of the App this account manages. */
export async function claimPlugin(db: Db, account: Account, id: string, how: Tell | null, now = new Date()): Promise<Done<'not-found'>> {
  const claim = (await repoClaims(db, account)).find((c) => c.id === id);
  if (!claim) return { error: 'not-found' };
  const [p] = await db.select().from(plugins).where(eq(plugins.id, id));
  await db.update(plugins).set({ repoFullName: claim.repository }).where(eq(plugins.id, id));
  await handOver(db, { ...p!, repoFullName: claim.repository }, account, how, 'repository', now);
  return { ok: true };
}

/**
 * Deletes the plugin, once its id is typed back: it leaves the catalogue at once; its page and archives go 30 days
 * later; installations keep what they have, marked « removed from the store »; its id stays the account's.
 */
export async function deletePlugin(
  db: Db,
  account: Account,
  id: string,
  confirm: unknown,
  how: Tell | null,
  now = new Date(),
): Promise<Done<'not-found' | 'unconfirmed'>> {
  const p = await ownPlugin(db, account, id);
  if (!p) return { error: 'not-found' };
  if (typeof confirm !== 'string' || confirm.trim() !== id) return { error: 'unconfirmed' };
  if (p.deletedAt) return { ok: true };
  await db.update(plugins).set({ deletedAt: now, transferTo: null, transferAskedAt: null }).where(eq(plugins.id, id));
  await db
    .update(versions)
    .set({ state: 'dismissed' })
    .where(and(eq(versions.pluginId, id), eq(versions.state, 'ready')));
  forgetCatalog();
  await note(db, 'plugin.deleted', who(account), { plugin: id }, now);
  await tell(db, account.id, how, 'deleted', { plugin: id, until: new Date(now.getTime() + DELETE_DAYS * DAY) });
  return { ok: true };
}

/** Undoes a deletion within its 30 days: the plugin comes back as it was (off the list, if it was). */
export async function restorePlugin(db: Db, account: Account, id: string, how: Tell | null, now = new Date()): Promise<Done<'not-found'>> {
  const p = await ownPlugin(db, account, id);
  if (!p?.deletedAt) return { error: 'not-found' };
  await db.update(plugins).set({ deletedAt: null }).where(eq(plugins.id, id));
  forgetCatalog();
  await note(db, 'plugin.restored', who(account), { plugin: id }, now);
  await tell(db, account.id, how, 'restored', { plugin: id });
  return { ok: true };
}

/**
 * What the nightly purge does of the plugins deleted 30 days ago: their versions and their kept files go (a file
 * another version still uses stays), the plugin's row stays, emptied, its id reserved, the numbers it used kept.
 */
export async function purgeDeleted(db: Db, archives: string, now = new Date()): Promise<{ plugins: number; files: number }> {
  const due = await db
    .select()
    .from(plugins)
    .where(and(isNotNull(plugins.deletedAt), isNull(plugins.purgedAt), lt(plugins.deletedAt, new Date(now.getTime() - DELETE_DAYS * DAY))));
  let files = 0;
  for (const p of due) {
    const list = await db.select().from(versions).where(eq(versions.pluginId, p.id));
    const kept = new Map<string, string>();
    for (const v of list) {
      if (v.archiveSha256) kept.set(v.archiveSha256, '.tar.gz');
      for (const b of Object.values((v.binaries ?? {}) as Record<string, { name: string; sha256: string }>))
        kept.set(b.sha256, /\.(tar\.gz|tgz|zip|tar\.xz|tar\.zst|exe)$/i.exec(b.name)?.[0].toLowerCase() ?? '.bin');
      for (const s of v.screenshots ?? []) kept.set(s.sha256, `.${s.format === 'jpeg' ? 'jpg' : s.format}`);
    }
    const used = new Set(
      list
        .map((v) => v.version)
        .filter((x): x is string => !!x)
        .concat(p.retiredVersions),
    );
    await db.delete(versions).where(eq(versions.pluginId, p.id));
    await db
      .update(plugins)
      .set({ purgedAt: now, categories: [], keywords: [], retiredVersions: [...used].sort(compareVersions) })
      .where(eq(plugins.id, p.id));
    for (const [sha, ext] of kept) {
      if (await stillUsed(db, sha)) continue;
      const path = join(archives, 'sha256', sha.slice(0, 2), `${sha}${ext}`);
      if (
        await unlink(path).then(
          () => true,
          () => false,
        )
      )
        files++;
    }
    await note(db, 'plugin.purged', { accountId: p.accountId }, { plugin: p.id, versions: list.length }, now);
  }
  return { plugins: due.length, files };
}

/** Whether another version still holds a kept file (two versions of the same commit share their archive). */
async function stillUsed(db: Db, sha: string): Promise<boolean> {
  const [r] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(versions)
    .where(
      sql`${versions.archiveSha256} = ${sha}
        or exists (select 1 from jsonb_each(coalesce(${versions.binaries}, '{}'::jsonb)) b where b.value->>'sha256' = ${sha})
        or exists (select 1 from jsonb_array_elements(coalesce(${versions.screenshots}, '[]'::jsonb)) s where s->>'sha256' = ${sha})`,
    );
  return (r?.n ?? 0) > 0;
}

/** The plugins of the account that keep it from being deleted: transfer them or delete them first (§2.2). */
export async function pluginsInTheWay(db: Db, accountId: number): Promise<string[]> {
  const rows = await db
    .select({ id: plugins.id })
    .from(plugins)
    .where(and(eq(plugins.accountId, accountId), isNull(plugins.deletedAt)))
    .orderBy(plugins.id);
  return rows.map((r) => r.id);
}
