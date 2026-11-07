import { execFile, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { firstHand } from '../../scripts/plugins-publish/publish';
import { MIT } from '../fake/plugins.mjs';
import { testKey } from '../fake/minisign.mjs';
import { FAKE_ADMIN_TOKEN } from '../fake/review-store.mjs';

// `npm run plugins:publish` whole (scripts/plugins-publish/, docs/PLUGINS.md §8.6.2): the command as the chief runs it, its
// `plugin-review` included, in a temporary Git repository with a bare `origin`, against the fake store of the app
// (tests/fixtures/store/fake-store.mjs). Signed by the stand-in for minisign with a key made for the test: no real store,
// no tag on this repository, no real signature. The store's image has neither Git nor tests/fixtures: skipped there.

const here = dirname(fileURLToPath(import.meta.url));
const STORE = resolve(here, '../..');
const FAKE_STORE = resolve(here, '../../../tests/fixtures/store/fake-store.mjs');
const MINISIGN = join(here, '../fake/fake-minisign.mjs');
const runnable = existsSync(FAKE_STORE) && spawnSync('git', ['--version']).status === 0;

type Publishing = {
  calls: string[];
  reports: { id: string; version: string; verdict: string | null }[];
  signatures: { id: string; version: string }[];
};
type FakeStore = { url: string; state: { publishing: Publishing & Record<string, unknown> }; close: () => Promise<void> };

const manifest = (id: string, version: string, store: unknown = { categories: ['themes'] }) =>
  JSON.stringify({
    id,
    name: { fr: id, en: id },
    version,
    description: { fr: 'Un plugin.', en: 'A plugin.' },
    license: 'MIT',
    repository: 'https://github.com/guillaume-gagnaire/escouade',
    escouade: { api: '1', min: '2.2.0' },
    ...(store ? { store } : {}),
  });

describe('the folders of this repository', () => {
  it('goes through the 23 modules, and the run.js and declarative plugins beside them, none left at 0.0.0', () => {
    const repo = resolve(here, '../../..');
    const all = firstHand(repo);
    const kind = (short: string) =>
      Object.keys(
        (JSON.parse(readFileSync(join(repo, 'plugins', short, 'escouade-plugin.json'), 'utf8')) as { run?: object }).run ?? {},
      )[0];
    expect(all.filter((p) => kind(p.short) === 'wasm')).toHaveLength(23);
    expect(all.filter((p) => kind(p.short) !== 'wasm').map((p) => p.short)).toEqual([
      'discord',
      'goose',
      'high-contrast',
      'slack',
      'team-rules',
    ]);
    expect(all.filter((p) => p.version === '0.0.0')).toEqual([]);
  });
});

describe.runIf(runnable)('npm run plugins:publish', () => {
  let root: string;
  let app: string;
  let origin: string;
  let reports: string;
  let store: FakeStore;

  const git = (cwd: string, ...args: string[]) => {
    const r = spawnSync('git', args, { cwd, encoding: 'utf8' });
    return { ok: r.status === 0, out: (r.stdout ?? '').trim() };
  };
  const addPlugin = (short: string, id: string, version: string, store?: unknown) => {
    mkdirSync(join(app, 'plugins', short), { recursive: true });
    writeFileSync(join(app, 'plugins', short, 'escouade-plugin.json'), manifest(id, version, store));
    writeFileSync(join(app, 'plugins', short, 'README.md'), `# ${short}\n`);
    writeFileSync(join(app, 'plugins', short, 'LICENSE'), MIT);
  };
  const commit = (message: string) => {
    git(app, 'add', '.');
    git(app, '-c', 'user.name=T', '-c', 'user.email=t@example.com', 'commit', '-qm', message);
  };

  /** The command, as a process of its own: the fake store lives in this one, and must keep answering. */
  const run = (args: string[], env: Record<string, string> = {}) =>
    new Promise<{ code: number; out: string }>((done) => {
      const clean = Object.fromEntries(Object.entries(process.env).filter(([k]) => !/^(ESCOUADE_|GITHUB_TOKEN|GH_TOKEN|STORE_)/.test(k)));
      execFile(
        process.execPath,
        ['--import', 'tsx', 'scripts/plugins-publish/index.ts', ...args],
        {
          cwd: STORE,
          env: {
            ...clean,
            ESCOUADE_STORE_URL: store.url,
            ESCOUADE_STORE_TOKEN: FAKE_ADMIN_TOKEN,
            ESCOUADE_REVIEW_GITHUB_API: `${store.url}/github`,
            ESCOUADE_REVIEW_HOME: reports,
            ESCOUADE_REVIEW_WORK: join(root, 'work'),
            ESCOUADE_REVIEW_ENGINE: 'none',
            ESCOUADE_MINISIGN: MINISIGN,
            ESCOUADE_SIGNING_KEY: join(root, 'store.key'),
            STORE_KEYS_DIR: join(root, 'keys'),
            ESCOUADE_PUBLISH_REPO: app,
            ESCOUADE_PUBLISH_POLL_MS: '10',
            ...env,
          },
        },
        (e, stdout, stderr) => done({ code: e ? ((e as { code?: number }).code ?? 1) : 0, out: `${stdout}\n${stderr}` }),
      );
    });

  const reportOf = (id: string) => join(reports, id, '2.2.0.md');
  /** What the reader does: the verdict in the findings. */
  const read = (id: string, verdict: string) =>
    writeFileSync(reportOf(id), `${readFileSync(reportOf(id), 'utf8')}\nLu par le chef.\n\n**Verdict final : ${verdict}**\n`);
  const catalogued = async (id: string) => {
    const cat = (await (await fetch(`${store.url}/api/v1/catalog.json`)).json()) as {
      plugins: { id: string; categories: string[]; versions: { version: string; verified: boolean }[] }[];
    };
    return cat.plugins.find((p) => p.id === id);
  };
  const pushedTags = () => git(origin, 'tag', '--list').out;

  beforeEach(async () => {
    root = mkdtempSync(join(tmpdir(), 'plugins-publish-'));
    app = join(root, 'app');
    origin = join(root, 'origin.git');
    reports = join(root, 'reports');
    git(root, 'init', '-q', '--bare', '-b', 'main', origin);
    git(root, 'init', '-q', '-b', 'main', app);
    git(app, 'remote', 'add', 'origin', origin);
    addPlugin('alpha', 'dev.escouade.alpha-cli', '2.2.0');
    addPlugin('beta', 'dev.escouade.beta', '2.2.0');
    addPlugin('core', 'dev.escouade.core', '0.0.0');
    commit('The plugins');
    git(app, 'push', '-q', 'origin', 'main');
    const key = testKey();
    writeFileSync(join(root, 'store.key'), key.secret);
    mkdirSync(join(root, 'keys'));
    writeFileSync(join(root, 'keys', 'store.pub'), key.pub);
    const { startFakeStore } = (await import(/* @vite-ignore */ pathToFileURL(FAKE_STORE).href)) as {
      startFakeStore: () => Promise<FakeStore>;
    };
    store = await startFakeStore();
    await fetch(`${store.url}/_fake/publishing`, { method: 'POST', body: JSON.stringify({ git: origin }) });
    // Git and the keys, on a machine that may be busy.
  }, 60_000);
  afterEach(async () => {
    await store.close();
    rmSync(root, { recursive: true, force: true });
  }, 30_000);

  it(
    'tags, submits, publishes and reviews, stops for the reading, then reports and signs, and redoes nothing',
    { timeout: 180_000 },
    async () => {
      const first = await run(['--changed']);
      expect(first.out).toContain('attend sa lecture');
      expect(first.code).toBe(1);
      // The tag on the pushed commit, the second plugin not begun, the one still in the core left aside.
      expect(pushedTags()).toBe('plugins/alpha/v2.2.0');
      expect(git(origin, 'rev-parse', 'plugins/alpha/v2.2.0').out).toBe(git(app, 'rev-parse', 'HEAD').out);
      expect(first.out).toContain('1 plugin(s) pas encore portés');
      expect(store.state.publishing.calls).toEqual(['submit_version', 'get_version', 'get_version', 'publish_version']);
      expect(store.state.publishing.reports).toEqual([]);
      expect(store.state.publishing.signatures).toEqual([]);
      expect((await catalogued('dev.escouade.alpha-cli'))?.categories).toEqual(['themes']);
      expect((await catalogued('dev.escouade.alpha-cli'))?.versions[0]).toMatchObject({ version: '2.2.0', verified: false });

      // Run again, the report still waits: no second submission, the report not written over.
      const unread = readFileSync(reportOf('dev.escouade.alpha-cli'), 'utf8');
      expect((await run(['alpha'])).code).toBe(1);
      expect(readFileSync(reportOf('dev.escouade.alpha-cli'), 'utf8')).toBe(unread);
      expect(store.state.publishing.calls).toHaveLength(4);

      read('dev.escouade.alpha-cli', 'Vérifier');
      const second = await run(['alpha']);
      expect(second.out).toContain('publiée avec son sceau');
      expect(second.code).toBe(0);
      expect(store.state.publishing.calls).toHaveLength(4);
      expect(store.state.publishing.reports).toMatchObject([{ id: 'dev.escouade.alpha-cli', version: '2.2.0', verdict: 'Vérifier' }]);
      expect(store.state.publishing.signatures).toEqual([{ id: 'dev.escouade.alpha-cli', version: '2.2.0' }]);
      expect((await catalogued('dev.escouade.alpha-cli'))?.versions[0]).toMatchObject({ verified: true });
      expect(readFileSync(reportOf('dev.escouade.alpha-cli'), 'utf8')).toContain('Lu par le chef.');

      const again = await run(['alpha']);
      expect(again.code).toBe(0);
      expect(again.out).toContain('Rien à publier');
      expect(store.state.publishing.calls).toHaveLength(4);
      expect(store.state.publishing.reports).toHaveLength(1);
      expect(store.state.publishing.signatures).toHaveLength(1);

      // --changed goes on with the next one.
      const next = await run(['--changed']);
      expect(next.out).toContain('dev.escouade.beta@2.2.0 : le rapport attend sa lecture');
      expect(next.code).toBe(1);
      expect(pushedTags().split('\n')).toEqual(['plugins/alpha/v2.2.0', 'plugins/beta/v2.2.0']);

      for (const line of [first.out, second.out].join('\n').split('\n')) {
        expect(line).not.toContain(FAKE_ADMIN_TOKEN);
        expect(line).not.toContain('store.key');
      }
    },
  );

  it('stops on a red check, with its reason', { timeout: 60_000 }, async () => {
    const red = { 'dev.escouade.alpha-cli': { key: 'license', why: 'La LICENSE ne dit pas sa licence.' } };
    await fetch(`${store.url}/_fake/publishing`, { method: 'POST', body: JSON.stringify({ red }) });
    const { code, out } = await run(['--changed']);
    expect(code).toBe(1);
    expect(out).toContain('un contrôle est rouge');
    expect(out).toContain('license : La LICENSE ne dit pas sa licence.');
    expect(store.state.publishing.calls).not.toContain('publish_version');
    expect(pushedTags()).toBe('plugins/alpha/v2.2.0');
    expect(existsSync(reportOf('dev.escouade.alpha-cli'))).toBe(false);
  });

  it('stops on a verdict that is not « Vérifier », and on a signature that fails', { timeout: 180_000 }, async () => {
    expect((await run(['alpha'])).code).toBe(1);
    const asked = readFileSync(reportOf('dev.escouade.alpha-cli'), 'utf8');
    read('dev.escouade.alpha-cli', 'Demander des changements');
    const { code, out } = await run(['alpha', 'beta']);
    expect(code).toBe(1);
    expect(out).toContain('verdict « Demander des changements »');
    expect(store.state.publishing.reports).toEqual([]);
    expect(store.state.publishing.signatures).toEqual([]);
    expect(pushedTags()).toBe('plugins/alpha/v2.2.0');

    writeFileSync(reportOf('dev.escouade.alpha-cli'), asked);
    read('dev.escouade.alpha-cli', 'Vérifier');
    const wrong = await run(['alpha'], { FAKE_MINISIGN_FAIL: '1' });
    expect(wrong.code).toBe(1);
    expect(wrong.out).toContain('plugin-review sign a échoué');
    expect(store.state.publishing.signatures).toEqual([]);
    expect((await run(['alpha'])).code).toBe(0);
    expect(store.state.publishing.signatures).toHaveLength(1);
  });

  it('refuses a dirty tree and a commit that is not on origin/main, and tags nothing', { timeout: 60_000 }, async () => {
    writeFileSync(join(app, 'plugins', 'alpha', 'stray.txt'), 'pas commité');
    const dirty = await run(['--changed']);
    expect(dirty.code).toBe(2);
    expect(dirty.out).toContain('l’arbre n’est pas propre');

    commit('Not pushed');
    const local = await run(['--changed']);
    expect(local.code).toBe(2);
    expect(local.out).toContain('n’est pas sur origin/main');
    expect(pushedTags()).toBe('');
    expect(git(app, 'tag', '--list').out).toBe('');
    expect(store.state.publishing.calls).toEqual([]);
  });

  it('stops at the store’s limit and takes up on the tag it posed', { timeout: 60_000 }, async () => {
    await fetch(`${store.url}/_fake/publishing`, { method: 'POST', body: JSON.stringify({ tooMany: true }) });
    const limited = await run(['beta']);
    expect(limited.code).toBe(1);
    expect(limited.out).toContain('relance dans une heure');
    expect(pushedTags()).toBe('plugins/beta/v2.2.0');

    await fetch(`${store.url}/_fake/publishing`, { method: 'POST', body: JSON.stringify({ tooMany: false }) });
    const resumed = await run(['beta']);
    expect(resumed.out).toContain('plugins/beta/v2.2.0 : déjà étiqueté');
    expect(resumed.code).toBe(1);
  });

  it(
    'leaves alone a version already at the store: signed, nothing; not signed, its review, never a submission',
    { timeout: 90_000 },
    async () => {
      addPlugin('discord', 'dev.escouade.discord', '1.1.0');
      addPlugin('slack', 'dev.escouade.slack', '1.1.0');
      commit('Those of the store');
      git(app, 'push', '-q', 'origin', 'main');
      const signed = await run(['discord']);
      expect(signed.out).toContain('Rien à publier');
      expect(signed.code).toBe(0);
      expect(pushedTags()).toBe('');
      expect(store.state.publishing.calls).toEqual([]);

      const unsigned = await run(['slack']);
      expect(unsigned.out).toContain('déjà au catalogue, non vérifiée');
      expect(unsigned.code).toBe(1);
      expect(pushedTags()).toBe('');
      expect(store.state.publishing.calls).toEqual([]);
      expect(store.state.publishing.signatures).toEqual([]);
    },
  );

  it('publishes a module that embeds its manifest under the tool’s categories', { timeout: 60_000 }, async () => {
    addPlugin('cursor', 'dev.escouade.cursor-cli', '2.2.0', null);
    commit('A harness');
    git(app, 'push', '-q', 'origin', 'main');
    expect((await run(['cursor'])).code).toBe(1);
    expect((await catalogued('dev.escouade.cursor-cli'))?.categories).toEqual(['harness']);
  });

  it('stops on a first version without a category, and refuses what cannot be published', { timeout: 60_000 }, async () => {
    addPlugin('bare', 'dev.escouade.bare', '2.2.0', null);
    addPlugin('other', 'io.github.someone.other', '2.2.0');
    commit('Two more');
    git(app, 'push', '-q', 'origin', 'main');
    const bare = await run(['bare']);
    expect(bare.code).toBe(1);
    expect(bare.out).toContain('store.categories');
    expect(store.state.publishing.calls).not.toContain('publish_version');

    for (const [name, says] of [
      ['core', 'n’est pas encore porté'],
      ['other', 'n’est pas un plugin de la première main'],
      ['nothing', 'n’est pas un plugin de la première main'],
    ] as const) {
      const { code, out } = await run([name]);
      expect(code).toBe(2);
      expect(out).toContain(says);
    }
    expect((await run([])).code).toBe(2);
  });
});
