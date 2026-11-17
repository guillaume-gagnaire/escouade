// A fake Open VSX for the e2e of the editor's extensions (ESC-481): its API (`/api/-/search`, `/api/<ns>/<name>[/<v>]`),
// the files of each version (VSIX, `.sha256`, `.sigzip`, icon, README) and its public key, each VSIX signed with an
// Ed25519 key of its own, as Open VSX signs them (the signature of the VSIX's bytes in `.signature.sig`). The app reaches it
// by `ESCOUADE_TEST_OPEN_VSX`, read by a build of development alone. It keeps what it was asked (`hits`).

import crypto from 'node:crypto';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import zlib from 'node:zlib';

/** An archive of `entries`, stored (no compression), as a zip reader takes it. */
export function zip(entries: [string, Buffer | string][]): Buffer {
  const locals: Buffer[] = [];
  const centrals: Buffer[] = [];
  let offset = 0;
  for (const [name, data] of entries) {
    const body = Buffer.isBuffer(data) ? data : Buffer.from(data);
    const fname = Buffer.from(name);
    const crc = zlib.crc32(body) >>> 0;
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0x0800, 6);
    local.writeUInt16LE(0, 8);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(body.length, 18);
    local.writeUInt32LE(body.length, 22);
    local.writeUInt16LE(fname.length, 26);
    locals.push(local, fname, body);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(0x0800, 8);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(body.length, 20);
    central.writeUInt32LE(body.length, 24);
    central.writeUInt16LE(fname.length, 28);
    central.writeUInt32LE(offset, 42);
    centrals.push(central, fname);
    offset += 30 + fname.length + body.length;
  }
  const dir = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(dir.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, dir, end]);
}

export interface FakeVersion {
  namespace: string;
  name: string;
  version: string;
  displayName: string;
  description: string;
  verified: boolean;
  vsix: Buffer;
}

const pkg = (ns: string, name: string, version: string, displayName: string, extra: object) =>
  JSON.stringify({ publisher: ns, name, version, displayName, engines: { vscode: '^1.80.0' }, ...extra }, null, 2);

/** A colour theme whose editor's background is `background`. */
export function theme(ns: string, name: string, version: string, label: string, background: string): FakeVersion {
  const vsix = zip([
    ['extension.vsixmanifest', '<PackageManifest/>'],
    [
      'extension/package.json',
      pkg(ns, name, version, label, { contributes: { themes: [{ label, uiTheme: 'vs-dark', path: './themes/theme.json' }] } }),
    ],
    [
      'extension/themes/theme.json',
      JSON.stringify({
        name: label,
        type: 'dark',
        colors: { 'editor.background': background, 'editorGutter.background': background },
        tokenColors: [{ scope: 'keyword', settings: { foreground: '#ff00aa' } }],
      }),
    ],
  ]);
  return { namespace: ns, name, version, displayName: label, description: `The theme ${label}`, verified: true, vsix };
}

/** A grammar of the language `acme` (files `*.acme`): `fonction` and `retour` are its keywords. */
export function grammar(ns: string, name: string, version: string): FakeVersion {
  const vsix = zip([
    [
      'extension/package.json',
      pkg(ns, name, version, 'Acme Language', {
        contributes: {
          languages: [{ id: 'acme', aliases: ['Acme'], extensions: ['.acme'], configuration: './language-configuration.json' }],
          grammars: [{ language: 'acme', scopeName: 'source.acme', path: './syntaxes/acme.tmLanguage.json' }],
          snippets: [{ language: 'acme', path: './snippets/acme.code-snippets' }],
        },
      }),
    ],
    ['extension/language-configuration.json', JSON.stringify({ comments: { lineComment: '#' }, brackets: [['(', ')']] })],
    [
      'extension/syntaxes/acme.tmLanguage.json',
      JSON.stringify({
        scopeName: 'source.acme',
        patterns: [
          { name: 'keyword.control.acme', match: '\\b(fonction|retour)\\b' },
          { name: 'comment.line.acme', match: '#.*$' },
        ],
      }),
    ],
    ['extension/snippets/acme.code-snippets', JSON.stringify({ fn: { prefix: 'fn', body: ['fonction $1()', '  retour $0'] } })],
  ]);
  return { namespace: ns, name, version, displayName: 'Acme Language', description: 'Acme files', verified: false, vsix };
}

/** An extension with code: for the web extension host (`browser`) or for Node (`main`). */
export function code(ns: string, name: string, version: string, browser: boolean): FakeVersion {
  const vsix = zip([
    [
      'extension/package.json',
      pkg(ns, name, version, browser ? 'Acme Web' : 'Acme Node', browser ? { browser: './web.js' } : { main: './main.js' }),
    ],
    ['extension/web.js', 'exports.activate = () => { globalThis.ranAcme = true; };'],
    ['extension/main.js', 'exports.activate = () => {};'],
  ]);
  return { namespace: ns, name, version, displayName: browser ? 'Acme Web' : 'Acme Node', description: 'With code', verified: true, vsix };
}

export interface FakeOpenVsx {
  url: string;
  hits: string[];
  versions: FakeVersion[];
  close: () => Promise<void>;
}

const cmp = (a: string, b: string) => {
  const pa = a.split('.').map(Number);
  const pb = b.split('.').map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) - (pb[i] ?? 0);
  return 0;
};

/** The fake, on `port` (0: one the system picks). */
export async function startFakeOpenVsx(port: number, versions: FakeVersion[]): Promise<FakeOpenVsx> {
  const { publicKey, privateKey } = crypto.generateKeyPairSync('ed25519');
  const pem = publicKey.export({ type: 'spki', format: 'pem' }).toString();
  const hits: string[] = [];
  let base = '';
  const card = (v: FakeVersion) => {
    const stem = `${v.namespace}.${v.name}-${v.version}`;
    const f = (x: string) => `${base}/files/${stem}${x}`;
    const all = versions.filter((o) => o.namespace === v.namespace && o.name === v.name);
    return {
      namespace: v.namespace,
      name: v.name,
      version: v.version,
      displayName: v.displayName,
      description: v.description,
      verified: v.verified,
      preRelease: false,
      downloadable: true,
      downloadCount: 4200,
      publishedBy: { loginName: v.namespace, fullName: `${v.namespace} Inc.` },
      allVersions: Object.fromEntries([['latest', ''], ...all.map((o) => [o.version, ''])]),
      files: {
        download: f('.vsix'),
        sha256: f('.sha256'),
        signature: f('.sigzip'),
        publicKey: `${base}/api/-/public-key/fake`,
        icon: f('/icon.png'),
        readme: f('/README.md'),
      },
    };
  };
  const latest = (ns: string, name: string) =>
    versions.filter((v) => v.namespace === ns && v.name === name).sort((a, b) => cmp(b.version, a.version))[0];
  // A PNG of one pixel.
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYPj/HwADAgH/3Uq0WwAAAABJRU5ErkJggg==', 'base64');
  const server = http.createServer((req, res) => {
    const url = new URL(req.url ?? '/', 'http://x');
    hits.push(url.pathname);
    const json = (v: unknown, status = 200) => res.writeHead(status, { 'Content-Type': 'application/json' }).end(JSON.stringify(v));
    if (url.pathname === '/api/-/public-key/fake') return res.writeHead(200, { 'Content-Type': 'text/plain' }).end(pem);
    if (url.pathname === '/api/-/search') {
      const q = (url.searchParams.get('query') ?? '').toLowerCase();
      const ids = [...new Set(versions.map((v) => `${v.namespace}/${v.name}`))].filter((id) =>
        id.replace('/', '.').toLowerCase().includes(q),
      );
      const list = ids.map((id) => card(latest(...(id.split('/') as [string, string]))));
      return json({ offset: 0, totalSize: list.length, extensions: list });
    }
    const api = /^\/api\/([^/]+)\/([^/]+)(?:\/([^/]+))?$/.exec(url.pathname);
    if (api) {
      const [, ns, name, version] = api;
      const v = version ? versions.find((o) => o.namespace === ns && o.name === name && o.version === version) : latest(ns, name);
      return v ? json(card(v)) : json({ error: 'not found' }, 404);
    }
    const file = /^\/files\/(.+)$/.exec(url.pathname)?.[1] ?? '';
    for (const v of versions) {
      const stem = `${v.namespace}.${v.name}-${v.version}`;
      if (file === `${stem}.vsix`) return res.writeHead(200, { 'Content-Type': 'application/octet-stream' }).end(v.vsix);
      if (file === `${stem}.sha256`) return res.writeHead(200).end(crypto.createHash('sha256').update(v.vsix).digest('hex'));
      if (file === `${stem}.sigzip`) {
        const sig = crypto.sign(null, v.vsix, privateKey);
        return res.writeHead(200).end(
          zip([
            ['.signature.sig', sig],
            ['.signature.manifest', '{}'],
            ['.signature.p7s', ''],
          ]),
        );
      }
      if (file === `${stem}/icon.png`) return res.writeHead(200, { 'Content-Type': 'image/png' }).end(png);
      if (file === `${stem}/README.md`)
        return res
          .writeHead(200, { 'Content-Type': 'text/markdown' })
          .end(`# ${v.displayName}\n\nUne extension d’essai.\n\n![capture](https://example.com/${v.name}.png)\n`);
    }
    res.writeHead(404).end('not found');
  });
  await new Promise<void>((resolve) => server.listen(port, '127.0.0.1', resolve));
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  return { url: base, hits, versions, close: () => new Promise((r) => server.close(() => r())) };
}
