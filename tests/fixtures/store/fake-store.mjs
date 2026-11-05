// A fake store.escouade.dev for the app (STORE-9): the app's API as STORE-6 serves it (catalog.json with its ETag,
// version.json, archive.tar.gz, binaries, the signatures of STORE-8, revocations.json and its .minisig, POST installs),
// and the copy of the revocation list as escouade.dev serves it (under /mirror). Its key is made at the start, for this
// run alone (store/tests/fake/minisign.mjs): the app trusts it through ESCOUADE_STORE_TEST_KEY, which a debug build
// alone reads. Never the store's key, never a file of the repository.
//
// Run alone: `node tests/fixtures/store/fake-store.mjs` (FAKE_STORE_PORT, any free port by default) prints one line of
// JSON, { url, key }, then serves until killed. The Rust tests and the end-to-end tests start it so.
//
// What it holds (the files of Escouade's own plugins, plugins/<id>/; under a third party's id but one, ESC-314):
// - dev.exemple.high-contrast: 1.0.0 verified, 1.1.0 not (an update that is not verified);
// - dev.exemple.team-rules: 1.0.0 verified, a program that decides permissions;
// - dev.exemple.slack: 1.0.0 not verified, a program and its binary for each system;
// - dev.exemple.later: 2.0.0 for an Escouade to come (min 99.0.0), 1.0.0 for this one (the last compatible is chosen);
// - dev.exemple.tampered: its archive changed after its version.json (refused);
// - dev.exemple.forged: said verified, its signature made for another version (refused);
// - dev.exemple.in-folder: from the folder plugins/discord of a repository with several (ESC-313);
// - dev.escouade.discord: 1.1.0 verified, from plugins/discord of the app's repository (Escouade's own hand, ESC-314),
//   installed as any plugin of the store (origin store, never the app's builtin);
// - dev.escouade.slack: 1.1.0 not signed yet (the app refuses it: Escouade's own installs signed only);
// - dev.exemple.climbs: its version.json names a folder that climbs out (refused);
// - dev.exemple.reactor: a run.wasm plugin (tests/fixtures/plugins/reactor), its archive with a plugin.wasm that is not the
//   one the store compiled (ESC-331); dev.exemple.reactor-bare: the same without a module of the store (refused);
// - dev.exemple.wasm: a run.js plugin (tests/fixtures/plugins/sdk-both), with the module the store compiled of it
//   (ESC-323), named by its version.json; dev.exemple.wasm-changed: its module changed after its version.json (refused);
// - dev.escouade.trello: 2.2.0 and 2.3.0 verified, a run.wasm plugin of the first hand, for the app's seed (ESC-338);
//   dev.escouade.seed-later: verified, for an Escouade to come (min 99.0.0); dev.escouade.seed-tampered: verified, its
//   archive changed after its version.json.
// Its controls: POST /_fake/revocations { entries, serial? } signs and serves the next list (the mirror too);
// POST /_fake/down { down } makes /api/v1 answer 503; GET /_fake/state says the installs counted and the requests;
// POST /_fake/seed { dir, plugins: [{ id, version }] } writes in `dir` the seed of those versions as the release makes it
// (the bytes and the signatures of the store, the list of the day), with its seed.json.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tarball, png } from '../../../store/tests/fake/plugins.mjs';
import { exampleFiles } from '../../../store/tests/fake/review-store.mjs';
import { testKey } from '../../../store/tests/fake/minisign.mjs';

const sha256 = (b) => createHash('sha256').update(b).digest('hex');
/** version.json as the store writes it: keys sorted, no space (store/server/utils/versions.ts). */
const canonical = (v) =>
  Array.isArray(v)
    ? `[${v.map(canonical).join(',')}]`
    : v !== null && typeof v === 'object'
      ? `{${Object.keys(v)
          .filter((k) => v[k] !== undefined)
          .sort()
          .map((k) => `${JSON.stringify(k)}:${canonical(v[k])}`)
          .join(',')}}`
      : JSON.stringify(v);

/** The repository of a version: its own, else the third party's (one per plugin, or one with several in folders). */
const repoOf = (s) => s.repo ?? (s.path ? 'escouade-dev/plugins' : `escouade-dev/plugin-${s.from}`);

const PLATFORMS = ['windows-x64', 'windows-arm64', 'macos-x64', 'macos-arm64', 'linux-x64', 'linux-arm64'];

/** A plugin's files (plugins/<name>/, as a repository would hold them), its manifest changed by `over`. */
function example(name, over = {}) {
  const out = exampleFiles(name);
  const m = { ...JSON.parse(String(out['escouade-plugin.json'])), ...over };
  out['escouade-plugin.json'] = JSON.stringify(m, null, 2);
  return out;
}

/** The run.js plugin of the SDK's tests, its manifest changed by `over`. */
function runJs(over) {
  const root = new URL('../plugins/sdk-both/', import.meta.url);
  const m = { ...JSON.parse(readFileSync(new URL('escouade-plugin.json', root), 'utf8')), ...over };
  return {
    'escouade-plugin.json': JSON.stringify(m, null, 2),
    'dist/plugin.mjs': readFileSync(new URL('dist/plugin.mjs', root)),
    'README.md': '# SDK\n\nUn plugin run.js.\n',
  };
}

/** The run.wasm plugin of the host's tests, its manifest changed by `over`: the store compiled its module. */
function runWasm(over) {
  const root = new URL('../plugins/reactor/', import.meta.url);
  const m = { ...JSON.parse(readFileSync(new URL('escouade-plugin.json', root), 'utf8')), ...over };
  return {
    'escouade-plugin.json': JSON.stringify(m, null, 2),
    'Cargo.toml': readFileSync(new URL('Cargo.toml', root)),
    'src/lib.rs': readFileSync(new URL('src/lib.rs', root)),
    'plugin.wasm': 'not the module the store compiled',
    'README.md': '# Reactor\n\nUn plugin run.wasm.\n',
  };
}

/** A WASM module the app takes for valid: its header and a custom section that names it. */
function wasmModule(name) {
  const title = Buffer.from('escouade-test');
  const payload = Buffer.from(name);
  const body = Buffer.concat([Buffer.from([title.length]), title, payload]);
  return Buffer.concat([Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 0, body.length]), body]);
}

const SPEC = [
  {
    id: 'dev.exemple.high-contrast',
    v: '1.0.0',
    from: 'high-contrast',
    verified: true,
    cat: ['themes'],
    kw: ['contraste', 'thème'],
    installs: 2310,
    icon: true,
  },
  { id: 'dev.exemple.high-contrast', v: '1.1.0', from: 'high-contrast', cat: ['themes'], kw: ['contraste', 'thème'] },
  {
    id: 'dev.exemple.team-rules',
    v: '1.0.0',
    from: 'team-rules',
    verified: true,
    cat: ['permissions'],
    kw: ['règles', 'autorisations'],
    installs: 214,
  },
  {
    id: 'dev.exemple.slack',
    v: '1.0.0',
    from: 'slack',
    cat: ['notifications'],
    kw: ['slack', 'notifications'],
    installs: 1284,
    binaries: true,
  },
  { id: 'dev.exemple.later', v: '1.0.0', from: 'high-contrast', cat: ['themes'], kw: [], installs: 12, name: 'Plus tard' },
  { id: 'dev.exemple.later', v: '2.0.0', from: 'high-contrast', cat: ['themes'], kw: [], name: 'Plus tard', min: '99.0.0' },
  { id: 'dev.exemple.tampered', v: '1.0.0', from: 'high-contrast', cat: ['themes'], kw: [], name: 'Piégé', tampered: true },
  { id: 'dev.exemple.forged', v: '1.0.0', from: 'high-contrast', cat: ['themes'], kw: [], name: 'Faux sceau', forged: true },
  // ESC-313: a plugin in a folder of a repository with several, and a version.json whose folder climbs out.
  {
    id: 'dev.exemple.in-folder',
    v: '1.0.0',
    from: 'discord',
    cat: ['notifications'],
    kw: [],
    name: 'En dossier',
    path: 'plugins/discord',
  },
  // ESC-314: Escouade's own hand, from plugins/<id>/ of the app's repository; signed, then not signed yet.
  {
    id: 'dev.escouade.discord',
    v: '1.1.0',
    from: 'discord',
    verified: true,
    cat: ['notifications'],
    kw: ['discord', 'notifications'],
    installs: 87,
    path: 'plugins/discord',
    repo: 'guillaume-gagnaire/escouade',
  },
  {
    id: 'dev.escouade.slack',
    v: '1.1.0',
    from: 'slack',
    cat: ['notifications'],
    kw: ['slack', 'notifications'],
    path: 'plugins/slack',
    repo: 'guillaume-gagnaire/escouade',
  },
  { id: 'dev.exemple.climbs', v: '1.0.0', from: 'high-contrast', cat: ['themes'], kw: [], name: 'Dossier qui remonte', path: '../escape' },
  // ESC-323: a run.js plugin and the module the store compiled of it; then one whose module changed after its version.json.
  { id: 'dev.exemple.wasm', v: '1.0.0', from: 'sdk-both', runJs: true, cat: ['tools'], kw: ['wasm'], name: 'En WASM' },
  {
    id: 'dev.exemple.wasm-changed',
    v: '1.0.0',
    from: 'sdk-both',
    runJs: true,
    cat: ['tools'],
    kw: [],
    name: 'WASM changé',
    wasmChanged: true,
  },
  // ESC-331: a run.wasm plugin and the module the store compiled of it; then one that came without any.
  { id: 'dev.exemple.reactor', v: '1.0.0', from: 'reactor', runWasm: true, cat: ['tools'], kw: ['wasm'], name: 'En Rust' },
  { id: 'dev.exemple.reactor-bare', v: '1.0.0', from: 'reactor', runWasm: true, bare: true, cat: ['tools'], kw: [], name: 'Sans module' },
  // ESC-338: what the app's seed holds, a plugin of the first hand in run.wasm, and one for an Escouade to come.
  ...['2.2.0', '2.3.0'].map((v) => ({
    id: 'dev.escouade.trello',
    v,
    from: 'reactor',
    runWasm: true,
    verified: true,
    cat: ['kanban'],
    kw: ['trello'],
    name: 'Trello',
    path: 'plugins/trello',
    repo: 'guillaume-gagnaire/escouade',
  })),
  {
    id: 'dev.escouade.seed-later',
    v: '1.0.0',
    from: 'reactor',
    runWasm: true,
    verified: true,
    cat: ['tools'],
    kw: [],
    name: 'Une version à venir',
    min: '99.0.0',
    path: 'plugins/seed-later',
    repo: 'guillaume-gagnaire/escouade',
  },
  {
    id: 'dev.escouade.seed-tampered',
    v: '1.0.0',
    from: 'reactor',
    runWasm: true,
    verified: true,
    tampered: true,
    cat: ['tools'],
    kw: [],
    name: 'Amorce changée',
    path: 'plugins/seed-tampered',
    repo: 'guillaume-gagnaire/escouade',
  },
];

/** The store's content: the catalogue, the files by path, signed with `key`. */
export function build(key) {
  const files = new Map();
  const plugins = new Map();
  SPEC.forEach((s, n) => {
    const commit = sha256(String(n)).slice(0, 40);
    // A third party's copy has no repository of the app's; Escouade's own keeps its manifest's.
    const over = {
      id: s.id,
      version: s.v,
      ...(s.repo ? {} : { repository: undefined }),
      ...(s.name ? { name: s.name } : {}),
      ...(s.min ? { escouade: { api: '1', min: s.min } } : {}),
    };
    const f = s.runJs ? runJs(over) : s.runWasm ? runWasm(over) : example(s.from, over);
    if (s.icon) {
      f['icon.png'] = png(256, 256);
      const m = JSON.parse(f['escouade-plugin.json']);
      m.icon = 'icon.png';
      f['escouade-plugin.json'] = JSON.stringify(m, null, 2);
    }
    const m = JSON.parse(f['escouade-plugin.json']);
    const archive = tarball(f, { top: `escouade-dev-${s.from}-${commit.slice(0, 7)}`, commit });
    const base = `/api/v1/plugins/${s.id}/${s.v}`;
    const binaries = {};
    const binaryPaths = {};
    if (s.binaries) {
      for (const p of PLATFORMS) {
        const bytes = Buffer.from(p.startsWith('windows') ? `@echo off\r\nrem ${s.id} ${p}\r\n` : `#!/bin/sh\n# ${s.id} ${p}\n`);
        binaries[p] = { name: `${s.id}-${s.v}-${p}`, sha256: sha256(bytes), size: bytes.length, attested: true };
        binaryPaths[p] = `${base}/binaries/${p}`;
        files.set(binaryPaths[p], bytes);
      }
    }
    let wasm;
    let wasmPath;
    if (s.runJs) {
      const bytes = wasmModule(s.id);
      wasm = { sha256: sha256(bytes), size: bytes.length, compiler: { javy: '9.1.0', engine: 'e'.repeat(64), prelude: '2.2.0' } };
      wasmPath = `${base}/plugin.wasm`;
      files.set(wasmPath, s.wasmChanged ? Buffer.concat([bytes, wasmModule('changed').subarray(8)]) : bytes);
    }
    if (s.runWasm && !s.bare) {
      const bytes = readFileSync(new URL('../plugins/reactor/plugin.wasm', import.meta.url));
      wasm = {
        sha256: sha256(bytes),
        size: bytes.length,
        compiler: { rust: '1.98.0', target: 'wasm32-wasip1', canon: 'c'.repeat(64), lock: 'd'.repeat(64) },
      };
      wasmPath = `${base}/plugin.wasm`;
      files.set(wasmPath, bytes);
    }
    const json = canonical({
      format: 1,
      id: s.id,
      version: s.v,
      repository: `https://github.com/${repoOf(s)}`,
      ...(s.path ? { path: s.path } : {}),
      tag: s.path ? `${s.path}/v${s.v}` : `v${s.v}`,
      commit,
      archive: { sha256: sha256(archive), size: archive.length },
      binaries,
      manifest_sha256: sha256(f['escouade-plugin.json']),
      escouade: m.escouade,
      wasm,
    });
    files.set(`${base}/version.json`, Buffer.from(json));
    files.set(`${base}/archive.tar.gz`, s.tampered ? tarball({ ...f, 'extra.js': 'evil()' }, { commit }) : archive);
    const signed = s.verified || s.forged;
    if (signed) {
      const target = s.forged ? `${s.id}@9.9.9` : `${s.id}@${s.v}`;
      files.set(`${base}/version.json.minisig`, Buffer.from(key.sign(json, `escouade-verified-v1 ${target} 2026-10-29`)));
    }
    if (!plugins.has(s.id))
      plugins.set(s.id, {
        id: s.id,
        name: m.name,
        description: m.description ?? '',
        author: repoOf(s).split('/')[0],
        repository: `https://github.com/${repoOf(s)}`,
        license: 'MIT',
        categories: s.cat,
        keywords: s.kw,
        installs: s.installs ?? 0,
        latest: null,
        versions: [],
      });
    const p = plugins.get(s.id);
    p.latest = s.v;
    p.versions.unshift({
      version: s.v,
      published_at: `2026-10-${String(20 + n).padStart(2, '0')}T08:00:00.000Z`,
      sha256: sha256(json),
      archive: { sha256: sha256(archive), size: archive.length },
      platforms: Object.keys(binaries),
      escouade: m.escouade,
      verified: !!signed,
      ...(signed
        ? {
            review: {
              signed_at: '2026-10-29T10:00:00.000Z',
              summary: { fr: 'Relu : réseau déclaré seulement.', en: 'Reviewed: declared network only.' },
            },
          }
        : {}),
      yanked: false,
      permissions: m.permissions ?? {},
      run: !!m.run,
      contributes: Object.keys(m.contributes ?? {}).sort(),
      files: {
        version_json: `${base}/version.json`,
        archive: `${base}/archive.tar.gz`,
        binaries: binaryPaths,
        ...(wasmPath ? { wasm: wasmPath } : {}),
        ...(signed ? { signature: `${base}/version.json.minisig` } : {}),
      },
    });
  });
  const catalog = {
    format: 1,
    updated_at: '2026-10-29T08:00:00.000Z',
    installs_until: '2026-10-29',
    revocations: null,
    plugins: [...plugins.values()].sort((a, b) => a.id.localeCompare(b.id)),
  };
  return { catalog, files };
}

/** Starts the fake store on `port` (0: any). */
export function startFakeStore(port = 0) {
  const key = testKey();
  const { catalog, files } = build(key);
  const body = Buffer.from(JSON.stringify(catalog));
  const etag = `"${sha256(body).slice(0, 32)}"`;
  const state = { installs: [], requests: [], down: false, list: null, serial: 0 };
  const signList = (entries, serial) => {
    const json = JSON.stringify({ serial, issued_at: new Date().toISOString(), entries });
    return { json, minisig: key.sign(json, `escouade-revocations-v1 ${serial} 2026-10-30`) };
  };
  const server = createServer(async (req, res) => {
    const url = new URL(req.url, 'http://x');
    const path = decodeURIComponent(url.pathname);
    const chunks = [];
    for await (const c of req) chunks.push(c);
    const json = () => JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}');
    state.requests.push(`${req.method} ${path}`);
    const send = (status, data, type = 'application/json', headers = {}) => {
      res.writeHead(status, { 'content-type': type, ...headers });
      res.end(data);
    };
    if (path === '/_fake/state')
      return send(200, JSON.stringify({ installs: state.installs, requests: state.requests, serial: state.serial }));
    if (path === '/_fake/down' && req.method === 'POST') {
      state.down = !!json().down;
      return send(200, '{}');
    }
    if (path === '/_fake/revocations' && req.method === 'POST') {
      const b = json();
      const serial = b.serial ?? state.serial + 1;
      state.serial = Math.max(state.serial, serial);
      state.list = signList(b.entries ?? [], serial);
      return send(200, JSON.stringify({ serial }));
    }
    if (path === '/_fake/seed' && req.method === 'POST') {
      const b = json();
      const plugins = b.plugins.map(({ id, version }) => {
        const base = `/api/v1/plugins/${id}/${version}`;
        const dir = join(b.dir, id);
        mkdirSync(dir, { recursive: true });
        for (const name of ['version.json', 'version.json.minisig', 'archive.tar.gz', 'plugin.wasm'])
          if (files.has(`${base}/${name}`)) writeFileSync(join(dir, name), files.get(`${base}/${name}`));
        return { id, version, sha256: sha256(files.get(`${base}/version.json`)) };
      });
      writeFileSync(join(b.dir, 'seed.json'), JSON.stringify({ format: 1, plugins }));
      if (state.list) {
        writeFileSync(join(b.dir, 'revocations.json'), state.list.json);
        writeFileSync(join(b.dir, 'revocations.json.minisig'), state.list.minisig);
      }
      return send(200, JSON.stringify({ plugins }));
    }
    if (path === '/mirror/revocations.json' || path === '/mirror/revocations.json.minisig') {
      if (!state.list) return send(404, '{}');
      return send(200, path.endsWith('.minisig') ? state.list.minisig : state.list.json, 'text/plain');
    }
    if (!path.startsWith('/api/v1/')) return send(404, '{"message":"Not Found"}');
    if (state.down) return send(503, '{"statusMessage":"down"}');
    if (path === '/api/v1/catalog.json') {
      if (req.headers['if-none-match'] === etag) return send(304, '', 'application/json', { etag });
      return send(200, body, 'application/json', { etag });
    }
    if (path === '/api/v1/revocations.json' || path === '/api/v1/revocations.json.minisig') {
      if (!state.list) return send(404, '{"statusMessage":"No revocation list yet"}');
      return send(200, path.endsWith('.minisig') ? state.list.minisig : state.list.json, 'text/plain');
    }
    if (path === '/api/v1/installs' && req.method === 'POST') {
      state.installs.push(json());
      return send(200, '{"ok":true}');
    }
    if (files.has(path)) return send(200, files.get(path), 'application/octet-stream');
    send(404, '{"statusMessage":"Not Found"}');
  });
  return new Promise((done) =>
    server.listen(port, '127.0.0.1', () => {
      const url = `http://127.0.0.1:${server.address().port}`;
      const pub = key.pub.split('\n')[1];
      done({ url, key: pub, state, close: () => new Promise((r) => server.close(r)) });
    }),
  );
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const s = await startFakeStore(Number(process.env.FAKE_STORE_PORT ?? 0));
  process.stdout.write(`${JSON.stringify({ url: s.url, key: s.key })}\n`);
}
