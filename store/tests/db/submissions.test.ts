import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import pg from 'pg';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { applyWebhook, signIn, tagOfWebhook, type Account } from '../../server/utils/accounts';
import { migrateDb, openDb, type Db } from '../../server/utils/db';
import { GithubError, type GithubInstallation } from '../../server/utils/github';
import { imageInfo } from '../../server/utils/images';
import { TooBig, type Asset, type Source } from '../../server/utils/source';
import {
  checkVersion,
  onTag,
  publishVersion,
  rereadTags,
  retrySubmission,
  startSubmission,
  SUBMISSIONS_PER_HOUR,
  versionView,
  type Deps,
} from '../../server/utils/submissions';
import { sha256 } from '../../server/utils/versions';
import { authorPage, catalog, pluginPage, search } from '../../server/utils/listing';
import { buildCatalog } from '../../server/utils/catalog';
import { authorPlugins } from '../../server/utils/author';
import { fileReport } from '../../server/utils/reports';
import { TRAPPED } from '../fake/catalog.mjs';
import { attestation, pluginFiles, png, tarball } from '../fake/plugins.mjs';

// The submission against a real PostgreSQL (TEST_DATABASE_URL alone, emptied first; skipped without it), with a
// fake GitHub in memory: a tag resolved once, the archive kept as it came and fingerprinted, the ten checks, the
// rejections, publishing, the new tags of the webhooks and the tags moved (docs/STORE.md §3).
const url = process.env.TEST_DATABASE_URL;
const SECRET = 's'.repeat(64);
const T0 = new Date('2026-10-29T10:00:00Z');

interface FakeRepo {
  id: number;
  private: boolean;
  tags: Record<string, string>;
  archives: Record<string, Buffer>;
  licenses: Record<string, string | null>;
  releases: Record<string, (Asset & { bytes: Buffer })[]>;
}

/** A GitHub in memory: repositories, their tags, archives, licenses, releases and attestations. */
function fakeGithub() {
  const repos = new Map<string, FakeRepo>();
  const attestations = new Map<string, unknown[]>();
  const calls: string[] = [];
  let failing = false;
  const repo = (name: string) => {
    const r = repos.get(name);
    if (!r) throw new GithubError('404', 404);
    return r;
  };
  const source: Source = {
    async repo(name) {
      return { id: repo(name).id, fullName: name, private: repo(name).private };
    },
    async tags(name) {
      return Object.entries(repo(name).tags).map(([t, commit]) => ({ name: t, commit, release: !!repo(name).releases[t], date: null }));
    },
    async resolve(name, tag) {
      const commit = repo(name).tags[tag];
      if (!commit) throw new GithubError('404', 404);
      return { commit, date: '2026-10-25T09:00:00Z' };
    },
    async tarball(name, commit, max) {
      calls.push(`tarball ${name}@${commit}`);
      if (failing) throw new GithubError('GitHub answered 502 to the archive', 502);
      const b = repo(name).archives[commit]!;
      if (b.length > max) throw new TooBig(b.length);
      return b;
    },
    async license(name, commit) {
      return repo(name).licenses[commit] ?? null;
    },
    async release(name, tag) {
      return repo(name).releases[tag] ?? null;
    },
    async asset(asset, max) {
      if (asset.size > max) throw new TooBig(asset.size);
      for (const r of repos.values())
        for (const list of Object.values(r.releases)) for (const a of list) if (a.url === asset.url) return a.bytes;
      throw new GithubError('404', 404);
    },
    async attestations(_name, digest) {
      return attestations.get(digest) ?? [];
    },
  };
  /** Pushes `tag` on `name` at a new commit made of `files`. */
  const push = (name: string, tag: string, files: Record<string, unknown>, license = 'MIT') => {
    const r = repo(name);
    const commit = sha256(`${name}${tag}${JSON.stringify(Object.keys(files))}${Math.random()}`).slice(0, 40);
    r.tags[tag] = commit;
    r.archives[commit] = tarball(files as Parameters<typeof tarball>[0], {
      commit,
      top: `${name.replace('/', '-')}-${commit.slice(0, 7)}`,
    });
    r.licenses[commit] = license;
    return commit;
  };
  const add = (name: string, id: number, isPrivate = false) =>
    repos.set(name, { id, private: isPrivate, tags: {}, archives: {}, licenses: {}, releases: {} });
  return { source, repos, attestations, calls, push, add, fail: (on: boolean) => (failing = on) };
}

describe.skipIf(!url)('the submission', () => {
  let pool: pg.Pool;
  let db: Db;
  let dir: string;
  let gh: ReturnType<typeof fakeGithub>;
  let deps: Deps;
  let sam: Account;
  const q = async (sql: string, params: unknown[] = []) => (await pool.query(sql, params)).rows;
  const REPO = 'sroux/escouade-ntfy';

  const signInAs = async (login: string, id: number, repos: { id: number; full_name: string; private: boolean }[]) => {
    const installation: GithubInstallation = { id: id * 10, account: { id, login, type: 'User' }, repository_selection: 'selected' };
    const { account } = await signIn(
      db,
      {
        user: { id, login, name: login, avatar_url: null },
        token: { token: `ghu_${login}`, expiresAt: null },
        installations: [{ installation, repos }],
      },
      SECRET,
      T0,
    );
    return account;
  };

  /** Submits `tag` of `repo` as `who`, then works its job; gives the version's id and where it stands. */
  const submit = async (who: Account, repo: string, tag: string) => {
    const started = await startSubmission(db, deps, who, { repo, tag }, T0);
    if ('error' in started) return { error: started.error };
    return { id: started.id, state: await checkVersion(db, deps, started.id) };
  };

  beforeAll(async () => {
    const admin = new pg.Client({ connectionString: url });
    await admin.connect();
    await admin.query(
      'drop schema if exists drizzle cascade; drop schema if exists pgboss cascade; drop schema public cascade; create schema public;',
    );
    await admin.end();
    ({ pool, db } = openDb(url!));
    await migrateDb(db, './server/db/migrations');
    dir = await mkdtemp(join(tmpdir(), 'store-archives-'));
  });
  beforeEach(async () => {
    await pool.query(
      'truncate accounts, sessions, installations, installation_repos, account_installations, audit, plugins, versions, reports, installs_daily, revocation_lists cascade',
    );
    gh = fakeGithub();
    gh.add(REPO, 77);
    gh.add('sroux/secret', 78, true);
    deps = { source: () => gh.source, archives: dir, env: {}, site: 'https://store.test' };
    sam = await signInAs('sroux', 4242, [
      { id: 77, full_name: REPO, private: false },
      { id: 78, full_name: 'sroux/secret', private: true },
    ]);
  });
  afterAll(async () => {
    await pool?.end();
    if (dir) await rm(dir, { recursive: true, force: true });
  });

  it('freezes a version: the tag resolved once, the archive kept as it came, its version.json and fingerprint', async () => {
    const commit = gh.push(REPO, 'v0.3.0', pluginFiles(REPO));
    const { id, state } = await submit(sam, REPO, 'v0.3.0');
    expect(state).toBe('ready');
    const [v] = await q('select * from versions where id = $1', [id]);
    expect(v).toMatchObject({
      commit,
      tag: 'v0.3.0',
      version: '0.3.0',
      plugin_id: 'io.github.sroux.escouade-ntfy',
      state: 'ready',
      origin: 'author',
    });
    expect(v.checks.map((c: { status: string }) => c.status)).toEqual(Array(10).fill('pass'));

    // The archive on disk is GitHub's, byte for byte, under its SHA-256.
    const archive = gh.repos.get(REPO)!.archives[commit]!;
    expect(v.archive_sha256).toBe(sha256(archive));
    const kept = await readFile(join(dir, 'sha256', v.archive_sha256.slice(0, 2), `${v.archive_sha256}.tar.gz`));
    expect(kept.equals(archive)).toBe(true);

    // version.json: canonical, its keys sorted, its fingerprint its own SHA-256.
    const file = JSON.parse(v.version_json);
    expect(file).toEqual({
      archive: { sha256: v.archive_sha256, size: archive.length },
      binaries: {},
      commit,
      escouade: { api: '1', min: '2.1.0' },
      format: 1,
      id: 'io.github.sroux.escouade-ntfy',
      manifest_sha256: v.manifest_sha256,
      repository: `https://github.com/${REPO}`,
      tag: 'v0.3.0',
      version: '0.3.0',
    });
    expect(v.version_sha256).toBe(sha256(v.version_json));

    // The screenshot, re-encoded without its metadata.
    expect(v.screenshots).toHaveLength(1);
    const shot = await readFile(join(dir, 'sha256', v.screenshots[0].sha256.slice(0, 2), `${v.screenshots[0].sha256}.png`));
    expect(imageInfo(shot)).toEqual({ format: 'png', width: 320, height: 200 });
    expect(shot.includes(Buffer.from('Somebody at'))).toBe(false);

    // The same tag again gives the same version back, without downloading again.
    const again = await startSubmission(db, deps, sam, { repo: REPO, tag: 'v0.3.0' }, T0);
    expect(again).toEqual({ id, again: true });
    expect(gh.calls.filter((c) => c.startsWith('tarball'))).toHaveLength(1);
  });

  it('keeps the icon re-encoded at 256 and 64 px without its metadata, in version.json, the catalogue and the pages (PLG-13)', async () => {
    const icon = png(512, 512, { text: 'Comment\0drawn at 48.85,2.35' });
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO, { manifest: { icon: 'store/icon.png' }, files: { 'store/icon.png': icon } }));
    const { id, state } = await submit(sam, REPO, 'v0.3.0');
    expect(state).toBe('ready');
    const [v] = await q('select icon, version_json from versions where id = $1', [id]);
    expect(Object.keys(v.icon).sort()).toEqual(['256', '64']);
    for (const [side, hash] of Object.entries(v.icon as Record<string, string>)) {
      const kept = await readFile(join(dir, 'sha256', hash.slice(0, 2), `${hash}.png`));
      expect(sha256(kept)).toBe(hash);
      expect(imageInfo(kept)).toEqual({ format: 'png', width: Number(side), height: Number(side) });
      expect(kept.includes(Buffer.from('drawn at'))).toBe(false);
    }
    expect(JSON.parse(v.version_json).icon).toEqual(v.icon);

    await publishVersion(db, sam, id!, { categories: ['notifications'], review: 'on' });
    const c = await buildCatalog(db);
    const entry = c.plugins[0]!.versions[0]!;
    expect(entry.icon).toEqual(v.icon);
    expect(entry.files.icon).toEqual({ '256': `/files/${v.icon['256']}.png`, '64': `/files/${v.icon['64']}.png` });
    // The tiles of every page: home and search, the plugin's page, its author's.
    expect((await catalog(db)).find((e) => e.id === 'io.github.sroux.escouade-ntfy')!.icon).toEqual(v.icon);
    expect((await pluginPage(db, dir, 'io.github.sroux.escouade-ntfy'))!.entry.icon).toEqual(v.icon);
    expect((await authorPage(db, 'sroux'))!.plugins[0]!.icon).toEqual(v.icon);
    expect((await authorPlugins(db, sam))[0]!.icon).toEqual(v.icon);

    // Without an icon: none anywhere, the letters stay; version.json says nothing of it.
    gh.push(REPO, 'v0.4.0', pluginFiles(REPO, { manifest: { version: '0.4.0' } }));
    const plain = await submit(sam, REPO, 'v0.4.0');
    const [w] = await q('select icon, version_json from versions where id = $1', [plain.id]);
    expect(w.icon).toBeNull();
    expect('icon' in JSON.parse(w.version_json)).toBe(false);
  });

  it('refuses a version whose icon does not hold, with the reason, and keeps none of its files (PLG-13)', async () => {
    const tries: [string, Record<string, unknown>, Record<string, unknown>, RegExp][] = [
      ['not square', { icon: 'icon.png' }, { 'icon.png': png(300, 256) }, /n'est pas carrée/],
      ['too small', { icon: 'icon.png' }, { 'icon.png': png(128, 128) }, /moins de 256 px/],
      ['an SVG', { icon: 'icon.svg' }, { 'icon.svg': '<svg><script>alert(1)</script></svg>' }, /n'est pas un PNG/],
      ['missing', { icon: 'icon.png' }, {}, /n'est pas un fichier du dossier/],
    ];
    // A PNG whose header holds but whose pixels do not decode: its data cut short.
    const broken = png(512, 512);
    tries.push(['undecodable', { icon: 'icon.png' }, { 'icon.png': broken.subarray(0, broken.length - 30) }, /ne se décode pas/]);
    let n = 0;
    for (const [what, manifest, files, why] of tries) {
      const tag = `v0.${++n}.0`;
      gh.push(REPO, tag, pluginFiles(REPO, { manifest: { version: `0.${n}.0`, ...manifest }, files }));
      const { id, state } = await submit(sam, REPO, tag);
      expect(state, what).toBe('rejected');
      const view = (await versionView(db, sam, id!))!;
      const failed = view.checks.filter((c) => c.status !== 'pass');
      expect(
        failed.map((c) => c.key),
        what,
      ).toEqual(['manifest']);
      expect(failed[0]!.problems.map((p) => p.path)).toEqual(['icon']);
      expect(failed[0]!.problems[0]!.why.fr, what).toMatch(why);
      const [v] = await q('select icon, version_json from versions where id = $1', [id]);
      expect(v).toEqual({ icon: null, version_json: null });
    }
  });

  it('publishes on the author’s click: the plugin made with its categories, unverified, in the review queue', async () => {
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO));
    const { id } = await submit(sam, REPO, 'v0.3.0');
    expect(await publishVersion(db, sam, id!, { categories: [], review: 'on' })).toEqual({ error: 'categories' });
    expect(
      await publishVersion(db, sam, id!, { categories: ['notifications', 'nope'], keywords: 'Phone, ntfy, , <b>', review: 'on' }),
    ).toEqual({ ok: true });
    const [p] = await q('select * from plugins');
    expect(p).toMatchObject({
      id: 'io.github.sroux.escouade-ntfy',
      repo_full_name: REPO,
      categories: ['notifications'],
      keywords: ['phone', 'ntfy'],
      auto_publish: false,
    });
    const [v] = await q('select state, review, published_at from versions where id = $1', [id]);
    expect(v).toMatchObject({ state: 'published', review: true });
    expect(await publishVersion(db, sam, id!, {})).toEqual({ error: 'not-ready' });
    const journal = (await q('select action from audit order by id')).map((r) => r.action);
    expect(journal).toEqual(['account.signed_in', 'version.submitted', 'version.ready', 'version.published']);
  });

  it('refuses a private repository, one it does not manage, a tag that is not there, and too many submissions', async () => {
    expect(await startSubmission(db, deps, sam, { repo: 'sroux/secret', tag: 'v1' }, T0)).toEqual({ error: 'private' });
    expect(await startSubmission(db, deps, sam, { repo: 'other/repo', tag: 'v1' }, T0)).toEqual({ error: 'not-yours' });
    expect(await startSubmission(db, deps, sam, { repo: REPO, tag: 'v9.9.9' }, T0)).toEqual({ error: 'no-tag' });
    // A repository made private since the last webhook: GitHub's word wins.
    gh.repos.get(REPO)!.private = true;
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO));
    expect(await startSubmission(db, deps, sam, { repo: REPO, tag: 'v0.3.0' }, T0)).toEqual({ error: 'private' });
    gh.repos.get(REPO)!.private = false;
    for (let i = 0; i < SUBMISSIONS_PER_HOUR; i++) {
      gh.push(REPO, `v0.3.${i + 1}`, pluginFiles(REPO));
      expect(await startSubmission(db, deps, sam, { repo: REPO, tag: `v0.3.${i + 1}` }, T0)).toMatchObject({ again: false });
    }
    gh.push(REPO, 'v0.4.0', pluginFiles(REPO));
    expect(await startSubmission(db, deps, sam, { repo: REPO, tag: 'v0.4.0' }, T0)).toEqual({ error: 'too-many' });
  });

  it('rejects a booby-trapped version, says why, and keeps none of its files', async () => {
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO, { files: { 'docs/key': { symlink: '/home/sam/.ssh/id_ed25519' } } }), 'Apache-2.0');
    const { id, state } = await submit(sam, REPO, 'v0.3.0');
    expect(state).toBe('rejected');
    const view = (await versionView(db, sam, id!))!;
    // The link is refused, the rest of the archive is read: every failure is said at once.
    expect(view.checks.filter((c) => c.status !== 'pass').map((c) => [c.key, c.status])).toEqual([
      ['license', 'fail'],
      ['archive', 'fail'],
    ]);
    const [v] = await q('select version_json, archive_sha256 from versions where id = $1', [id]);
    expect(v.version_json).toBeNull();
    await expect(readFile(join(dir, 'sha256', v.archive_sha256.slice(0, 2), `${v.archive_sha256}.tar.gz`))).rejects.toThrow();
  });

  it('a version with a program: the release’s binaries kept, their attestations read, in version.json', async () => {
    const commit = gh.push(REPO, 'v0.3.0', pluginFiles(REPO, { manifest: { run: { command: 'bin/ntfy', windows: 'bin/ntfy.exe' } } }));
    const asset = (platform: string, bytes: Buffer): Asset & { bytes: Buffer } => ({
      name: `ntfy-0.3.0-${platform}.tar.gz`,
      size: bytes.length,
      url: `https://api.github.test/assets/${platform}`,
      bytes,
    });
    const bins = { 'windows-x64': Buffer.from('win'), 'macos-arm64': Buffer.from('mac'), 'linux-x64': Buffer.from('linux') };
    gh.repos.get(REPO)!.releases['v0.3.0'] = Object.entries(bins).map(([p, b]) => asset(p, b));
    for (const [p, b] of Object.entries(bins))
      if (p !== 'linux-x64') gh.attestations.set(sha256(b), [attestation(sha256(b), { repo: REPO, commit })]);
    const { id, state } = await submit(sam, REPO, 'v0.3.0');
    expect(state).toBe('ready');
    const [v] = await q('select version_json from versions where id = $1', [id]);
    const file = JSON.parse(v.version_json);
    expect(file.binaries).toEqual({
      'windows-x64': { name: 'ntfy-0.3.0-windows-x64.tar.gz', sha256: sha256(bins['windows-x64']), size: 3, attested: true },
      'macos-arm64': { name: 'ntfy-0.3.0-macos-arm64.tar.gz', sha256: sha256(bins['macos-arm64']), size: 3, attested: true },
      'linux-x64': { name: 'ntfy-0.3.0-linux-x64.tar.gz', sha256: sha256(bins['linux-x64']), size: 5, attested: false },
    });
    const kept = await readFile(join(dir, 'sha256', sha256(bins['linux-x64']).slice(0, 2), `${sha256(bins['linux-x64'])}.tar.gz`));
    expect(kept.toString()).toBe('linux');
    const view = (await versionView(db, sam, id!))!;
    expect(view.checks.find((c) => c.key === 'binaries')!.detail.fr).toMatch(/sans attestation de provenance pour 1/);
  });

  it('an id of another author is refused; a name too close is held for a person', async () => {
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO));
    const { id } = await submit(sam, REPO, 'v0.3.0');
    await publishVersion(db, sam, id!, { categories: ['notifications'] });

    gh.add('q7x/ntfy-clone', 90);
    const q7x = await signInAs('q7x', 5151, [{ id: 90, full_name: 'q7x/ntfy-clone', private: false }]);
    gh.push(
      'q7x/ntfy-clone',
      'v1.0.0',
      pluginFiles('q7x/ntfy-clone', { manifest: { id: 'io.github.sroux.escouade-ntfy', version: '1.0.0' } }),
    );
    const stolen = await submit(q7x, 'q7x/ntfy-clone', 'v1.0.0');
    expect(stolen.state).toBe('rejected');
    expect((await versionView(db, q7x, stolen.id!))!.checks.find((c) => c.key === 'id')!.detail.en).toBe(
      'io.github.sroux.escouade-ntfy belongs to another author',
    );

    gh.push(
      'q7x/ntfy-clone',
      'v1.0.1',
      pluginFiles('q7x/ntfy-clone', { manifest: { id: 'io.github.q7x.escouade-ntfi', version: '1.0.1' } }),
    );
    const close = await submit(q7x, 'q7x/ntfy-clone', 'v1.0.1');
    expect(close.state).toBe('held');
    expect(await publishVersion(db, q7x, close.id!, { categories: ['notifications'] })).toEqual({ error: 'held' });
    // Nobody else sees a version that is not theirs.
    expect(await versionView(db, sam, close.id!)).toBeNull();
    expect(await publishVersion(db, sam, close.id!, { categories: ['notifications'] })).toEqual({ error: 'not-found' });
  });

  it('a new tag by webhook becomes a version that waits for its author, or is published when they asked', async () => {
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO));
    const first = await submit(sam, REPO, 'v0.3.0');
    await publishVersion(db, sam, first.id!, { categories: ['notifications'] });

    gh.push(REPO, 'v0.4.0', pluginFiles(REPO, { manifest: { version: '0.4.0' } }));
    const hook = tagOfWebhook('create', {
      ref: 'v0.4.0',
      ref_type: 'tag',
      repository: { id: 77, full_name: REPO, private: false },
      installation: { id: 42420, account: { id: 4242, login: 'sroux', type: 'User' } },
    })!;
    expect(hook).toEqual({ repoId: 77, fullName: REPO, installationId: 42420, tag: 'v0.4.0' });
    const { check } = await onTag(db, deps, hook, T0);
    expect(await checkVersion(db, deps, check!)).toBe('ready');
    const [v] = await q('select origin, account_id, plugin_id from versions where id = $1', [check]);
    expect(v).toMatchObject({ origin: 'webhook', account_id: sam.id, plugin_id: 'io.github.sroux.escouade-ntfy' });
    expect((await versionView(db, sam, check!))!).toMatchObject({ first: false, previous: '0.3.0' });
    // The same tag told again (a release after the tag) adds nothing.
    expect(await onTag(db, deps, hook, T0)).toEqual({});

    await q('update plugins set auto_publish = true');
    gh.push(REPO, 'v0.5.0', pluginFiles(REPO, { manifest: { version: '0.5.0' } }));
    const next = await onTag(db, deps, { ...hook, tag: 'v0.5.0' }, T0);
    expect(await checkVersion(db, deps, next.check!)).toBe('published');

    // A tag of a repository that is no plugin's is only noted.
    expect(await onTag(db, deps, { ...hook, repoId: 999, fullName: 'x/y' }, T0)).toEqual({});
    // A tag whose number is taken goes through the checks, which say so.
    gh.push(REPO, '0.5.0', pluginFiles(REPO, { manifest: { version: '0.5.0' } }));
    const dup = await onTag(db, deps, { ...hook, tag: '0.5.0' }, T0);
    expect(await checkVersion(db, deps, dup.check!)).toBe('rejected');

    // A transfer freezes the publications 7 days: a new tag then waits for a click, even with auto-publish on.
    await q(`update plugins set frozen_until = now() + interval '7 days'`);
    gh.push(REPO, 'v0.6.0', pluginFiles(REPO, { manifest: { version: '0.6.0' } }));
    const frozen = await onTag(db, deps, { ...hook, tag: 'v0.6.0' }, T0);
    expect(await checkVersion(db, deps, frozen.check!)).toBe('ready');
    // A plugin deleted takes no new tag; once purged, the numbers it used stay taken.
    await q(`update plugins set frozen_until = null, deleted_at = now()`);
    gh.push(REPO, 'v0.7.0', pluginFiles(REPO, { manifest: { version: '0.7.0' } }));
    expect(await onTag(db, deps, { ...hook, tag: 'v0.7.0' }, T0)).toEqual({});
    await q(`update plugins set deleted_at = null, retired_versions = '["0.9.0"]'`);
    gh.push(REPO, 'v0.8.0', pluginFiles(REPO, { manifest: { version: '0.8.0' } }));
    const below = await onTag(db, deps, { ...hook, tag: 'v0.8.0' }, T0);
    expect(await checkVersion(db, deps, below.check!)).toBe('rejected');
  });

  it('a tag moved after publication is noted and told; the published version does not change', async () => {
    const commit = gh.push(REPO, 'v0.3.0', pluginFiles(REPO));
    const { id } = await submit(sam, REPO, 'v0.3.0');
    await publishVersion(db, sam, id!, { categories: ['notifications'] });
    const before = (await q('select version_json, version_sha256, commit from versions where id = $1', [id]))[0];

    const moved = gh.push(REPO, 'v0.3.0', pluginFiles(REPO, { files: { 'evil.js': 'steal()' } }));
    expect(moved).not.toBe(commit);
    expect(await onTag(db, deps, { repoId: 77, fullName: REPO, installationId: null, tag: 'v0.3.0' }, T0)).toEqual({ moved: id });
    const after = (await q('select version_json, version_sha256, commit, state, tag_moved_to from versions where id = $1', [id]))[0];
    expect(after).toMatchObject({ ...before, state: 'published', tag_moved_to: moved });
    expect((await q("select detail from audit where action = 'version.tag_moved'"))[0].detail).toMatchObject({ was: commit, now: moved });
    // Told once: the same move seen again by the weekly read adds nothing.
    expect(await rereadTags(db, deps, T0)).toEqual({ read: 1, moved: 0, failed: 0 });

    // A tag moved without a webhook is caught by the weekly read.
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO, { files: { 'other.js': 'x' } }));
    expect(await rereadTags(db, deps, T0)).toEqual({ read: 1, moved: 1, failed: 0 });
  });

  it('the store’s own failure leaves the version in error, and the checks run again', async () => {
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO));
    gh.fail(true);
    const { id, state } = await submit(sam, REPO, 'v0.3.0');
    expect(state).toBe('error');
    expect((await versionView(db, sam, id!))!.error).toBe('GitHub answered 502 to the archive');
    gh.fail(false);
    expect(await retrySubmission(db, sam, id!)).toBe(true);
    expect(await checkVersion(db, deps, id!)).toBe('ready');
    expect(await retrySubmission(db, sam, id!)).toBe(false);
  });

  it('notes the tags of the webhooks, once per delivery', async () => {
    const p = { ref: 'refs/tags/v0.3.0', repository: { id: 77, full_name: REPO, private: false } };
    expect(await applyWebhook(db, 'push', 'd-1', p, T0)).toBe('handled');
    expect(await applyWebhook(db, 'push', 'd-1', p, T0)).toBe('duplicate');
    expect(tagOfWebhook('push', p)).toMatchObject({ tag: 'v0.3.0' });
    expect(tagOfWebhook('push', { ...p, deleted: true })).toBeNull();
    expect(tagOfWebhook('push', { ...p, ref: 'refs/heads/main' })).toBeNull();
    expect(tagOfWebhook('release', { ...p, action: 'published', release: { tag_name: 'v1.0.0' } })).toMatchObject({ tag: 'v1.0.0' });
    expect(tagOfWebhook('release', { ...p, action: 'edited', release: { tag_name: 'v1.0.0' } })).toBeNull();
  });
  it('renders the README at the check, and the public pages show the version once published (STORE-4)', async () => {
    gh.push(REPO, 'v0.3.0', pluginFiles(REPO, { files: { 'README.md': TRAPPED, 'store/1.png': png(120, 60) } }));
    const { id } = await submit(sam, REPO, 'v0.3.0');
    const [v] = await q('select docs from versions where id = $1', [id]);
    // Rendered once, cut down: nothing that runs, styles or tracks; the repository's image copied under its SHA-256.
    expect(v.docs.readme.fr).toBe(v.docs.readme.en);
    expect(v.docs.readme.fr).not.toMatch(/<script|<style|onerror|tracker\.example|href="javascript/i);
    expect(v.docs.images).toHaveLength(1);
    const [name] = v.docs.images;
    expect(v.docs.readme.fr).toContain(`src="/files/${name}"`);
    const copied = await readFile(join(dir, 'sha256', name.slice(0, 2), name));
    expect(imageInfo(copied)).toEqual({ format: 'png', width: 120, height: 60 });

    // Not published yet: not on the public pages.
    expect(await pluginPage(db, dir, 'io.github.sroux.escouade-ntfy')).toBeNull();
    await publishVersion(db, sam, id!, { categories: ['notifications'], keywords: 'phone', review: 'on' });
    const page = await pluginPage(db, dir, 'io.github.sroux.escouade-ntfy');
    expect(page!.entry).toMatchObject({ trust: 'review', version: '0.3.0', author: { login: 'sroux', githubId: 4242 } });
    expect(page!.latest!.docs.readme.fr).toBe(v.docs.readme.fr);
    expect(page!.latest!.screenshots[0]!.src).toMatch(/^\/files\/[0-9a-f]{64}\.png$/);
    expect(page!.permissions.map((p) => p.text.en)).toEqual(['Gets these events', 'Connects to']);
    expect(page!.versions.map((x) => x.version)).toEqual(['0.3.0']);
    expect(search(await catalog(db), { q: 'telephone' }).results.map((e) => e.id)).toEqual(['io.github.sroux.escouade-ntfy']);
    expect((await authorPage(db, 'SROUX'))!.plugins.map((e) => e.id)).toEqual(['io.github.sroux.escouade-ntfy']);
    expect(await authorPage(db, 'nobody')).toBeNull();

    // The installations counted until yesterday (STORE-6), as the app's catalogue says them.
    const yesterday = new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);
    const today = new Date().toISOString().slice(0, 10);
    await q(
      `insert into installs_daily (plugin_id, version, day, event, count) values ($1, '0.3.0', $2, 'install', 7), ($1, '0.3.0', $3, 'install', 5)`,
      ['io.github.sroux.escouade-ntfy', yesterday, today],
    );
    expect((await pluginPage(db, dir, 'io.github.sroux.escouade-ntfy'))!.entry.installs).toBe(7);

    // A version the revocation list disables leaves the pages; with it the plugin's only one, the plugin too.
    const list = {
      serial: 1,
      issued_at: '2026-10-29T10:00:00Z',
      entries: [
        { id: 'io.github.sroux.escouade-ntfy', versions: ['0.3.0'], action: 'disable', reason: { fr: 'x', en: 'x' }, date: '2026-10-29' },
      ],
    };
    await q(`insert into revocation_lists (serial, json, minisig, issued_at) values (1, $1, 'sig', now())`, [JSON.stringify(list)]);
    expect(await pluginPage(db, dir, 'io.github.sroux.escouade-ntfy')).toBeNull();
    expect((await catalog(db)).some((e) => e.id === 'io.github.sroux.escouade-ntfy')).toBe(false);
    await q('delete from revocation_lists');

    // A version published before STORE-4 has no docs: rendered from its kept archive the first time it is shown.
    await q('update versions set docs = null where id = $1', [id]);
    const again = await pluginPage(db, dir, 'io.github.sroux.escouade-ntfy');
    expect(again!.latest!.docs.readme.fr).toBe(v.docs.readme.fr);
    const [after] = await q('select docs from versions where id = $1', [id]);
    expect(after.docs).not.toBeNull();
  });

  it('takes a report from anyone with an address, at most five an hour from one address', async () => {
    const form = { reason: 'malicious', details: 'It reads ~/.ssh and sends it away.', email: ' Eve@Exemple.DEV ', faith: 'yes' };
    const ctx = { pluginId: 'io.github.sroux.escouade-ntfy', version: '0.3.0', lang: 'en' as const };
    expect(await fileReport(db, { ...form, reason: 'boring' }, ctx, T0)).toEqual({ error: 'reason' });
    expect(await fileReport(db, { ...form, details: 'short' }, ctx, T0)).toEqual({ error: 'details' });
    expect(await fileReport(db, { ...form, email: 'eve' }, ctx, T0)).toEqual({ error: 'email' });
    expect(await fileReport(db, { ...form, faith: undefined }, ctx, T0)).toEqual({ error: 'faith' });
    for (let i = 0; i < 5; i++) expect(await fileReport(db, form, ctx, T0)).toMatchObject({ email: 'Eve@exemple.dev' });
    expect(await fileReport(db, form, ctx, T0)).toEqual({ error: 'too-many' });
    expect(await fileReport(db, form, ctx, new Date(T0.getTime() + 3601_000))).toMatchObject({ email: 'Eve@exemple.dev' });
    const rows = await q('select plugin_id, version, reason, state, lang from reports');
    expect(rows).toHaveLength(6);
    expect(rows[0]).toEqual({ plugin_id: ctx.pluginId, version: '0.3.0', reason: 'malicious', state: 'open', lang: 'en' });
    // The journal keeps what was reported, not who reported it.
    const journal = await q("select account_id, detail from audit where action = 'report.filed'");
    expect(journal).toHaveLength(6);
    expect(JSON.stringify(journal)).not.toContain('Eve');
  });
});
