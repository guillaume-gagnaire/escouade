import { gzipSync } from 'node:zlib';
import { eq, lt, sql } from 'drizzle-orm';
import { accounts, installsDaily, plugins, versions } from '../db/schema';
import type { Db } from './db';
import { parseRevocations, revoked, type RevocationList } from './revocation-list';
import { lastRevocations } from './revocations';
import { compareVersions, sha256, type Binary } from './versions';

// The catalogue the app downloads whole (docs/STORE.md, §1.2 and §5.1): every published plugin with its published
// versions, what each asks for and the Escouade it is for (§8.1: the app picks the last compatible one); the search
// runs on the user's machine. One JSON, compressed, with an ETag: an app that has it gets a 304. Its bytes change
// only when something does (a version published, a revocation list, the day's installations): the installations
// counted are those of the days before today, so the catalogue stays the same through a day.

/** Where an app finds a version's files, from the store's address: the paths of server/api/v1/plugins/. */
export const filePaths = (id: string, version: string) => {
  const base = `/api/v1/plugins/${encodeURIComponent(id)}/${encodeURIComponent(version)}`;
  return { versionJson: `${base}/version.json`, archive: `${base}/archive.tar.gz`, binary: (p: string) => `${base}/binaries/${p}` };
};

export interface CatalogVersion {
  version: string;
  published_at: string;
  /** The SHA-256 of its `version.json`: the version's fingerprint, what a signature covers. */
  sha256: string;
  archive: { sha256: string; size: number };
  /** The systems it has a program for (`windows-x64`…); empty for a plugin whose program is in its repository, or none. */
  platforms: string[];
  /** The protocol it speaks and the Escouade versions it was tried on (`max` absent: no upper bound). */
  escouade: { api: string; min: string; max?: string };
  /** Signed by the review (STORE-8 sets it from the signatures it takes in); false until then. */
  verified: boolean;
  /** Why its badge was withdrawn by the revocation list, if it was. */
  unverified?: { fr: string; en: string };
  /** What it asks for (the manifest's `permissions`), whether it runs a program, what it adds. */
  permissions: Record<string, unknown>;
  run: boolean;
  contributes: string[];
  files: { version_json: string; archive: string; binaries: Record<string, string> };
}

export interface CatalogPlugin {
  id: string;
  name: unknown;
  description: unknown;
  author: string | null;
  repository: string;
  license: string | null;
  categories: string[];
  keywords: string[];
  /** Installations and updates counted until `installs_until` (only those whose users accepted the statistics). */
  installs: number;
  /** The highest version published (a release before a pre-release); the app picks among `versions` what it can run. */
  latest: string;
  versions: CatalogVersion[];
}

export interface Catalog {
  format: 1;
  /** When it last changed: a publication, a revocation list, or the day's counts. */
  updated_at: string;
  installs_until: string;
  revocations: { serial: number } | null;
  plugins: CatalogPlugin[];
}

const day = (d: Date) => d.toISOString().slice(0, 10);
const isPre = (v: string) => /^\d+\.\d+\.\d+-/.test(v);

/** Builds the catalogue as of `now`: the published versions, less those the revocation list disables. */
export async function buildCatalog(db: Db, now = new Date()): Promise<Catalog> {
  const today = day(now);
  const yesterday = day(new Date(Date.parse(`${today}T00:00:00Z`) - 86_400_000));
  const rows = await db
    .select({ v: versions, plugin: plugins, login: accounts.login })
    .from(versions)
    .innerJoin(plugins, eq(plugins.id, versions.pluginId))
    .leftJoin(accounts, eq(accounts.id, plugins.accountId))
    .where(eq(versions.state, 'published'));
  const counts = await db
    .select({ id: installsDaily.pluginId, n: sql<number>`sum(${installsDaily.count})::int` })
    .from(installsDaily)
    .where(lt(installsDaily.day, today))
    .groupBy(installsDaily.pluginId);
  const installs = new Map(counts.map((c) => [c.id, Number(c.n)]));
  const stored = await lastRevocations(db);
  const parsed = stored ? parseRevocations(stored.json) : null;
  const list: RevocationList | null = parsed && !('error' in parsed) ? parsed : null;

  let updated = stored?.receivedAt ?? new Date(0);
  const byPlugin = new Map<string, CatalogPlugin>();
  const manifests = new Map<string, { version: string; manifest: Record<string, unknown> }>();
  for (const { v, plugin, login } of rows) {
    const version = v.version!;
    if (revoked(list, plugin.id, version, 'disable')) continue;
    const m = (v.manifest ?? {}) as Record<string, unknown>;
    const escouade = (m.escouade ?? {}) as { api: string; min: string; max?: string };
    const binaries = (v.binaries ?? {}) as Record<string, Binary>;
    const paths = filePaths(plugin.id, version);
    const unverify = revoked(list, plugin.id, version, 'unverify');
    const entry: CatalogVersion = {
      version,
      published_at: v.publishedAt!.toISOString(),
      sha256: v.versionSha256!,
      archive: { sha256: v.archiveSha256!, size: v.archiveSize! },
      platforms: Object.keys(binaries).sort(),
      escouade: { api: escouade.api, min: escouade.min, ...(escouade.max ? { max: escouade.max } : {}) },
      verified: false,
      ...(unverify ? { unverified: unverify.reason } : {}),
      permissions: (m.permissions ?? {}) as Record<string, unknown>,
      run: !!m.run,
      contributes: Object.keys((m.contributes ?? {}) as object).sort(),
      files: {
        version_json: paths.versionJson,
        archive: paths.archive,
        binaries: Object.fromEntries(
          Object.keys(binaries)
            .sort()
            .map((p) => [p, paths.binary(p)]),
        ),
      },
    };
    if (v.publishedAt! > updated) updated = v.publishedAt!;
    let p = byPlugin.get(plugin.id);
    if (!p) {
      p = {
        id: plugin.id,
        name: null,
        description: null,
        author: login ?? null,
        repository: `https://github.com/${plugin.repoFullName}`,
        license: null,
        categories: plugin.categories,
        keywords: plugin.keywords,
        installs: installs.get(plugin.id) ?? 0,
        latest: version,
        versions: [],
      };
      byPlugin.set(plugin.id, p);
    }
    p.versions.push(entry);
    const best = manifests.get(plugin.id);
    if (!best || newer(version, best.version)) manifests.set(plugin.id, { version, manifest: m });
  }
  for (const p of byPlugin.values()) {
    p.versions.sort((a, b) => compareVersions(b.version, a.version));
    const m = manifests.get(p.id)!;
    p.latest = m.version;
    p.name = m.manifest.name ?? p.id;
    p.description = m.manifest.description ?? '';
    p.license = typeof m.manifest.license === 'string' ? m.manifest.license : null;
  }
  // The counts move once a day: the catalogue says so from the day's start.
  const dayStart = new Date(`${today}T00:00:00Z`);
  if (installs.size && dayStart > updated) updated = dayStart;
  return {
    format: 1,
    updated_at: updated.toISOString(),
    installs_until: yesterday,
    revocations: stored ? { serial: stored.serial } : null,
    plugins: [...byPlugin.values()].sort((a, b) => a.id.localeCompare(b.id)),
  };
}

/** Whether `a` should be shown before `b` as the plugin's latest: a release beats a pre-release, then semver. */
function newer(a: string, b: string): boolean {
  if (isPre(a) !== isPre(b)) return !isPre(a);
  return compareVersions(a, b) > 0;
}

export interface Built {
  body: string;
  gzip: Buffer;
  etag: string;
  at: number;
}

/** How long a built catalogue is served before it is built again: a change shows within a minute. */
export const CATALOG_TTL = 60_000;
let cached: Built | null = null;

/** The catalogue's bytes, compressed, and its ETag (the SHA-256 of the bytes): built again after `CATALOG_TTL`. */
export async function catalogBytes(db: Db, now = new Date()): Promise<Built> {
  if (cached && now.getTime() - cached.at < CATALOG_TTL) return cached;
  const body = JSON.stringify(await buildCatalog(db, now));
  cached = { body, gzip: gzipSync(body, { level: 9 }), etag: `"${sha256(body)}"`, at: now.getTime() };
  return cached;
}

/** Forgets the catalogue built: the next request builds it again (a publication, a revocation list, a test). */
export const forgetCatalog = () => {
  cached = null;
};
