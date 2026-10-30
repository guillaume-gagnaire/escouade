import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { and, eq, isNull, lt, sql } from 'drizzle-orm';
import { accounts, installsDaily, plugins, reports, versions } from '../db/schema';
import { BUILTINS, isBuiltinId, type Builtin } from './builtin';
import { textOf, type Lang, type Text } from './checks';
import type { Db } from './db';
import { renderDocs, type Docs } from './readme';
import { CATEGORIES, keepImage } from './submissions';
import { parseRevocations, revoked, type RevocationList } from './revocation-list';
import { lastRevocations } from './revocations';
import { signatureIndex, verifiedBy, type SignatureIndex } from './signatures';
import { readArchive } from './tar';
import { fold, words } from '../../shared/search';
import { hueOf, markOf, type Icon } from '../../shared/tile';
import type { Logo } from '../../shared/marks';
import { compareVersions } from './versions';

export { hueOf, markOf };

// What the public pages show (docs/STORE.md §6, STORE-4): the published plugins, each by its last published version,
// and Escouade's own; the search, made on the server (decided on 2026-10-29: relevance, accents and case ignored);
// a plugin's page with its README, its permissions in words, its versions and where it comes from; an author's page.

export type Category = (typeof CATEGORIES)[number];

/** The categories in the order of choice 214, with their names in the two languages. */
export const CATEGORY_NAMES: Record<Category, Text> = {
  harness: { fr: 'Harness', en: 'Harnesses' },
  'ticket-sources': { fr: 'Sources du Kanban', en: 'Kanban sources' },
  forges: { fr: 'Hébergeurs de code', en: 'Code hosts' },
  themes: { fr: 'Thèmes', en: 'Themes' },
  notifications: { fr: 'Notifications', en: 'Notifications' },
  permissions: { fr: 'Autorisations', en: 'Permissions' },
  tools: { fr: 'Outils', en: 'Tools' },
  languages: { fr: 'Langues', en: 'Languages' },
};

/** What a plugin asks for, in one line of a list: the strongest first (docs/PLUGINS.md §3.4). */
export type Ask = { kind: 'run' | 'decide' | 'events' | 'secret' | 'none' } | { kind: 'net'; host: string };

/** Where a plugin's version stands with the review: Escouade's own, verified (signed, STORE-8), asked, or not. */
export type Trust = 'builtin' | 'verified' | 'review' | 'unverified';

export interface Entry {
  id: string;
  name: Text;
  description: Text;
  /** The two letters of its tile, and their hue (a class `h<hue>`: the pages allow no style attribute). */
  mark: string;
  hue: number;
  /** Its icon, by its last published version's (PLG-13); null draws its letters. */
  icon: Icon | null;
  /** The logo of one of Escouade's own, drawn inline (shared/marks.ts); null for the others. */
  logo: Logo | null;
  author: { login: string; githubId: number } | null;
  builtin: boolean;
  categories: Category[];
  keywords: string[];
  /** Its last published version; Escouade's own have the app's. */
  version: string | null;
  trust: Trust;
  installs: number;
  updatedAt: string | null;
  firstAt: string | null;
  asks: Ask[];
}

export interface Permission {
  icon: 'run' | 'gavel' | 'tool' | 'bolt' | 'net' | 'key' | 'leaf';
  tone: 'strong' | 'red' | '';
  text: Text;
  /** In a fixed-width font: the program, the hosts, the events, the commands. */
  mono?: string;
  detail?: Text;
}

const t = (fr: string, en: string): Text => ({ fr, en });
const list = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []);
const obj = (v: unknown): Record<string, unknown> =>
  v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {};

/** A manifest's text in a language, the other one when it has none (`name`, `description`). */
export function localized(v: unknown): Text {
  if (typeof v === 'string') return { fr: v, en: v };
  const o = obj(v);
  const any = textOf(v);
  return { fr: typeof o.fr === 'string' && o.fr ? o.fr : any, en: typeof o.en === 'string' && o.en ? o.en : any };
}

/** What a manifest asks for, in one line. */
export function asksOf(m: Record<string, unknown>): Ask[] {
  const p = obj(m.permissions);
  const asks: Ask[] = [m.run ? { kind: 'run' } : { kind: 'none' }];
  if (p.decide === true) asks.push({ kind: 'decide' });
  for (const host of list(p.network)) asks.push({ kind: 'net', host });
  if (list(p.secrets).length) asks.push({ kind: 'secret' });
  return asks;
}

/** What a manifest asks for, in words, the strongest first: the « Ce qu'il pourra faire » of its page. */
export function permissionsOf(m: Record<string, unknown>, binaries: Record<string, unknown> | null): Permission[] {
  const p = obj(m.permissions);
  const run = obj(m.run);
  const out: Permission[] = [];
  if (m.run) {
    const attested = Object.values(binaries ?? {}).some((b) => obj(b).attested === true);
    out.push({
      icon: 'run',
      tone: 'strong',
      text: t('Lance un programme sur ton ordinateur, avec tes droits', 'Runs a program on your computer, with your rights'),
      mono: typeof run.command === 'string' ? run.command : undefined,
      detail:
        binaries && Object.keys(binaries).length
          ? attested
            ? t('compilé par la CI du dépôt, attesté', 'built by the repository’s CI, attested')
            : t('compilé hors du dépôt, sans attestation', 'built outside the repository, unattested')
          : t('dans le dépôt', 'in the repository'),
    });
  }
  if (p.decide === true)
    out.push({ icon: 'gavel', tone: 'red', text: t('Décide des autorisations de tes agents', 'Decides your agents’ permissions') });
  if (list(p.commands).length)
    out.push({ icon: 'tool', tone: '', text: t('Lance ces commandes', 'Runs these commands'), mono: list(p.commands).join(' · ') });
  if (list(p.events).length)
    out.push({ icon: 'bolt', tone: '', text: t('Reçoit ces événements', 'Gets these events'), mono: list(p.events).join(' · ') });
  for (const host of list(p.network)) out.push({ icon: 'net', tone: '', text: t('Se connecte à', 'Connects to'), mono: host });
  const titles = new Map(
    (Array.isArray(obj(m.contributes).secrets) ? (obj(m.contributes).secrets as unknown[]) : []).map((s) => [
      obj(s).key,
      localized(obj(s).title),
    ]),
  );
  for (const key of list(p.secrets)) {
    const title = titles.get(key) ?? { fr: key, en: key };
    out.push({ icon: 'key', tone: '', text: t(`Garde un secret : ${title.fr}`, `Keeps a secret: ${title.en}`) });
  }
  if (!out.length)
    out.push({
      icon: 'leaf',
      tone: '',
      text: t('Ne demande rien : rien ne s’exécute, rien ne sort', 'Asks for nothing: nothing runs, nothing leaves'),
    });
  return out;
}

/** The words of one of Escouade's own: its program is the app's, signed with it. */
export function builtinPermissions(b: Builtin): Permission[] {
  const out: Permission[] = [
    {
      icon: 'run',
      tone: 'strong',
      text: t('Lance un programme d’Escouade, avec tes droits', 'Runs a program of Escouade, with your rights'),
      mono: 'escouade-plugins',
      detail: t('livré et signé avec l’app', 'shipped and signed with the app'),
    },
  ];
  for (const host of b.network) out.push({ icon: 'net', tone: '', text: t('Se connecte à', 'Connects to'), mono: host });
  if (b.categories.includes('forges') && !b.network.length)
    out.push({ icon: 'net', tone: '', text: t('Se connecte à l’instance que tu ajoutes', 'Connects to the instance you add') });
  if (b.secrets) out.push({ icon: 'key', tone: '', text: t('Garde les jetons de tes comptes', 'Keeps your accounts’ tokens') });
  return out;
}

function builtinEntry(b: Builtin): Entry {
  return {
    id: b.id,
    name: { fr: b.name, en: b.name.replace(' et ', ' and ') },
    description: b.description,
    mark: markOf(b.name),
    hue: 180,
    icon: null,
    logo: b.logo,
    author: null,
    builtin: true,
    categories: b.categories as Category[],
    keywords: [],
    version: null,
    trust: 'builtin',
    installs: 0,
    updatedAt: null,
    firstAt: null,
    asks: [
      { kind: 'run' },
      ...b.network.map((host) => ({ kind: 'net' as const, host })),
      ...(b.secrets ? [{ kind: 'secret' as const }] : []),
    ],
  };
}

/** The published versions of every plugin, with its author; the newest version of each plugin first. */
async function publishedRows(db: Db, where = sql`true`) {
  return db
    .select({ plugin: plugins, version: versions, login: accounts.login, githubId: accounts.githubId })
    .from(plugins)
    .innerJoin(versions, and(eq(versions.pluginId, plugins.id), eq(versions.state, 'published')))
    .leftJoin(accounts, eq(accounts.id, plugins.accountId))
    .where(where);
}

type Row = Awaited<ReturnType<typeof publishedRows>>[number];

/** What the lists read beside the versions (STORE-6): the installations counted until yesterday, as the catalogue of
 * the app says them, and the last revocation list, whose disabled versions leave the pages. */
interface Context {
  installs: Map<string, number>;
  list: RevocationList | null;
  signed: SignatureIndex;
}

async function contextOf(db: Db, now = new Date()): Promise<Context> {
  const today = now.toISOString().slice(0, 10);
  const counts = await db
    .select({ id: installsDaily.pluginId, n: sql<number>`sum(${installsDaily.count})::int` })
    .from(installsDaily)
    .where(lt(installsDaily.day, today))
    .groupBy(installsDaily.pluginId);
  const stored = await lastRevocations(db);
  const parsed = stored ? parseRevocations(stored.json) : null;
  return {
    installs: new Map(counts.map((c) => [c.id, Number(c.n)])),
    list: parsed && !('error' in parsed) ? parsed : null,
    signed: await signatureIndex(db),
  };
}

const disabled = (ctx: Context, r: Row) => !!revoked(ctx.list, r.plugin.id, r.version.version!, 'disable');

/** The signature that makes a row's version verified (STORE-8), or null. */
const signedOf = (ctx: Context, r: { plugin: { id: string }; version: { version: string | null; versionSha256: string | null } }) =>
  verifiedBy(ctx.signed, ctx.list, r.plugin.id, r.version.version!, r.version.versionSha256);

/** Where a version stands: verified once signed, else asked for review, or not. */
const trustOf = (ctx: Context, r: Row): Trust => (signedOf(ctx, r) ? 'verified' : r.version.review ? 'review' : 'unverified');

/**
 * The entries of the rows: one per plugin, by its last published version that the revocation list does not disable
 * and its author did not take back (docs/STORE.md §2.2); a plugin whose versions were all taken back has none, unless
 * `taken` (its own page, still readable).
 */
function entriesOf(all: Row[], ctx: Context, taken = false): Entry[] {
  const rows = all.filter((r) => !disabled(ctx, r));
  const by = new Map<string, Row[]>();
  for (const r of rows) by.set(r.plugin.id, [...(by.get(r.plugin.id) ?? []), r]);
  return [...by.values()].flatMap((rs) => {
    rs.sort((a, b) => compareVersions(b.version.version!, a.version.version!));
    const offered = rs.find((r) => !r.version.yankedAt);
    if (!offered && !taken) return [];
    const row = offered ?? rs[0]!;
    const { plugin, version: v, login, githubId } = row;
    const m = v.manifest ?? {};
    const name = localized(m.name);
    const dates = rs
      .map((r) => r.version.publishedAt?.toISOString() ?? null)
      .filter((d): d is string => !!d)
      .sort();
    return {
      id: plugin.id,
      name,
      description: localized(m.description),
      mark: markOf(name.en),
      hue: hueOf(plugin.id),
      icon: v.icon ?? null,
      logo: null,
      author: login && githubId ? { login, githubId } : null,
      builtin: false,
      categories: plugin.categories.filter((c): c is Category => (CATEGORIES as readonly string[]).includes(c)),
      keywords: plugin.keywords,
      version: v.version,
      trust: trustOf(ctx, row),
      installs: ctx.installs.get(plugin.id) ?? 0,
      updatedAt: v.publishedAt?.toISOString() ?? null,
      firstAt: dates[0] ?? null,
      asks: asksOf(m),
    };
  });
}

/** The plugins its author keeps on the list: not taken off it, not deleted (docs/STORE.md §2.2). */
const listed = sql`${isNull(plugins.unlistedAt)} and ${isNull(plugins.deletedAt)}`;

/** The whole catalog: Escouade's own, then the published plugins on the list; Escouade's own alone without a database. */
export async function catalog(db: Db | null): Promise<Entry[]> {
  const own = BUILTINS.map(builtinEntry);
  if (!db) return own;
  return [...own, ...entriesOf(await publishedRows(db, listed), await contextOf(db)).filter((e) => !isBuiltinId(e.id))];
}

// ---- The search ----

export interface Query {
  q?: string;
  cat?: string;
  verified?: boolean;
  noProgram?: boolean;
  noNetwork?: boolean;
  noSecret?: boolean;
  sort?: 'relevance' | 'installs' | 'recent';
}

/** How well `e` answers the words of a query (0: not at all; every word must be found somewhere). */
export function score(e: Entry, q: string): number {
  const ws = words(q);
  if (!ws.length) return 1;
  const names = [e.name.fr, e.name.en].map(fold);
  const nameWords = names.flatMap((n) => n.split(/\s+/));
  const keywords = e.keywords.flatMap(words);
  const cats = e.categories.flatMap((c) => [CATEGORY_NAMES[c].fr, CATEGORY_NAMES[c].en, c]).flatMap(words);
  const author = fold(e.builtin ? 'escouade' : (e.author?.login ?? ''));
  const desc = [e.description.fr, e.description.en].map(fold).join(' ');
  const id = fold(e.id);
  let total = 0;
  for (const w of ws) {
    let s = 0;
    if (nameWords.includes(w)) s = Math.max(s, 12);
    else if (nameWords.some((n) => n.startsWith(w))) s = Math.max(s, 9);
    else if (names.some((n) => n.includes(w))) s = Math.max(s, 6);
    if (keywords.includes(w)) s = Math.max(s, 8);
    else if (keywords.some((k) => k.startsWith(w))) s = Math.max(s, 5);
    if (cats.some((c) => c.startsWith(w))) s = Math.max(s, 5);
    if (author.split(/\s+/).some((a) => a.startsWith(w))) s = Math.max(s, 5);
    if (id.includes(w)) s = Math.max(s, 4);
    if (desc.split(/\s+/).some((d) => d.startsWith(w))) s = Math.max(s, 3);
    else if (desc.includes(w)) s = Math.max(s, 1);
    if (!s) return 0;
    total += s;
  }
  // The whole query as the name: first.
  if (names.includes(ws.join(' '))) total += 20;
  return total;
}

const runs = (e: Entry) => e.asks.some((a) => a.kind === 'run');
const verifiedish = (e: Entry) => e.trust === 'builtin' || e.trust === 'verified';

/** The entries that answer `query`, sorted; and what each filter would leave, for the counts of the filters. */
export function search(entries: Entry[], query: Query): { results: Entry[]; counts: Record<string, number>; all: number } {
  const q = (query.q ?? '').trim().slice(0, 200);
  const scored = entries.map((e) => ({ e, s: score(e, q) })).filter((x) => x.s > 0);
  const kept = scored.filter(
    ({ e }) =>
      (!query.verified || verifiedish(e)) &&
      (!query.noProgram || !runs(e)) &&
      (!query.noNetwork || !e.asks.some((a) => a.kind === 'net')) &&
      (!query.noSecret || !e.asks.some((a) => a.kind === 'secret')),
  );
  const counts: Record<string, number> = {};
  for (const { e } of kept) for (const c of e.categories) counts[c] = (counts[c] ?? 0) + 1;
  const inCat = query.cat ? kept.filter(({ e }) => (e.categories as string[]).includes(query.cat!)) : kept;
  const by = query.sort ?? (q ? 'relevance' : 'installs');
  const name = (e: Entry) => fold(e.name.en);
  inCat.sort((a, b) => {
    if (by === 'relevance' && b.s !== a.s) return b.s - a.s;
    if (by === 'recent' && (b.e.updatedAt ?? '') !== (a.e.updatedAt ?? '')) return (b.e.updatedAt ?? '').localeCompare(a.e.updatedAt ?? '');
    if (b.e.installs !== a.e.installs) return b.e.installs - a.e.installs;
    return name(a.e).localeCompare(name(b.e));
  });
  return { results: inCat.map((x) => x.e), counts, all: kept.length };
}

// ---- The pages ----

/** Escouade's own, one of each category in turn: the row of the verified is not six harnesses. */
function mixed(own: Entry[]): Entry[] {
  const queues = new Map<string, Entry[]>();
  for (const e of own) queues.set(e.categories[0]!, [...(queues.get(e.categories[0]!) ?? []), e]);
  const out: Entry[] = [];
  while (out.length < own.length) for (const q of queues.values()) if (q.length) out.push(q.shift()!);
  return out;
}

/** The home page: the categories and their counts, the verified, the new, the most installed. */
export function home(entries: Entry[]) {
  const counts: Record<string, number> = {};
  for (const e of entries) for (const c of e.categories) counts[c] = (counts[c] ?? 0) + 1;
  const store = entries.filter((e) => !e.builtin);
  return {
    total: entries.length,
    counts,
    verified: [...store.filter(verifiedish), ...mixed(entries.filter((e) => e.builtin))].slice(0, 6),
    fresh: [...store].sort((a, b) => (b.firstAt ?? '').localeCompare(a.firstAt ?? '')).slice(0, 4),
    top: store
      .filter((e) => e.installs > 0)
      .sort((a, b) => b.installs - a.installs)
      .slice(0, 4),
  };
}

export interface VersionLine {
  version: string;
  publishedAt: string | null;
  tag: string;
  commit: string;
  sha256: string | null;
  trust: Trust;
  /** The tag moved on the repository after the store froze it: the version is kept as it was. */
  tagMoved: string | null;
  escouade: string | null;
  /** Taken off by the revocation list: no longer installable. */
  disabled: boolean;
  /** Taken back by its author: no longer offered; those who have it keep it. */
  yanked: boolean;
}

const range = (m: Record<string, unknown>) => {
  const e = obj(m.escouade);
  if (typeof e.min !== 'string') return null;
  return typeof e.max === 'string' ? `${e.min} – ${e.max}` : `≥ ${e.min}`;
};

/** A version's README and documentation, rendered at its check; for a version published before, now, from its archive. */
export async function docsOf(db: Db, dir: string, v: typeof versions.$inferSelect): Promise<Docs | null> {
  if (v.docs) return v.docs;
  if (!v.archiveSha256) return null;
  try {
    const gz = await readFile(join(dir, 'sha256', v.archiveSha256.slice(0, 2), `${v.archiveSha256}.tar.gz`));
    const docs = await renderDocs(readArchive(gz), { repo: v.repoFullName, commit: v.commit, keep: keepImage(dir) });
    await db.update(versions).set({ docs }).where(eq(versions.id, v.id));
    return docs;
  } catch (e) {
    console.error(`[store] the docs of version ${v.id} could not be read:`, e instanceof Error ? e.message : e);
    return null;
  }
}

/** A plugin's page: its entry, its last version's files and facts, its versions; null if it has none published. */
export async function pluginPage(db: Db | null, dir: string, id: string) {
  const b = BUILTINS.find((x) => x.id === id);
  if (b)
    return {
      entry: builtinEntry(b),
      builtin: { docs: b.docs },
      permissions: builtinPermissions(b),
      versions: [],
      latest: null,
      status: null,
    };
  if (!db || id.length > 214) return null;
  const rows = await publishedRows(db, eq(plugins.id, id));
  const ctx = await contextOf(db);
  const [entry] = entriesOf(rows, ctx, true);
  if (!entry) return null;
  rows.sort((a, b) => compareVersions(b.version.version!, a.version.version!));
  const shown = (rows.find((r) => !disabled(ctx, r) && !r.version.yankedAt) ?? rows.find((r) => !disabled(ctx, r)))!;
  const v = shown.version;
  const sig = signedOf(ctx, shown);
  const plugin = rows[0]!.plugin;
  const m = v.manifest ?? {};
  const docs = await docsOf(db, dir, v);
  const [open] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(reports)
    .where(and(eq(reports.pluginId, id), eq(reports.state, 'open')));
  return {
    entry,
    builtin: null,
    permissions: permissionsOf(m, v.binaries),
    versions: rows.map((r): VersionLine => ({
      version: r.version.version!,
      publishedAt: r.version.publishedAt?.toISOString() ?? null,
      tag: r.version.tag,
      commit: r.version.commit,
      sha256: r.version.versionSha256,
      trust: trustOf(ctx, r),
      tagMoved: r.version.tagMovedTo,
      escouade: range(r.version.manifest ?? {}),
      disabled: disabled(ctx, r),
      yanked: !!r.version.yankedAt,
    })),
    // What its author did of it (docs/STORE.md §2.2): its page stays readable, and says so.
    status: {
      unlisted: !!plugin.unlistedAt,
      deletedAt: plugin.deletedAt?.toISOString() ?? null,
      transferredAt: plugin.transferredAt?.toISOString() ?? null,
      transferredFrom: plugin.transferredFrom,
    },
    latest: {
      repository: v.repoFullName,
      tag: v.tag,
      commit: v.commit,
      sha256: v.versionSha256,
      license: typeof m.license === 'string' ? m.license : null,
      escouade: range(m),
      systems: Object.keys(v.binaries ?? {}),
      screenshots: (v.screenshots ?? []).map((s) => ({
        src: `/files/${s.sha256}.${s.format === 'jpeg' ? 'jpg' : s.format}`,
        width: s.width,
        height: s.height,
      })),
      docs: docs ?? { readme: {}, pages: [], images: [] },
      reports: open?.n ?? 0,
      // The review's signature of this version (STORE-8): when, by which key, and its public summary.
      signature: sig ? { signedAt: sig.signedAt.toISOString(), keyId: sig.keyId, summary: sig.summary } : null,
    },
  };
}

/** An author's page: who they are on GitHub, and their published plugins; null if nobody has this login. */
export async function authorPage(db: Db | null, login: string) {
  if (login.toLowerCase() === 'escouade') return { author: null, builtin: true, plugins: BUILTINS.map(builtinEntry) };
  if (!db || !/^[A-Za-z0-9-]{1,39}$/.test(login)) return null;
  const [account] = await db
    .select({ id: accounts.id, login: accounts.login, githubId: accounts.githubId, name: accounts.name, createdAt: accounts.createdAt })
    .from(accounts)
    .where(sql`lower(${accounts.login}) = lower(${login})`);
  if (!account) return null;
  const entries = entriesOf(await publishedRows(db, and(eq(plugins.accountId, account.id), listed)), await contextOf(db));
  entries.sort((a, b) => b.installs - a.installs || fold(a.name.en).localeCompare(fold(b.name.en)));
  return {
    author: { login: account.login, githubId: account.githubId, name: account.name, since: account.createdAt.toISOString() },
    builtin: false,
    plugins: entries,
  };
}
