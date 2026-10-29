import { compareVersions, sha256, type VersionFile } from '../../server/utils/versions';
import type { Catalog, CatalogPlugin, CatalogVersion } from '../../server/utils/catalog';

// Downloads what the review reads (docs/STORE.md §4.2, step 1): the catalogue's entry of the version, its
// `version.json`, its archive and its binaries, from the store's app API (/api/v1, STORE-6), and checks every
// fingerprint: `version.json` against the catalogue, the archive and the binaries against `version.json`. A
// fingerprint that does not hold stops the review: what would be read is not what the store hands out. Then the tag on
// GitHub, compared to the commit the store froze (a tag moved after the submission shows here).

export interface Downloaded {
  plugin: CatalogPlugin;
  entry: CatalogVersion;
  /** `version.json`, byte for byte, and what it says. */
  json: string;
  file: VersionFile;
  archive: Buffer;
  /** By platform (`linux-x64`…), as `version.json` names them. */
  binaries: Record<string, Buffer>;
}

/** A fingerprint or an identity that does not hold: the review stops there. */
export class Integrity extends Error {}

export interface Source {
  store: string;
  /** GitHub's API, for the tag; a fake one in the tests. */
  githubApi: string;
  githubToken?: string;
  fetch?: typeof fetch;
}

const MAX = 60 * 1024 * 1024;

async function get(src: Source, url: string, what: string): Promise<Buffer> {
  const res = await (src.fetch ?? fetch)(url, { headers: { 'user-agent': 'escouade-plugin-review' }, redirect: 'error' });
  if (!res.ok) throw new Integrity(`${what} : ${res.status} pour ${url}`);
  const len = Number(res.headers.get('content-length') ?? 0);
  if (len > MAX) throw new Integrity(`${what} : ${len} octets, au-delà de ${MAX}`);
  const body = Buffer.from(await res.arrayBuffer());
  if (body.length > MAX) throw new Integrity(`${what} : ${body.length} octets, au-delà de ${MAX}`);
  return body;
}

/** `<id>@<version>`, as the queue gives it to copy; null if it is not one. */
export function parseTarget(s: string): { id: string; version: string } | null {
  const m = /^([a-z0-9][a-z0-9.-]*[a-z0-9])@v?(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/.exec(s.trim());
  return m ? { id: m[1]!, version: m[2]! } : null;
}

/** The catalogue as the store serves it. */
export async function catalog(src: Source): Promise<Catalog> {
  const raw = await get(src, `${src.store}/api/v1/catalog.json`, 'Catalogue');
  try {
    return JSON.parse(raw.toString('utf8')) as Catalog;
  } catch {
    throw new Integrity('Catalogue illisible');
  }
}

/** Downloads version `version` of plugin `id` and checks every fingerprint; throws `Integrity` when one fails. */
export async function download(src: Source, cat: Catalog, id: string, version: string): Promise<Downloaded> {
  const plugin = cat.plugins.find((p) => p.id === id);
  if (!plugin) throw new Integrity(`${id} n’est pas au catalogue du store`);
  const entry = plugin.versions.find((v) => v.version === version);
  if (!entry)
    throw new Integrity(
      `${id}@${version} n’est pas au catalogue (versions : ${plugin.versions.map((v) => v.version).join(', ') || 'aucune'})`,
    );

  const json = (await get(src, `${src.store}${entry.files.version_json}`, 'version.json')).toString('utf8');
  const got = sha256(json);
  if (got !== entry.sha256) throw new Integrity(`version.json : empreinte ${got}, le catalogue dit ${entry.sha256}`);
  let file: VersionFile;
  try {
    file = JSON.parse(json) as VersionFile;
  } catch {
    throw new Integrity('version.json illisible');
  }
  if (file.id !== id || file.version !== version)
    throw new Integrity(`version.json nomme ${file.id}@${file.version}, pas ${id}@${version}`);
  if (!/^[0-9a-f]{40}$/.test(file.commit ?? '')) throw new Integrity(`version.json : commit illisible (${file.commit})`);

  const archive = await get(src, `${src.store}${entry.files.archive}`, 'Archive');
  if (archive.length !== file.archive.size)
    throw new Integrity(`Archive : ${archive.length} octets, version.json en dit ${file.archive.size}`);
  if (sha256(archive) !== file.archive.sha256)
    throw new Integrity(`Archive : empreinte ${sha256(archive)}, version.json dit ${file.archive.sha256}`);

  const binaries: Record<string, Buffer> = {};
  for (const [platform, b] of Object.entries(file.binaries ?? {})) {
    const path = entry.files.binaries[platform];
    if (!path || !b) throw new Integrity(`Binaire ${platform} : absent du catalogue`);
    const bytes = await get(src, `${src.store}${path}`, `Binaire ${platform}`);
    if (sha256(bytes) !== b.sha256) throw new Integrity(`Binaire ${platform} : empreinte ${sha256(bytes)}, version.json dit ${b.sha256}`);
    binaries[platform] = bytes;
  }
  return { plugin, entry, json, file, archive, binaries };
}

/**
 * The version to read the diff from: `since` if given, else the highest version below `version` that the review
 * verified (`verified` in the catalogue); null for a first review.
 */
export function baseVersion(plugin: CatalogPlugin, version: string, since?: string): CatalogVersion | null {
  if (since) {
    const v = plugin.versions.find((x) => x.version === since);
    if (!v) throw new Integrity(`--since ${since} : cette version n’est pas au catalogue`);
    return v;
  }
  return (
    plugin.versions
      .filter((v) => v.verified && compareVersions(v.version, version) < 0)
      .sort((a, b) => compareVersions(b.version, a.version))[0] ?? null
  );
}

export type TagCheck =
  { state: 'same'; commit: string } | { state: 'moved'; commit: string } | { state: 'gone' } | { state: 'unknown'; why: string };

/** Where the tag of `file` points on GitHub now, compared to the commit the store froze. */
export async function checkTag(src: Source, file: VersionFile): Promise<TagCheck> {
  const repo = file.repository.replace(/^https:\/\/github\.com\//i, '').replace(/\.git$/, '');
  if (!/^[\w.-]+\/[\w.-]+$/.test(repo)) return { state: 'unknown', why: `dépôt illisible : ${file.repository}` };
  const headers: Record<string, string> = { accept: 'application/vnd.github+json', 'user-agent': 'escouade-plugin-review' };
  if (src.githubToken) headers.authorization = `Bearer ${src.githubToken}`;
  const f = src.fetch ?? fetch;
  try {
    const res = await f(`${src.githubApi}/repos/${repo}/git/ref/tags/${encodeURIComponent(file.tag)}`, { headers });
    if (res.status === 404) return { state: 'gone' };
    if (!res.ok) return { state: 'unknown', why: `GitHub répond ${res.status}` };
    let obj = ((await res.json()) as { object: { type: string; sha: string } }).object;
    // An annotated tag points at a tag object, which points at the commit.
    for (let i = 0; obj.type === 'tag' && i < 3; i++) {
      const t = await f(`${src.githubApi}/repos/${repo}/git/tags/${obj.sha}`, { headers });
      if (!t.ok) return { state: 'unknown', why: `GitHub répond ${t.status} pour l’étiquette annotée` };
      obj = ((await t.json()) as { object: { type: string; sha: string } }).object;
    }
    return obj.sha === file.commit ? { state: 'same', commit: obj.sha } : { state: 'moved', commit: obj.sha };
  } catch (e) {
    return { state: 'unknown', why: (e as Error).message };
  }
}
