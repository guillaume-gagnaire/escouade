// The app's seed of plugins (ESC-338, docs/PLUGINS.md §8.7): what the installer carries so that Escouade works from its
// first start, with no network.
//
//   npx tsx store/scripts/seed.ts fetch [--list plugins/seed.json] [--to src-tauri/seed] [--store https://store.escouade.dev]
//                                 [--mirror https://escouade.dev] [--keys store/keys]
//   npx tsx store/scripts/seed.ts dev <plugin folder>… [--to src-tauri/seed]
//
// `fetch` is the release's: it downloads the versions plugins/seed.json names, byte for byte, with the revocation list of
// the day, and checks all of it as the app will: version.json against its fingerprint, its signature with the store's public
// keys, the archive and the module against version.json. A gap, a version withdrawn or one this Escouade cannot run stops
// the release, which is the point: the chef publishes first, the release comes after.
//
// `dev` is scripts/seed-dev.sh's: the same files, made here from folders with their plugin.wasm, not signed, for a
// build of development, which alone takes them; it writes its own seed.json beside them.

import { createHash } from 'node:crypto';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readPublicKey, readSignature, verifySigned, type PublicKey } from '../server/utils/minisign';
import { checkRevocations, revoked, type RevocationList } from '../server/utils/revocation-list';
import { tarball } from '../tests/fake/plugins.mjs';

export interface Entry {
  id: string;
  version: string;
  sha256: string;
}

const sha256 = (b: Buffer | string) => createHash('sha256').update(b).digest('hex');

/** The list as plugins/seed.json holds it. */
export function parseList(text: string): Entry[] {
  const list = JSON.parse(text) as { format?: number; plugins?: Entry[] };
  if (list.format !== 1 || !Array.isArray(list.plugins)) throw new Error('plugins/seed.json: format 1 and a list of plugins');
  for (const e of list.plugins)
    if (!e.id || !e.version || !/^[0-9a-f]{64}$/.test(e.sha256 ?? ''))
      throw new Error(`plugins/seed.json: ${e.id ?? 'an entry'} needs an id, a version and the SHA-256 of its version.json`);
  return list.plugins;
}

/** Whether version `a` comes before `b`, by their numbers. */
function before(a: string, b: string) {
  const n = (v: string) => v.split(/[-+]/)[0]!.split('.').map(Number);
  const [x, y] = [n(a), n(b)];
  for (let i = 0; i < 3; i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) < (y[i] ?? 0);
  return false;
}

export interface FetchOptions {
  entries: Entry[];
  to: string;
  store: string;
  mirror: string;
  keys: readonly PublicKey[];
  /** This Escouade's version: a version of the seed must run on it. */
  app: string;
}

async function get(url: string): Promise<Buffer | null> {
  const r = await fetch(url, { headers: { 'user-agent': 'escouade-seed' } });
  if (r.status === 404) return null;
  if (!r.ok) throw new Error(`${url}: answered ${r.status}`);
  return Buffer.from(await r.arrayBuffer());
}

async function need(url: string, what: string): Promise<Buffer> {
  const b = await get(url);
  if (!b) throw new Error(`${what}: not found at ${url}`);
  return b;
}

/** The revocation list of the day and its signature, from the store, else from its copy. */
async function list(store: string, mirror: string, keys: readonly PublicKey[]) {
  for (const base of [`${store}/api/v1`, mirror]) {
    const json = await get(`${base}/revocations.json`).catch(() => null);
    const sig = json && (await get(`${base}/revocations.json.minisig`).catch(() => null));
    if (!json || !sig) continue;
    const checked = checkRevocations(json.toString('utf8'), sig.toString('utf8'), keys);
    if ('error' in checked) throw new Error(`the revocation list of ${base}: ${checked.error}`);
    return { json, sig, list: checked.list as RevocationList };
  }
  throw new Error('no revocation list could be read: the seed would carry none');
}

/** Downloads the seed into `to`, emptied first (but its README), checking everything. Says what it wrote. */
export async function fetchSeed({ entries, to, store, mirror, keys, app }: FetchOptions): Promise<string[]> {
  await mkdir(to, { recursive: true });
  for (const name of await readdir(to)) if (name !== 'README.md') await rm(join(to, name), { recursive: true, force: true });
  // Nothing published yet (until Z2): the release goes on without an empty seed asking the store anything.
  if (!entries.length) return [];
  if (!keys.length) throw new Error('no public key: nothing can be checked');
  const catalog = JSON.parse((await need(`${store}/api/v1/catalog.json`, 'the catalogue')).toString('utf8')) as {
    plugins: { id: string; versions: { version: string; yanked?: boolean; files: Record<string, string> }[] }[];
  };
  const revocations = await list(store, mirror, keys);
  const said: string[] = [];
  for (const e of entries) {
    const name = `${e.id} v${e.version}`;
    const v = catalog.plugins.find((p) => p.id === e.id)?.versions.find((x) => x.version === e.version);
    if (!v) throw new Error(`${name} is not in the store's catalogue: the chef publishes first, the release comes after`);
    if (v.yanked) throw new Error(`${name} was taken back by its author`);
    for (const action of ['disable', 'unverify'] as const) {
      const hit = revoked(revocations.list, e.id, e.version, action);
      if (hit) throw new Error(`${name} is revoked (${action}): ${hit.reason.en}`);
    }
    const url = (path: string | undefined) => `${store}${path}`;
    const json = await need(url(v.files.version_json), `${name}: version.json`);
    if (sha256(json) !== e.sha256) throw new Error(`${name}: its version.json is not the one plugins/seed.json names`);
    if (!v.files.signature) throw new Error(`${name} is not signed`);
    const minisig = await need(url(v.files.signature), `${name}: version.json.minisig`);
    const sig = readSignature(minisig.toString('utf8'));
    if (!sig || !verifySigned(json, sig, keys)) throw new Error(`${name}: its signature does not hold with the store's keys`);
    if (sig.trustedComment.split(/\s+/).slice(0, 2).join(' ') !== `escouade-verified-v1 ${e.id}@${e.version}`)
      throw new Error(`${name}: signed for another version (${sig.trustedComment})`);
    const file = JSON.parse(json.toString('utf8')) as {
      id: string;
      version: string;
      archive: { sha256: string; size: number };
      wasm?: { sha256: string; size: number };
      escouade?: { min?: string; max?: string };
    };
    if (file.id !== e.id || file.version !== e.version) throw new Error(`${name}: its version.json is ${file.id}@${file.version}'s`);
    if (file.escouade?.min && before(app, file.escouade.min))
      throw new Error(`${name} asks for Escouade ${file.escouade.min}, this one is ${app}`);
    if (file.escouade?.max && before(file.escouade.max, app))
      throw new Error(`${name} stops at Escouade ${file.escouade.max}, this one is ${app}`);
    const archive = await need(url(v.files.archive), `${name}: archive.tar.gz`);
    if (sha256(archive) !== file.archive.sha256 || archive.length !== file.archive.size)
      throw new Error(`${name}: its archive is not the one version.json names`);
    const dir = join(to, e.id);
    await mkdir(dir, { recursive: true });
    await writeFile(join(dir, 'version.json'), json);
    await writeFile(join(dir, 'version.json.minisig'), minisig);
    await writeFile(join(dir, 'archive.tar.gz'), archive);
    if (file.wasm) {
      const module = await need(url(v.files.wasm), `${name}: plugin.wasm`);
      if (sha256(module) !== file.wasm.sha256 || module.length !== file.wasm.size)
        throw new Error(`${name}: its module is not the one version.json names`);
      await writeFile(join(dir, 'plugin.wasm'), module);
    }
    said.push(`${name}: ${archive.length} bytes, its signature holds`);
  }
  await writeFile(join(to, 'revocations.json'), revocations.json);
  await writeFile(join(to, 'revocations.json.minisig'), revocations.sig);
  said.push(`the revocation list no. ${revocations.list.serial}`);
  return said;
}

/** The files of a plugin's folder, by their path in it, its build's left out. */
async function folderFiles(dir: string, at = dir, out: Record<string, Buffer> = {}) {
  for (const entry of await readdir(at, { withFileTypes: true })) {
    if (['.git', 'target', 'node_modules'].includes(entry.name)) continue;
    const path = join(at, entry.name);
    if (entry.isDirectory()) await folderFiles(dir, path, out);
    else if (entry.isFile()) out[relative(dir, path).replaceAll('\\', '/')] = await readFile(path);
  }
  return out;
}

/** The seed of a build of development, from `folders` with their `plugin.wasm`: not signed, its own seed.json beside it. */
export async function devSeed(folders: string[], to: string): Promise<Entry[]> {
  await mkdir(to, { recursive: true });
  for (const name of await readdir(to)) if (name !== 'README.md') await rm(join(to, name), { recursive: true, force: true });
  const entries: Entry[] = [];
  for (const folder of folders) {
    const files = await folderFiles(folder);
    const text = files['escouade-plugin.json'];
    if (!text) throw new Error(`${folder}: no escouade-plugin.json`);
    const m = JSON.parse(text.toString('utf8')) as { id: string; version: string; escouade?: unknown };
    const module = files['plugin.wasm'];
    if (!module) throw new Error(`${folder}: no plugin.wasm, build it first`);
    const archive = tarball(files, { top: m.id, commit: '0'.repeat(40) }) as Buffer;
    const json = JSON.stringify({
      format: 1,
      id: m.id,
      version: m.version,
      archive: { sha256: sha256(archive), size: archive.length },
      manifest_sha256: sha256(text),
      escouade: m.escouade,
      wasm: { sha256: sha256(module), size: module.length },
    });
    const dir = join(to, m.id);
    await mkdir(dir, { recursive: true });
    await writeFile(join(dir, 'version.json'), json);
    await writeFile(join(dir, 'archive.tar.gz'), archive);
    await writeFile(join(dir, 'plugin.wasm'), module);
    entries.push({ id: m.id, version: m.version, sha256: sha256(json) });
  }
  await writeFile(join(to, 'seed.json'), `${JSON.stringify({ format: 1, plugins: entries }, null, 2)}\n`);
  return entries;
}

/** The public keys of the `.pub` files of `dir`. */
export async function keysOf(dir: string): Promise<PublicKey[]> {
  const names = (await readdir(dir)).filter((n) => n.endsWith('.pub'));
  const read = await Promise.all(names.map(async (n) => readPublicKey(await readFile(join(dir, n), 'utf8'))));
  return read.filter((k): k is PublicKey => !!k);
}

async function main(args: string[]) {
  const arg = (name: string, fallback: string) => {
    const i = args.indexOf(`--${name}`);
    return i >= 0 && args[i + 1] ? args[i + 1]! : fallback;
  };
  const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
  const to = arg('to', join(root, 'src-tauri', 'seed'));
  if (args[0] === 'dev') {
    const folders = args.slice(1).filter((a, i, all) => !a.startsWith('--') && all[i - 1] !== '--to');
    for (const e of await devSeed(folders, to)) console.log(`${e.id} v${e.version}, not signed`);
    return;
  }
  if (args[0] !== 'fetch') throw new Error('store/scripts/seed.ts fetch | dev <folder>…');
  const said = await fetchSeed({
    entries: parseList(await readFile(arg('list', join(root, 'plugins', 'seed.json')), 'utf8')),
    to,
    store: arg('store', 'https://store.escouade.dev').replace(/\/+$/, ''),
    mirror: arg('mirror', 'https://escouade.dev').replace(/\/+$/, ''),
    keys: await keysOf(arg('keys', join(root, 'store', 'keys'))),
    app: (JSON.parse(await readFile(join(root, 'package.json'), 'utf8')) as { version: string }).version,
  });
  for (const line of said.length ? said : ['plugins/seed.json names nothing yet: an empty seed']) console.log(line);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1])
  await main(process.argv.slice(2)).catch((e: Error) => {
    console.error(`::error::${e.message}`);
    process.exit(1);
  });
