import { mkdir, rename, stat, writeFile } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import { and, desc, eq, gte, inArray, isNotNull, ne, sql } from 'drizzle-orm';
import { accountInstallations, accounts, installationRepos, installations, plugins, versions } from '../db/schema';
import { note, type Account } from './accounts';
import { judgeAttestations } from './attest';
import {
  CHECK_KEYS,
  iconOf,
  manifestOf,
  whole,
  platformOf,
  programs,
  rejectionText,
  runChecks,
  screenshotPaths,
  textOf,
  verdict,
  type CheckKey,
  type CheckResult,
  type FetchedBinary,
  type Snapshot,
} from './checks';
import type { Env } from './config';
import type { Db } from './db';
import { GithubError } from './github';
import { imageInfo, reencode, reencodeIcon } from './images';
import { authorMail, rejectionMail, sendMail, tagMovedMail, type AuthorEvent } from './mail';
import { renderDocs, type KeepImage } from './readme';
import { TooBig, type Source } from './source';
import { ARCHIVE_MAX, readArchive, type ArchiveRead } from './tar';
import { forgetCatalog } from './catalog';
import { announce } from './discord';
import { compareVersions, sha256, tagVersion, versionFile, type Binary, type Platform } from './versions';

// The submission of a version (docs/STORE.md, §3): an author picks a public repository the App reads and one of its
// tags; the store resolves the tag into its commit once, downloads the commit's archive once and keeps it as it came,
// with its SHA-256, takes the release's binaries the same way, runs the ten checks on them without running anything,
// and, if they pass, writes `version.json` and its fingerprint. The version then waits for its author's click (or is
// published at once, for a plugin whose author asked it for new tags). A new tag pushed on the repository comes the
// same way, by the App's webhooks; a tag moved after its version was published is noted and told, nothing else moves.

/** Submissions an account may start in an hour: enough to fix and retry, too few to make the store a downloader. */
export const SUBMISSIONS_PER_HOUR = 20;
/** A binary of the release: 50 MB at most, as the archive. */
export const BINARY_MAX = 50 * 1024 * 1024;
/** Binaries a version may bring: three systems, two architectures. */
export const BINARIES_MAX = 6;

/** The categories of the store (choice 214 of the preview). */
export const CATEGORIES = ['harness', 'ticket-sources', 'forges', 'themes', 'notifications', 'permissions', 'tools', 'languages'] as const;
export const KEYWORDS_MAX = 8;

export type Version = typeof versions.$inferSelect;
export type Plugin = typeof plugins.$inferSelect;

export interface Deps {
  /** The repositories of an installation, read through the App (tests: a fake). */
  source: (installationId: number) => Source;
  /** Where the archives, binaries and screenshots are kept (the volume, /data/archives). */
  archives: string;
  env: Env;
  /** The store's address, for the links of the e-mails. */
  site: string;
}

// ---- What the author can pick ----

export interface RepoChoice {
  fullName: string;
  repoId: number;
  installationId: number;
  private: boolean;
  suspended: boolean;
}

/** The repositories the App reads among the account's installations (the private ones shown, and refused). */
export async function repoChoices(db: Db, accountId: number): Promise<RepoChoice[]> {
  const rows = await db
    .select({ repo: installationRepos, suspendedAt: installations.suspendedAt })
    .from(accountInstallations)
    .innerJoin(installations, eq(installations.id, accountInstallations.installationId))
    .innerJoin(installationRepos, eq(installationRepos.installationId, installations.id))
    .where(eq(accountInstallations.accountId, accountId))
    .orderBy(installationRepos.fullName);
  return rows.map(({ repo, suspendedAt }) => ({
    fullName: repo.fullName,
    repoId: repo.repoId,
    installationId: repo.installationId,
    private: repo.private,
    suspended: suspendedAt !== null,
  }));
}

/** The repository `fullName` if the account may submit from it; else why not. */
export async function pickRepo(
  db: Db,
  accountId: number,
  fullName: unknown,
): Promise<RepoChoice | { error: 'not-yours' | 'private' | 'suspended' }> {
  if (typeof fullName !== 'string') return { error: 'not-yours' };
  const repo = (await repoChoices(db, accountId)).find((r) => r.fullName.toLowerCase() === fullName.trim().toLowerCase());
  if (!repo) return { error: 'not-yours' };
  if (repo.suspended) return { error: 'suspended' };
  if (repo.private) return { error: 'private' };
  return repo;
}

// ---- Starting a submission ----

export type Started = { id: number; again: boolean } | { error: 'not-yours' | 'private' | 'suspended' | 'too-many' | 'no-tag' | 'github' };

/**
 * Starts the submission of `tag` of `repo` by `account`: the tag resolved into its commit, the version written down,
 * `checking`. The same tag already submitted and not refused gives that version back (`again`). The caller sends the
 * job that checks it (`checkVersion`).
 */
export async function startSubmission(
  db: Db,
  deps: Pick<Deps, 'source'>,
  account: Account,
  input: { repo: unknown; tag: unknown },
  now = new Date(),
): Promise<Started> {
  const picked = await pickRepo(db, account.id, input.repo);
  if ('error' in picked) return picked;
  if (typeof input.tag !== 'string' || !input.tag.trim() || input.tag.length > 200) return { error: 'no-tag' };
  const tag = input.tag.trim();
  const [recent] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(versions)
    .where(
      and(eq(versions.accountId, account.id), eq(versions.origin, 'author'), gte(versions.createdAt, new Date(now.getTime() - 3600_000))),
    );
  if ((recent?.n ?? 0) >= SUBMISSIONS_PER_HOUR) return { error: 'too-many' };

  const source = deps.source(picked.installationId);
  let resolved: { commit: string; date: string | null };
  try {
    // GitHub's word on the repository, not the last webhook's: a repository made private since is refused.
    const repo = await source.repo(picked.fullName);
    if (repo.private) return { error: 'private' };
    resolved = await source.resolve(picked.fullName, tag);
  } catch (e) {
    if (e instanceof GithubError && e.status === 404) return { error: 'no-tag' };
    console.error('[store] submission:', e instanceof Error ? e.message : e);
    return { error: 'github' };
  }
  const [same] = await db
    .select({ id: versions.id })
    .from(versions)
    .where(
      and(
        eq(versions.repoId, picked.repoId),
        eq(versions.tag, tag),
        eq(versions.commit, resolved.commit),
        inArray(versions.state, ['checking', 'held', 'ready', 'published']),
      ),
    )
    .orderBy(desc(versions.id))
    .limit(1);
  if (same) return { id: same.id, again: true };
  const id = await insertVersion(db, {
    accountId: account.id,
    origin: 'author',
    installationId: picked.installationId,
    repoId: picked.repoId,
    repoFullName: picked.fullName,
    tag,
    commit: resolved.commit,
    tagDate: resolved.date ? new Date(resolved.date) : null,
    createdAt: now,
  });
  await note(
    db,
    'version.submitted',
    { accountId: account.id, githubId: account.githubId },
    { repository: picked.fullName, tag, commit: resolved.commit, version: id },
    now,
  );
  return { id, again: false };
}

async function insertVersion(db: Db, row: typeof versions.$inferInsert): Promise<number> {
  const running = CHECK_KEYS.map((key) => ({ key, status: 'run', detail: { fr: '', en: '' }, problems: [] }));
  const [v] = await db
    .insert(versions)
    .values({ ...row, state: 'checking', step: 'archive', checks: running })
    .returning({ id: versions.id });
  return v!.id;
}

/** Runs the checks of a version again, once the store's own failure has passed (`error`), or after a rejection. */
export async function retrySubmission(db: Db, account: Account, id: number): Promise<boolean> {
  const running = CHECK_KEYS.map((key) => ({ key, status: 'run', detail: { fr: '', en: '' }, problems: [] }));
  const done = await db
    .update(versions)
    .set({ state: 'checking', step: 'archive', error: null, checks: running })
    .where(and(eq(versions.id, id), eq(versions.accountId, account.id), eq(versions.state, 'error')))
    .returning({ id: versions.id });
  return done.length > 0;
}

// ---- Keeping the files ----

/** Keeps `bytes` once under their SHA-256 (`sha256/ab/abcd….tar.gz`): what is kept never changes, two versions share it. */
export async function keep(dir: string, bytes: Buffer, ext: string): Promise<{ sha256: string; path: string }> {
  const hash = sha256(bytes);
  const path = join(dir, 'sha256', hash.slice(0, 2), `${hash}${ext}`);
  const there = await stat(path).then(
    (s) => s.size === bytes.length,
    () => false,
  );
  if (!there) {
    await mkdir(dirname(path), { recursive: true });
    const tmp = `${path}.${process.pid}.${Date.now()}.part`;
    await writeFile(tmp, bytes, { flag: 'wx' });
    await rename(tmp, path);
  }
  return { sha256: hash, path };
}

/** The extension a file is kept under, from its name in the release. */
export const ext = (name: string) => /\.(tar\.gz|tgz|zip|tar\.xz|tar\.zst|exe)$/i.exec(name)?.[0].toLowerCase() ?? '.bin';

/** Keeps a README's image as a screenshot is kept; gives the name it is served under (`<sha256>.png`). */
export const keepImage =
  (dir: string): KeepImage =>
  async (bytes, format) =>
    basename((await keep(dir, bytes, `.${format === 'jpeg' ? 'jpg' : format}`)).path);

// ---- The checks ----

const running = (results: CheckResult[], waiting: CheckKey[]): CheckResult[] =>
  results.map((r) => (waiting.includes(r.key) ? { key: r.key, status: 'run' as const, detail: { fr: '', en: '' }, problems: [] } : r));

/**
 * The job of a version: download, freeze, check, and write down where it stands. Nothing of the plugin runs; its
 * archive is read in memory. A failure of the store or of GitHub (not of the plugin) leaves it in `error`, to retry.
 */
export async function checkVersion(db: Db, deps: Deps, id: number, now = () => new Date()): Promise<Version['state']> {
  const [v] = await db.select().from(versions).where(eq(versions.id, id));
  if (!v || v.state !== 'checking') return v?.state ?? 'error';
  const source = deps.source(v.installationId);
  const step = (name: string, checks?: CheckResult[]) =>
    db
      .update(versions)
      .set({ step: name, ...(checks ? { checks } : {}) })
      .where(eq(versions.id, id));
  try {
    // 1. The archive, once, as GitHub sends it.
    let gz: Buffer | null = null;
    let read: ArchiveRead;
    let size: number;
    try {
      gz = await source.tarball(v.repoFullName, v.commit, ARCHIVE_MAX);
      read = readArchive(gz);
      size = gz.length;
    } catch (e) {
      if (!(e instanceof TooBig)) throw e;
      read = { files: new Map(), dirs: new Set(), problems: [{ code: 'too-big', size: e.size }], comment: null };
      size = e.size;
    }
    const m = whole(read) ? manifestOf(read) : null;
    const pluginId = typeof m?.id === 'string' ? m.id : null;

    // What the store already knows of this id: its plugin, the numbers taken, the last published; the others' names.
    const [existing] = pluginId ? await db.select().from(plugins).where(eq(plugins.id, pluginId)) : [];
    const taken = pluginId
      ? await db
          .select({ version: versions.version, state: versions.state })
          .from(versions)
          .where(
            and(
              eq(versions.pluginId, pluginId),
              ne(versions.id, id),
              inArray(versions.state, ['published', 'ready', 'held']),
              isNotNull(versions.version),
            ),
          )
      : [];
    // The numbers of a plugin deleted and purged stay used: its id comes back with them.
    for (const r of existing?.retiredVersions ?? []) taken.push({ version: r, state: 'published' });
    const published = taken.filter((x) => x.state === 'published').map((x) => x.version!);
    const latest = published.sort(compareVersions).at(-1) ?? null;
    const others = await otherPlugins(db, v.accountId);
    const snapshot: Snapshot = {
      repository: v.repoFullName,
      tag: v.tag,
      commit: v.commit,
      archive: { size, read },
      license: null,
      binaries: [],
      releaseAssets: null,
      existing: existing
        ? { mine: existing.accountId !== null && existing.accountId === v.accountId, repository: existing.repoFullName }
        : null,
      taken: taken.map((x) => x.version!),
      latest,
      others,
    };
    await step('license', running(runChecks(snapshot), ['license', 'screenshots', 'binaries']));

    // 2. The license GitHub detects in the commit.
    snapshot.license = m ? await source.license(v.repoFullName, v.commit) : null;
    await step('binaries', running(runChecks(snapshot), ['screenshots', 'binaries']));

    // 3. The release's binaries, for the systems whose program is not in the archive: kept, and their attestations read.
    const fetched: { bin: FetchedBinary; bytes: Buffer }[] = [];
    const needs = programs(m, read).filter((p) => !p.inArchive);
    if (needs.length) {
      const assets = await source.release(v.repoFullName, v.tag);
      snapshot.releaseAssets = assets?.map((a) => a.name) ?? null;
      const wanted = (assets ?? [])
        .map((a) => ({ a, platform: platformOf(a.name) }))
        .filter(
          (x): x is { a: (typeof x)['a']; platform: Platform } => !!x.platform && needs.some((n) => x.platform!.startsWith(`${n.system}-`)),
        );
      const seen = new Set<Platform>();
      snapshot.oversized = [];
      for (const { a, platform } of wanted) {
        if (seen.has(platform) || seen.size >= BINARIES_MAX) continue;
        seen.add(platform);
        let bytes: Buffer;
        try {
          bytes = await source.asset(a, BINARY_MAX);
        } catch (e) {
          if (!(e instanceof TooBig)) throw e;
          snapshot.oversized.push(a.name);
          continue;
        }
        const hash = sha256(bytes);
        const attestation = judgeAttestations(await source.attestations(v.repoFullName, hash), {
          sha256: hash,
          repository: v.repoFullName,
          commit: v.commit,
        });
        fetched.push({ bin: { platform, name: a.name, sha256: hash, size: bytes.length, attestation }, bytes });
      }
      snapshot.binaries = fetched.map((f) => f.bin);
    }
    await step('screenshots', running(runChecks(snapshot), ['screenshots']));

    // 4. The screenshots, re-encoded (their metadata dropped) once they passed their own check.
    let results = runChecks(snapshot);
    const shots: { path: string; bytes: Buffer; format: string; width: number; height: number }[] = [];
    if (results.find((r) => r.key === 'screenshots')?.status === 'pass') {
      const undecodable: string[] = [];
      for (const path of screenshotPaths(m)) {
        const raw = read.files.get(path)!;
        const info = imageInfo(raw)!;
        try {
          const bytes = await reencode(raw, info.format);
          const out = imageInfo(bytes) ?? info;
          shots.push({ path, bytes, format: out.format, width: out.width, height: out.height });
        } catch {
          undecodable.push(path);
        }
      }
      if (undecodable.length) snapshot.undecodable = undecodable;
    }
    // The icon, decoded within a bound of pixels and re-encoded without its metadata at 256 and 64 px, once the
    // manifest's check read its header (PLG-13).
    const iconAt = iconOf(m);
    let icon: Record<'256' | '64', Buffer> | null = null;
    if (iconAt && results.find((r) => r.key === 'manifest')?.status === 'pass') {
      icon = await reencodeIcon(read.files.get(iconAt)!).catch(() => null);
      if (!icon) snapshot.iconUndecodable = true;
    }
    if (snapshot.undecodable || snapshot.iconUndecodable) results = runChecks(snapshot);

    // 5. The verdict; for a version that may be published, its files kept and its version.json.
    const state = verdict(results);
    const base = {
      pluginId,
      version: typeof m?.version === 'string' ? m.version : null,
      manifest: m,
      manifestSha256: read.files.has('escouade-plugin.json') ? sha256(read.files.get('escouade-plugin.json')!) : null,
      archiveSha256: gz ? sha256(gz) : null,
      archiveSize: size,
      checks: results,
      step: null,
      checkedAt: now(),
    };
    if (state === 'rejected') {
      await db
        .update(versions)
        .set({ ...base, state })
        .where(eq(versions.id, id));
      await tellRejection(db, deps, { ...v, ...base }, results);
      await note(
        db,
        'version.rejected',
        { accountId: v.accountId },
        { version: id, plugin: pluginId, failed: results.filter((r) => r.status !== 'pass').map((r) => r.key) },
      );
      return state;
    }
    await keep(deps.archives, gz!, '.tar.gz');
    const binaries: Partial<Record<Platform, Binary>> = {};
    for (const { bin, bytes } of fetched) {
      await keep(deps.archives, bytes, ext(bin.name));
      binaries[bin.platform] = { name: bin.name, sha256: bin.sha256, size: bin.size, attested: bin.attestation.state === 'attested' };
    }
    const screenshots = [];
    for (const s of shots) {
      const kept = await keep(deps.archives, s.bytes, `.${s.format === 'jpeg' ? 'jpg' : s.format}`);
      screenshots.push({ path: s.path, sha256: kept.sha256, format: s.format, width: s.width, height: s.height });
    }
    const iconKept = icon
      ? { '256': (await keep(deps.archives, icon['256'], '.png')).sha256, '64': (await keep(deps.archives, icon['64'], '.png')).sha256 }
      : null;
    // The README and the documentation, rendered once and cut down, their images copied (server/utils/readme.ts).
    const docs = await renderDocs(read, { repo: v.repoFullName, commit: v.commit, keep: keepImage(deps.archives) });
    const escouade = m!.escouade as { api: string; min: string; max?: string };
    const file = versionFile({
      id: pluginId!,
      version: base.version!,
      repository: `https://github.com/${v.repoFullName}`,
      tag: v.tag,
      commit: v.commit,
      archive: { sha256: base.archiveSha256!, size },
      binaries,
      manifest_sha256: base.manifestSha256!,
      escouade: { api: escouade.api, min: escouade.min, ...(escouade.max ? { max: escouade.max } : {}) },
      ...(iconKept ? { icon: iconKept } : {}),
    });
    await db
      .update(versions)
      .set({ ...base, state, binaries, screenshots, icon: iconKept, docs, versionJson: file.json, versionSha256: file.sha256 })
      .where(eq(versions.id, id));
    await note(db, `version.${state}`, { accountId: v.accountId }, { version: id, plugin: pluginId, sha256: file.sha256 });
    if (state === 'held') void announce({ kind: 'held', version: id });
    if (state === 'ready' && v.origin === 'webhook' && existing) {
      // A transfer freezes the publications 7 days: the version waits for a click once the freeze is over.
      const frozen = !!existing.frozenUntil && existing.frozenUntil > now();
      if (existing.autoPublish && !frozen && !existing.deletedAt) {
        await markPublished(db, id, now());
        await note(db, 'version.published', { accountId: v.accountId }, { version: id, plugin: pluginId, by: 'auto' });
        await tellAuthor(db, deps, v.accountId, 'auto-published', pluginId!, base.version!);
        void announce({ kind: 'new-version', version: id });
        void announce({ kind: 'review', version: id });
        return 'published';
      }
      await tellAuthor(db, deps, v.accountId, 'ready', pluginId!, base.version!);
    }
    return state;
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    console.error(`[store] checks of version ${id} stopped:`, message);
    await db
      .update(versions)
      .set({ state: 'error', error: message.slice(0, 300) })
      .where(eq(versions.id, id));
    return 'error';
  }
}

/** The plugins of the other accounts, by id and name, for the names too close. */
export async function otherPlugins(db: Db, accountId: number | null): Promise<Snapshot['others']> {
  const rows = await db
    .select({ id: plugins.id, accountId: plugins.accountId, manifest: versions.manifest })
    .from(plugins)
    .leftJoin(versions, and(eq(versions.pluginId, plugins.id), eq(versions.state, 'published')));
  const byId = new Map<string, string>();
  for (const r of rows)
    if (r.accountId !== accountId || accountId === null) byId.set(r.id, textOf(r.manifest?.name) || byId.get(r.id) || '');
  return [...byId].map(([id, name]) => ({ id, name }));
}

/** Tells the author of an event of their plugin by e-mail, if they gave an address that was checked. */
async function tellAuthor(
  db: Db,
  deps: Pick<Deps, 'env' | 'site'>,
  accountId: number | null,
  event: AuthorEvent,
  plugin: string,
  version: string,
) {
  if (!accountId) return;
  const [a] = await db.select().from(accounts).where(eq(accounts.id, accountId));
  if (a?.email && a.emailVerifiedAt)
    await sendMail(
      deps.env,
      authorMail(a.email, event, { plugin, version, page: `${deps.site}/account?plugin=${encodeURIComponent(plugin)}` }),
    );
}

async function tellRejection(db: Db, deps: Deps, v: Version, results: CheckResult[]): Promise<void> {
  if (!v.accountId) return;
  const [a] = await db.select().from(accounts).where(eq(accounts.id, v.accountId));
  if (!a?.email || !a.emailVerifiedAt) return;
  await sendMail(
    deps.env,
    rejectionMail(a.email, {
      plugin: v.pluginId ?? v.repoFullName,
      tag: v.tag,
      page: `${deps.site}/submit?version=${v.id}`,
      fr: rejectionText(results, 'fr'),
      en: rejectionText(results, 'en'),
    }),
  );
}

// ---- Publishing ----

export type Published = { ok: true } | { error: 'not-found' | 'not-ready' | 'held' | 'categories' | 'taken' | 'frozen' | 'deleted' };

async function markPublished(db: Db, id: number, now: Date) {
  await db.update(versions).set({ state: 'published', publishedAt: now }).where(eq(versions.id, id));
  forgetCatalog();
}

/** The words of a form: a list, or a text cut by commas. */
const words = (v: unknown): string[] =>
  (Array.isArray(v) ? v : typeof v === 'string' ? v.split(',') : [])
    .filter((x): x is string => typeof x === 'string')
    .map((x) => x.trim().toLowerCase())
    .filter(Boolean);

/**
 * Publishes a version that passed its checks, on its author's click. A first version makes the plugin, with its
 * categories (1 to 3, of the store's) and keywords; `review` puts the version in the review queue (checked by
 * default); `autoPublish` lets the next tags be published without a click. Unverified until the review signs it.
 */
export async function publishVersion(
  db: Db,
  account: Account,
  id: number,
  form: { categories?: unknown; keywords?: unknown; review?: unknown; autoPublish?: unknown },
  now = new Date(),
  tell: Pick<Deps, 'env' | 'site'> | null = null,
): Promise<Published> {
  const [v] = await db.select().from(versions).where(eq(versions.id, id));
  if (!v || !v.pluginId) return { error: 'not-found' };
  const [plugin] = await db.select().from(plugins).where(eq(plugins.id, v.pluginId));
  if (plugin ? plugin.accountId !== account.id : v.accountId !== account.id) return { error: 'not-found' };
  if (v.state === 'held') return { error: 'held' };
  if (plugin?.deletedAt) return { error: 'deleted' };
  if (plugin?.frozenUntil && plugin.frozenUntil > now) return { error: 'frozen' };
  if (v.state !== 'ready') return { error: 'not-ready' };
  const on = (x: unknown) => x === true || x === 'on' || x === '1' || x === 'true';
  if (!plugin) {
    const categories = [...new Set(words(form.categories))].filter((c) => (CATEGORIES as readonly string[]).includes(c));
    if (categories.length < 1 || categories.length > 3) return { error: 'categories' };
    const keywords = [...new Set(words(form.keywords))]
      .filter((k) => k.length <= 24 && /^[\p{L}\p{N}][\p{L}\p{N} -]*$/u.test(k))
      .slice(0, KEYWORDS_MAX);
    const made = await db
      .insert(plugins)
      .values({
        id: v.pluginId,
        accountId: account.id,
        repoId: v.repoId,
        repoFullName: v.repoFullName,
        installationId: v.installationId,
        categories,
        keywords,
        autoPublish: on(form.autoPublish),
        createdAt: now,
      })
      .onConflictDoNothing()
      .returning({ id: plugins.id });
    if (!made.length) return { error: 'taken' };
  } else if (form.autoPublish !== undefined) {
    await db
      .update(plugins)
      .set({ autoPublish: on(form.autoPublish) })
      .where(eq(plugins.id, plugin.id));
  }
  try {
    await db
      .update(versions)
      .set({ state: 'published', publishedAt: now, review: form.review === undefined ? true : on(form.review) })
      .where(and(eq(versions.id, id), eq(versions.state, 'ready')));
  } catch {
    // The unique index of the published numbers: the same number published meanwhile.
    return { error: 'taken' };
  }
  // The catalogue of the app shows it from the next request (server/utils/catalog.ts).
  forgetCatalog();
  await note(
    db,
    'version.published',
    { accountId: account.id, githubId: account.githubId },
    { version: id, plugin: v.pluginId, sha256: v.versionSha256 },
    now,
  );
  if (tell) await tellAuthor(db, tell, account.id, 'published', v.pluginId, v.version!);
  // Discord, by the queue: the author's click never waits for it.
  void announce({ kind: plugin ? 'new-version' : 'new-plugin', version: id });
  if (form.review === undefined || on(form.review)) void announce({ kind: 'review', version: id });
  return { ok: true };
}

// ---- The tags of the repositories ----

/**
 * A tag pushed or released on a repository the App reads (webhooks `create`, `release`, `push` of a tag). On the
 * repository of a plugin of the store: a tag of a version published (or waiting) that now names another commit is
 * noted as moved, nothing else changes; a new tag becomes a version to check, by the author of the plugin. Gives the
 * version to check, if any.
 */
export async function onTag(
  db: Db,
  deps: Deps,
  hook: { repoId: number; fullName: string; installationId: number | null; tag: string },
  now = new Date(),
): Promise<{ check?: number; moved?: number }> {
  const [plugin] = await db.select().from(plugins).where(eq(plugins.repoId, hook.repoId)).limit(1);
  // A plugin deleted takes no new tag: its author restores it first.
  if (!plugin || plugin.deletedAt) return {};
  const installationId = hook.installationId ?? plugin.installationId;
  if (!installationId) return {};
  const { commit, date } = await deps.source(installationId).resolve(hook.fullName, hook.tag);
  const known = await db
    .select()
    .from(versions)
    .where(
      and(
        eq(versions.repoId, hook.repoId),
        eq(versions.tag, hook.tag),
        inArray(versions.state, ['checking', 'held', 'ready', 'published']),
      ),
    );
  if (known.length) {
    const moved = await noteMoved(db, deps, known, commit, now);
    return moved ? { moved } : {};
  }
  // A tag whose version number is already taken goes through the checks all the same: they say why it is refused.
  const id = await insertVersion(db, {
    pluginId: plugin.id,
    accountId: plugin.accountId,
    origin: 'webhook',
    installationId,
    repoId: hook.repoId,
    repoFullName: hook.fullName,
    tag: hook.tag,
    commit,
    tagDate: date ? new Date(date) : null,
    version: tagVersion(hook.tag),
    createdAt: now,
  });
  await note(db, 'version.tag_seen', { accountId: plugin.accountId }, { plugin: plugin.id, tag: hook.tag, commit, version: id }, now);
  return { check: id };
}

/** The versions of a tag that now names `commit`: the published or waiting ones on another commit are marked moved, once. */
async function noteMoved(db: Db, deps: Deps, known: Version[], commit: string, now: Date): Promise<number | undefined> {
  let first: number | undefined;
  for (const v of known) {
    if (v.commit === commit || v.tagMovedTo === commit) continue;
    await db.update(versions).set({ tagMovedAt: now, tagMovedTo: commit }).where(eq(versions.id, v.id));
    await note(
      db,
      'version.tag_moved',
      { accountId: v.accountId },
      { version: v.id, plugin: v.pluginId, tag: v.tag, was: v.commit, now: commit },
      now,
    );
    first ??= v.id;
    void announce({ kind: 'tag-moved', version: v.id });
    if (!v.accountId || v.state !== 'published') continue;
    const [a] = await db.select().from(accounts).where(eq(accounts.id, v.accountId));
    if (a?.email && a.emailVerifiedAt)
      await sendMail(deps.env, tagMovedMail(a.email, { plugin: v.pluginId ?? v.repoFullName, tag: v.tag, was: v.commit, now: commit }));
  }
  return first;
}

/**
 * Reads again, once a week, the tag of every published version: a tag moved without a webhook (a `git push --force`
 * of a tag sends none of `create` or `release`) is caught here. Gives how many were read and how many moved.
 */
export async function rereadTags(db: Db, deps: Deps, now = new Date()): Promise<{ read: number; moved: number; failed: number }> {
  const rows = await db.select().from(versions).where(eq(versions.state, 'published'));
  let moved = 0;
  let failed = 0;
  for (const v of rows) {
    try {
      const { commit } = await deps.source(v.installationId).resolve(v.repoFullName, v.tag);
      if (await noteMoved(db, deps, [v], commit, now)) moved++;
    } catch (e) {
      // A tag deleted, or a repository the App no longer reads: the version stays as it is.
      failed++;
      console.warn(`[store] tag ${v.repoFullName}@${v.tag} not read again:`, e instanceof Error ? e.message : e);
    }
  }
  return { read: rows.length, moved, failed };
}

// ---- What the submission's page shows ----

/** A version as its author sees it on the submission's page; null if it is not theirs. */
export async function versionView(db: Db, account: Account, id: number) {
  const [v] = await db.select().from(versions).where(eq(versions.id, id));
  if (!v) return null;
  const [plugin] = v.pluginId ? await db.select().from(plugins).where(eq(plugins.id, v.pluginId)) : [];
  if (v.accountId !== account.id && plugin?.accountId !== account.id) return null;
  const m = v.manifest;
  const store = (m?.store ?? {}) as Record<string, unknown>;
  const [previous] = v.pluginId
    ? await db
        .select({ version: versions.version })
        .from(versions)
        .where(and(eq(versions.pluginId, v.pluginId), eq(versions.state, 'published'), ne(versions.id, v.id)))
        .orderBy(desc(versions.publishedAt))
        .limit(1)
    : [];
  return {
    id: v.id,
    state: v.state,
    step: v.step,
    error: v.error,
    origin: v.origin,
    repository: v.repoFullName,
    tag: v.tag,
    commit: v.commit,
    tagDate: v.tagDate,
    pluginId: v.pluginId,
    name: m ? textOf(m.name) : null,
    version: v.version,
    archive: v.archiveSha256 ? { sha256: v.archiveSha256, size: v.archiveSize } : null,
    fingerprint: v.versionSha256,
    checks: v.checks as CheckResult[],
    first: !plugin,
    previous: previous?.version ?? null,
    plugin: plugin ? { categories: plugin.categories, keywords: plugin.keywords, autoPublish: plugin.autoPublish } : null,
    suggested: {
      categories: Array.isArray(store.categories)
        ? store.categories.filter((c) => (CATEGORIES as readonly string[]).includes(c as string))
        : [],
      keywords: Array.isArray(store.keywords) ? store.keywords.filter((k) => typeof k === 'string') : [],
    },
    review: v.review,
    publishedAt: v.publishedAt,
    tagMovedAt: v.tagMovedAt,
    tagMovedTo: v.tagMovedTo,
  };
}

/** The account's last submissions, newest first, for the first step of the page. */
export async function recentSubmissions(db: Db, accountId: number, limit = 10) {
  return db
    .select({
      id: versions.id,
      repository: versions.repoFullName,
      tag: versions.tag,
      state: versions.state,
      pluginId: versions.pluginId,
      createdAt: versions.createdAt,
    })
    .from(versions)
    .where(eq(versions.accountId, accountId))
    .orderBy(desc(versions.id))
    .limit(limit);
}
