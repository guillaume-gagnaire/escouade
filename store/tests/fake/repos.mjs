// The repositories of the fake GitHub (tests/fake/github.mjs), as the App reads them through an installation's
// token: their tags (v0.3.0 annotated, to unwrap), the archives of their commits (by a redirect, as codeload),
// the license GitHub detects, the releases, their files and their attestations. They hold the test plugins of
// tests/fake/plugins.mjs, by their name: `*-private` is private, `*-trap` is booby-trapped (a license that is not
// its file's, a symbolic link), `*-bin` runs a program built for each system, attested but for Linux; the others
// are valid.
import { randomBytes } from 'node:crypto';
import { attestation, pluginFiles, sha256, tarball } from './plugins.mjs';

const code = new Map(); // full_name -> { id, private, tags: { name: { sha, annotated } }, commits: { sha: { gz, license } }, releases: { tag: [asset] } }
const attestations = new Map(); // sha256 -> [attestation]
const appTokens = new Map(); // installation token -> installation id
let nextAsset = 1;

/** Pushes `tag` on `fullName` at a new commit of its plugin (`extra` files added); gives the commit. */
export function pushTag(fullName, tag, { extra = {}, manifest = {} } = {}) {
  const repo = code.get(fullName);
  const name = fullName.split('/')[1];
  const version = tag.replace(/^v/, '');
  const sha = sha256(`${fullName}@${tag}@${Date.now()}@${Math.random()}`).slice(0, 40);
  const trap = name.endsWith('-trap');
  const bin = name.endsWith('-bin');
  const files = pluginFiles(fullName, {
    manifest: {
      version,
      name: { en: name, fr: name },
      ...(bin ? { run: { command: 'bin/plugin', windows: 'bin/plugin.exe' } } : {}),
      ...manifest,
    },
    files: { ...(trap ? { 'docs/key': { symlink: '/home/me/.ssh/id_ed25519' } } : {}), ...extra },
  });
  repo.commits[sha] = {
    gz: tarball(files, { commit: sha, top: `${fullName.replace('/', '-')}-${sha.slice(0, 7)}` }),
    license: trap ? 'Apache-2.0' : 'MIT',
  };
  repo.tags[tag] = { sha, annotated: tag === 'v0.3.0' };
  if (bin)
    repo.releases[tag] = ['windows-x64', 'macos-arm64', 'linux-x64'].map((p) => {
      const bytes = Buffer.from(`${p} binary of ${fullName} ${tag}`);
      if (p !== 'linux-x64') attestations.set(sha256(bytes), [attestation(sha256(bytes), { repo: fullName, commit: sha })]);
      return { id: nextAsset++, name: `${name}-${version}-${p}.tar.gz`, bytes };
    });
  return sha;
}

/** A repository the App was installed on: two tags, v0.2.4 and v0.3.0. */
export function seedRepo(r) {
  if (code.has(r.full_name)) return;
  code.set(r.full_name, { id: r.id, private: r.private, tags: {}, commits: {}, releases: {} });
  pushTag(r.full_name, 'v0.2.4');
  pushTag(r.full_name, 'v0.3.0');
}

export const repoOf = (fullName) => code.get(fullName);

/** Answers what the App asks of the repositories; false if `path` is none of it. */
export function serveRepos(req, res, { path, q, port, installations, json, redirect }) {
  const access = path.match(/^\/app\/installations\/(\d+)\/access_tokens$/);
  if (access && req.method === 'POST') {
    // The App's JWT, as GitHub reads it: a token that is no JWT is refused.
    if (!(req.headers.authorization ?? '').startsWith('Bearer ey'))
      return (json(res, 401, { message: 'A JSON web token could not be decoded' }), true);
    if (!installations.has(Number(access[1]))) return (json(res, 404, { message: 'Not Found' }), true);
    const token = `ghs_${randomBytes(18).toString('hex')}`;
    appTokens.set(token, Number(access[1]));
    return (json(res, 201, { token, expires_at: new Date(Date.now() + 3600_000).toISOString() }), true);
  }
  const load = path.match(/^\/_codeload\/([^/]+)\/([0-9a-f]+)$/);
  if (load) {
    const c = code.get(decodeURIComponent(load[1]))?.commits[load[2]];
    if (!c) return (json(res, 404, { message: 'Not Found' }), true);
    res.writeHead(200, { 'content-type': 'application/x-gzip', 'content-length': c.gz.length });
    return (res.end(c.gz), true);
  }
  const r = path.match(/^\/repos\/([^/]+\/[^/]+?)(\/.*)?$/);
  if (!r) return false;
  const inst = installations.get(appTokens.get((req.headers.authorization ?? '').replace(/^Bearer /, '')));
  const fullName = decodeURIComponent(r[1]);
  const repo = code.get(fullName);
  if (!inst || !repo || !inst.repos.some((x) => x.full_name === fullName)) return (json(res, 404, { message: 'Not Found' }), true);
  const rest = r[2] ?? '';
  if (rest === '') return (json(res, 200, { id: repo.id, full_name: fullName, private: repo.private }), true);
  if (rest === '/tags')
    return (
      json(
        res,
        200,
        Object.entries(repo.tags).map(([name, t]) => ({ name, commit: { sha: t.sha } })),
      ),
      true
    );
  if (rest === '/releases')
    return (
      json(
        res,
        200,
        Object.keys(repo.releases).map((tag) => ({ tag_name: tag, draft: false, published_at: '2026-10-25T09:00:00Z' })),
      ),
      true
    );
  let m = rest.match(/^\/git\/ref\/tags\/(.+)$/);
  if (m) {
    const t = repo.tags[decodeURIComponent(m[1])];
    if (!t) return (json(res, 404, { message: 'Not Found' }), true);
    return (json(res, 200, { object: t.annotated ? { type: 'tag', sha: `tag-${t.sha}` } : { type: 'commit', sha: t.sha } }), true);
  }
  m = rest.match(/^\/git\/tags\/tag-([0-9a-f]+)$/);
  if (m) return (json(res, 200, { object: { type: 'commit', sha: m[1] }, tagger: { date: '2026-10-25T09:00:00Z' } }), true);
  m = rest.match(/^\/tarball\/([0-9a-f]+)$/);
  if (m && repo.commits[m[1]]) return (redirect(res, `/_codeload/${encodeURIComponent(fullName)}/${m[1]}`), true);
  if (rest === '/license') {
    const c = repo.commits[q.get('ref')];
    return (c ? json(res, 200, { license: { spdx_id: c.license } }) : json(res, 404, { message: 'Not Found' }), true);
  }
  m = rest.match(/^\/releases\/tags\/(.+)$/);
  if (m) {
    const assets = repo.releases[decodeURIComponent(m[1])];
    if (!assets) return (json(res, 404, { message: 'Not Found' }), true);
    const url = (a) => `http://127.0.0.1:${port}/repos/${fullName}/releases/assets/${a.id}`;
    return (json(res, 200, { assets: assets.map((a) => ({ name: a.name, size: a.bytes.length, url: url(a) })) }), true);
  }
  m = rest.match(/^\/releases\/assets\/(\d+)$/);
  if (m) {
    const a = Object.values(repo.releases)
      .flat()
      .find((x) => x.id === Number(m[1]));
    if (!a || req.headers.accept !== 'application/octet-stream') return (json(res, 404, { message: 'Not Found' }), true);
    res.writeHead(200, { 'content-type': 'application/octet-stream', 'content-length': a.bytes.length });
    return (res.end(a.bytes), true);
  }
  m = rest.match(/^\/attestations\/sha256:([0-9a-f]{64})$/);
  if (m)
    return (
      attestations.has(m[1]) ? json(res, 200, { attestations: attestations.get(m[1]) }) : json(res, 404, { message: 'Not Found' }),
      true
    );
  return (json(res, 404, { message: 'Not Found' }), true);
}
