// A fake store.escouade.dev for the app (STORE-9): the app's API as STORE-6 serves it (catalog.json with its ETag,
// version.json, archive.tar.gz, binaries, the signatures of STORE-8, revocations.json and its .minisig, POST installs),
// and the copy of the revocation list as escouade.dev serves it (under /mirror). Its key is made at the start, for this
// run alone (store/tests/fake/minisign.mjs): the app trusts it through ESCOUADE_STORE_TEST_KEY, which a debug build
// alone reads. Never the store's key, never a file of the repository.
//
// Run alone: `node tests/fixtures/store/fake-store.mjs` (FAKE_STORE_PORT, any free port by default) prints one line of
// JSON, { url, key }, then serves until killed. The Rust tests and the end-to-end tests start it so.
//
// What it holds (examples of PLG-10, plugin-examples/, under ids of their own):
// - dev.exemple.high-contrast: 1.0.0 verified, 1.1.0 not (an update that is not verified);
// - dev.exemple.team-rules: 1.0.0 verified, a program that decides permissions;
// - dev.exemple.slack: 1.0.0 not verified, a program and its binary for each system;
// - dev.exemple.later: 2.0.0 for an Escouade to come (min 99.0.0), 1.0.0 for this one (the last compatible is chosen);
// - dev.exemple.tampered: its archive changed after its version.json (refused);
// - dev.exemple.forged: said verified, its signature made for another version (refused).
// Its controls: POST /_fake/revocations { entries, serial? } signs and serves the next list (the mirror too);
// POST /_fake/down { down } makes /api/v1 answer 503; GET /_fake/state says the installs counted and the requests.
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
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

const PLATFORMS = ['windows-x64', 'windows-arm64', 'macos-x64', 'macos-arm64', 'linux-x64', 'linux-arm64'];

/** An example's files (plugin-examples/, as their repository would hold them), its manifest changed by `over`. */
function example(name, over = {}) {
  const out = exampleFiles(name);
  const m = { ...JSON.parse(String(out['escouade-plugin.json'])), ...over };
  out['escouade-plugin.json'] = JSON.stringify(m, null, 2);
  return out;
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
];

/** The store's content: the catalogue, the files by path, signed with `key`. */
export function build(key) {
  const files = new Map();
  const plugins = new Map();
  SPEC.forEach((s, n) => {
    const commit = sha256(String(n)).slice(0, 40);
    const over = { id: s.id, version: s.v, ...(s.name ? { name: s.name } : {}), ...(s.min ? { escouade: { api: '1', min: s.min } } : {}) };
    const f = example(s.from, over);
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
    const json = canonical({
      format: 1,
      id: s.id,
      version: s.v,
      repository: `https://github.com/escouade-dev/plugin-${s.from}`,
      tag: `v${s.v}`,
      commit,
      archive: { sha256: sha256(archive), size: archive.length },
      binaries,
      manifest_sha256: sha256(f['escouade-plugin.json']),
      escouade: m.escouade,
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
        author: 'escouade-dev',
        repository: `https://github.com/escouade-dev/plugin-${s.from}`,
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
