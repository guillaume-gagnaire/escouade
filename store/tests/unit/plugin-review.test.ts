import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { verifyAttestation } from '../../scripts/plugin-review/attest';
import {
  benchFolder,
  DEFAULT_IMAGE,
  findCli,
  findEngine,
  playBench,
  refusals,
  runArgs,
  type BenchSetup,
} from '../../scripts/plugin-review/bench';
import { grownPermissions } from '../../scripts/plugin-review/diff';
import { baseVersion, catalog, download, Integrity, parseTarget } from '../../scripts/plugin-review/download';
import { extract, inside } from '../../scripts/plugin-review/extract';
import { findLeads, hostAllowed, type LeadKind } from '../../scripts/plugin-review/leads';
import { blockers, proposed, renderReport } from '../../scripts/plugin-review/report';
import { review, type ReviewOptions } from '../../scripts/plugin-review/review';
import { tarball } from '../fake/plugins.mjs';
import { exampleFiles, startReviewStore, TRAPPED } from '../fake/review-store.mjs';

// The local review (scripts/plugin-review/, docs/STORE.md §4.2) on PLG-10's examples and on a plugin booby-trapped on
// purpose, served by a fake store and a fake GitHub (tests/fake/review-store.mjs); the bench by a fake container engine
// (tests/fake/fake-engine.mjs). Nothing of a plugin runs here.

const here = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(here, '../../..');
const ENGINE = join(here, '../fake/fake-engine.mjs');
const files = (o: Record<string, unknown>) =>
  new Map(
    Object.entries(o)
      .filter(([, v]) => typeof v === 'string' || Buffer.isBuffer(v))
      .map(([k, v]) => [k, Buffer.from(v as string)]),
  );
const kinds = (l: { kind: LeadKind }[]) => [...new Set(l.map((x) => x.kind))].sort();

let root: string;
let store: Awaited<ReturnType<typeof startReviewStore>>;
const noBench: BenchSetup = { engine: null, cli: null, image: 'img', timeout: 10_000 };

beforeAll(async () => {
  root = mkdtempSync(join(tmpdir(), 'plugin-review-test-'));
  store = await startReviewStore();
});
afterAll(async () => {
  await store.close();
  rmSync(root, { recursive: true, force: true });
});

const options = (target: string, over: Partial<ReviewOptions> = {}): ReviewOptions => ({
  ...parseTarget(target)!,
  source: { store: store.url, githubApi: store.githubApi },
  work: join(root, 'work'),
  reports: join(root, 'reports'),
  bench: noBench,
  benchWhy: 'aucun moteur de conteneur (test)',
  repo: REPO,
  now: new Date('2026-10-29T10:00:00Z'),
  ...over,
});

describe('the target', () => {
  it('reads <id>@<version>, a v before the number allowed', () => {
    expect(parseTarget('dev.exemple.slack@1.3.0')).toEqual({ id: 'dev.exemple.slack', version: '1.3.0' });
    expect(parseTarget('dev.exemple.slack@v1.3.0-beta.1')).toEqual({ id: 'dev.exemple.slack', version: '1.3.0-beta.1' });
    for (const bad of ['', 'dev.exemple.slack', 'dev.exemple.slack@latest', '../x@1.0.0', 'A@1.0.0']) expect(parseTarget(bad)).toBeNull();
  });
});

describe('the leads', () => {
  it('point at everything the trapped plugin does past its permissions', () => {
    const leads = findLeads(files(TRAPPED), ['hooks.slack.com']);
    expect(kinds(leads)).toEqual(['dependencies', 'dynamic', 'exec', 'files', 'host', 'instructions', 'network', 'obscure', 'secrets']);
    const hosts = leads.filter((l) => l.kind === 'host');
    expect(hosts.map((h) => h.why)).toEqual([
      'collect.q7x-telemetry.net n’est pas dans permissions.network',
      'q7x-telemetry.net n’est pas dans permissions.network',
    ]);
    expect(hosts.every((h) => h.file === 'index.mjs' && h.line)).toBe(true);
    expect(leads.some((l) => l.kind === 'secrets' && l.text?.includes('.ssh/id_ed25519'))).toBe(true);
    expect(leads.some((l) => l.kind === 'dependencies' && l.why.includes('postinstall'))).toBe(true);
    expect(leads.some((l) => l.kind === 'dependencies' && l.why === 'dépendance hors du registre npm')).toBe(true);
    expect(leads.some((l) => l.kind === 'instructions' && l.file === 'CLAUDE.md' && !l.line)).toBe(true);
    expect(leads.some((l) => l.kind === 'instructions' && l.line === 3)).toBe(true);
    // What the base64 hides is shown.
    expect(leads.some((l) => l.why === 'texte en base64, décodé' && l.text?.includes('/etc/cron.d/x'))).toBe(true);
  });

  it('leave the examples quiet where they keep to their permissions', () => {
    const slack = findLeads(files(exampleFiles('slack')), ['hooks.slack.com']);
    expect(slack.filter((l) => ['host', 'exec', 'dynamic', 'dependencies', 'instructions'].includes(l.kind))).toEqual([]);
    expect(slack.some((l) => l.kind === 'network')).toBe(true);
    // Without the permission, the same calls are a lead.
    expect(findLeads(new Map([['a.mjs', Buffer.from("fetch('https://hooks.slack.com/x')")]]), []).map((l) => l.kind)).toEqual([
      'network',
      'host',
    ]);
    expect(findLeads(files(exampleFiles('high-contrast')), [])).toEqual([]);
  });

  it('read a binary as a lead, never as text, and skip images and prose', () => {
    const leads = findLeads(
      new Map([
        ['tool.bin', Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0, 1, 2])],
        ['shot.png', Buffer.from([0x89, 0x50, 0, 0])],
        ['README.md', Buffer.from('See https://other.example.net and call fetch(url).')],
      ]),
      [],
    );
    expect(leads).toEqual([{ kind: 'obscure', file: 'tool.bin', why: 'fichier binaire (7 octets) : on ne lit pas ce qu’il fait' }]);
  });

  it('let a host through by its exact name or a *.domain entry', () => {
    expect(hostAllowed('hooks.slack.com', ['hooks.slack.com'])).toBe(true);
    expect(hostAllowed('a.b.ghe.com', ['*.ghe.com'])).toBe(true);
    // The app's rule: `*.domain` covers its subdomains, not the domain itself, declared apart.
    expect(hostAllowed('ghe.com', ['*.ghe.com'])).toBe(false);
    expect(hostAllowed('ghe.com', ['*.ghe.com', 'ghe.com'])).toBe(true);
    expect(hostAllowed('Hooks.Slack.COM', ['hooks.slack.com'])).toBe(true);
    expect(hostAllowed('evilghe.com', ['*.ghe.com'])).toBe(false);
    expect(hostAllowed('slack.com.evil.net', ['slack.com'])).toBe(false);
  });
});

describe('the extraction', () => {
  it('writes the regular files alone, never outside its folder, never executable', () => {
    mkdirSync(join(root, 'extract-trapped'));
    const dest = join(root, 'extract-trapped', 'plugin');
    const gz = tarball(
      {
        'a.txt': 'a',
        'bin/run': '#!/bin/sh\necho hi',
        ssh: { symlink: '/home/user/.ssh' },
        hard: { hardlink: 'a.txt' },
        pipe: { fifo: true },
      },
      { raw: { '../../escape.txt': 'out', '/etc/evil': 'abs', 'top/../../../up.txt': 'up' } },
    );
    expect(() => extract(gz, dest)).not.toThrow();
    const got = readdirSync(dest, { recursive: true }).map(String).sort();
    expect(got.map((p) => p.replace(/\\/g, '/'))).toEqual(['a.txt', 'bin', 'bin/run']);
    expect(existsSync(join(root, 'escape.txt'))).toBe(false);
    expect(existsSync(join(root, 'extract-trapped', 'escape.txt'))).toBe(false);
    expect(existsSync(join(root, 'up.txt'))).toBe(false);
    if (process.platform !== 'win32') expect(statSync(join(dest, 'bin/run')).mode & 0o111).toBe(0);
  });

  it('says what it did not write, and wants a folder that does not exist yet', () => {
    const dest = join(root, 'extract-problems');
    const r = extract(tarball({ ok: 'ok', link: { symlink: '../x' } }, { raw: { '/etc/evil': 'x' } }), dest);
    expect(r.problems.map((p) => p.code)).toEqual(['symlink', 'absolute']);
    expect(() => extract(tarball({ ok: 'ok' }), dest)).toThrow();
  });

  it('checks each path against the folder again', () => {
    const d = join(root, 'x');
    expect(inside(d, 'a/b.txt')).toBe(true);
    for (const bad of ['', '../a', 'a/../../b', '/etc/passwd', 'C:/x', 'C:\\x', '\\\\server\\share', 'a\0b'])
      expect(inside(d, bad)).toBe(false);
  });
});

describe('the download', () => {
  it('checks version.json against the catalogue, the archive and the binaries against version.json', async () => {
    const src = { store: store.url, githubApi: store.githubApi };
    const cat = await catalog(src);
    const ok = await download(src, cat, 'dev.exemple.slack', '1.0.0');
    expect(ok.file.commit).toMatch(/^[0-9a-f]{40}$/);
    await expect(download(src, cat, 'io.github.q7x.slak-notify', '0.0.2')).rejects.toThrow(
      /^Archive : \d+ octets, version.json en dit \d+$/,
    );
    await expect(download(src, cat, 'dev.nobody', '1.0.0')).rejects.toBeInstanceOf(Integrity);
    // A store that serves a version.json other than the one the catalogue fingerprints.
    const lying: typeof fetch = async (url, init) => {
      const res = await fetch(url, init);
      if (String(url).endsWith('/version.json')) return new Response((await res.text()).replace('"1.0.0"', '"1.0.0" '), { status: 200 });
      return res;
    };
    await expect(download({ ...src, fetch: lying }, cat, 'dev.exemple.slack', '1.0.0')).rejects.toThrow(
      /^version.json : empreinte [0-9a-f]{64}, le catalogue dit/,
    );
  });

  it('diffs from the last verified version below, or from --since', async () => {
    const cat = await catalog({ store: store.url, githubApi: store.githubApi });
    const slack = cat.plugins.find((p) => p.id === 'dev.exemple.slack')!;
    expect(baseVersion(slack, '1.1.0')?.version).toBe('1.0.0');
    expect(baseVersion(slack, '1.0.0')).toBeNull();
    expect(baseVersion(slack, '1.0.0', '1.1.0')?.version).toBe('1.1.0');
    expect(() => baseVersion(slack, '1.1.0', '0.9.0')).toThrow(Integrity);
  });

  it('puts the permissions that grow first', () => {
    expect(
      grownPermissions(
        { permissions: { network: ['a.com'], decide: false }, contributes: { secrets: [] } },
        {
          permissions: { network: ['a.com', 'b.com'], decide: true, events: ['x'] },
          run: { command: 'x' },
          contributes: { secrets: [], webhooks: [] },
        },
      ),
    ).toEqual([
      { what: 'permissions.decide', added: 'oui' },
      { what: 'permissions.events', added: 'x' },
      { what: 'permissions.network', added: 'b.com' },
      { what: 'run', added: 'lance désormais un programme' },
      { what: 'contributes.webhooks', added: 'nouvelle contribution' },
    ]);
  });
});

describe('the bench in a container', () => {
  const setup = (mode: string, timeout = 30_000): BenchSetup => {
    process.env.FAKE_ENGINE_MODE = mode;
    return {
      engine: { program: process.execPath, prefix: [ENGINE] },
      cli: '/opt/escouade-plugin',
      image: 'node:22-trixie-slim',
      timeout,
    };
  };

  it('runs without network, read-only, bounded, unprivileged, the folder and the CLI mounted read-only', () => {
    const a = runArgs('/r/bench', '/opt/escouade-plugin', 'img', 'escouade-review-1');
    for (const flag of [
      '--network=none',
      '--read-only',
      '--memory=512m',
      '--memory-swap=512m',
      '--pids-limit=256',
      '--cap-drop=ALL',
      '--security-opt=no-new-privileges',
      '--user=65534:65534',
      '--rm',
    ])
      expect(a).toContain(flag);
    expect(a).toContain('/r/bench:/plugin:ro');
    expect(a).toContain('/opt/escouade-plugin:/usr/local/bin/escouade-plugin:ro');
    expect(a.slice(a.indexOf('img'))).toEqual(['img', 'escouade-plugin', 'test', '/plugin', '--json', '--lang', 'fr']);
    expect(a.join(' ')).not.toMatch(/--privileged|--network=host|:rw\b(?!,size)/);
  });

  it('is not played without an engine or a Linux escouade-plugin, and says why', async () => {
    expect(await playBench(noBench, root, 'aucun moteur')).toEqual({ played: false, why: 'aucun moteur' });
    const r = await playBench({ ...setup('pass'), cli: null }, root);
    expect(r.played).toBe(false);
    expect(!r.played && r.why).toMatch(/ESCOUADE_PLUGIN_LINUX/);
    expect(findEngine({ ESCOUADE_REVIEW_ENGINE: 'none' })).toEqual({ engine: null, why: 'ESCOUADE_REVIEW_ENGINE=none' });
    expect(findEngine({ ESCOUADE_REVIEW_ENGINE: 'no-such-engine-here' }).engine).toBeNull();
    expect(findCli({ ESCOUADE_PLUGIN_LINUX: join(root, 'missing') })).toBeNull();
    expect(findCli({}, 'win32', 'x64')).toBeNull();
  });

  it('reads the bench’s report, and what the sandbox refused', async () => {
    const pass = await playBench(setup('pass'), root);
    expect(pass.played && pass.passed).toBe(true);
    const refused = await playBench(setup('refused'), root);
    expect(refused.played && !refused.passed).toBe(true);
    expect(refused.played && refused.refused).toEqual([
      'Error: getaddrinfo EAI_AGAIN collect.q7x-telemetry.net',
      "Error: EROFS: read-only file system, open '/plugin/x'",
    ]);
    // A program that ended before it was kept: its last lines are in the scenario's words (as podman gave them).
    const ended = await playBench(setup('ended'), root);
    expect(ended.played && ended.refused).toEqual([
      'slak: fetch failed EAI_AGAIN',
      "slak: EROFS: read-only file system, open '/plugin/cache.json' EROFS",
    ]);
    const garbage = await playBench(setup('garbage'), root);
    expect(garbage.played && garbage.raw).toMatch(/^le banc n’a pas rendu son rapport \(code 139\)\nSegmentation fault/);
  });

  it('stops a bench past its time', async () => {
    const r = await playBench(setup('hang', 1500), root);
    expect(r.played && r.raw).toMatch(/^arrêté au bout de 2 s/);
  });

  it('plays the program of the release where the manifest names it, executable', () => {
    const src = join(root, 'bench-src');
    extract(tarball({ 'escouade-plugin.json': '{}', 'README.md': 'x' }), src);
    const dest = benchFolder(
      src,
      join(root, 'bench-dest'),
      { run: { command: 'bin/tool', windows: 'bin/tool.exe' } },
      Buffer.from('\x7fELF'),
    );
    expect(readFileSync(join(dest, 'bin/tool'), 'latin1')).toBe('\x7fELF');
    if (process.platform !== 'win32') expect(statSync(join(dest, 'bin/tool')).mode & 0o100).toBeTruthy();
    // A program path that leaves the folder is not written.
    benchFolder(src, join(root, 'bench-dest-2'), { run: { command: '../../evil' } }, Buffer.from('x'));
    expect(existsSync(join(root, 'evil'))).toBe(false);
  });

  it('picks the refusals out of what the plugin wrote', () => {
    expect(
      refusals(
        [
          { kind: 'stderr', line: 'connect ENETUNREACH 1.2.3.4:443' },
          { kind: 'stderr', line: 'hello' },
        ],
        'x\nopen EACCES /root',
      ),
    ).toEqual(['connect ENETUNREACH 1.2.3.4:443', 'open EACCES /root']);
  });
});

describe('the attestations', () => {
  const statement = (commit: string) => [
    {
      verificationResult: {
        statement: {
          predicate: {
            buildDefinition: {
              externalParameters: { workflow: { path: '.github/workflows/release.yml' } },
              resolvedDependencies: [{ digest: { gitCommit: commit } }],
            },
          },
        },
      },
    },
  ];
  const run =
    (status: number, out: unknown, err = '') =>
    () => ({ status, stdout: typeof out === 'string' ? out : JSON.stringify(out), stderr: err });

  it('want gh to verify a signature that names the commit relu', () => {
    const c = 'a'.repeat(40);
    let asked: string[] = [];
    expect(
      verifyAttestation('/w/binaries/linux-x64', 'o/r', c, (a) => {
        asked = a;
        return run(0, statement(c))();
      }),
    ).toEqual({ state: 'verified', workflow: '.github/workflows/release.yml' });
    expect(asked).toEqual(['attestation', 'verify', '/w/binaries/linux-x64', '--repo', 'o/r', '--format', 'json']);
    expect(verifyAttestation('/b', 'o/r', c, run(0, statement('b'.repeat(40))))).toEqual({ state: 'other-commit', commit: 'b'.repeat(40) });
    expect(verifyAttestation('/b', 'o/r', c, run(1, '', 'no attestations found'))).toEqual({
      state: 'refused',
      why: 'no attestations found',
    });
    expect(verifyAttestation('/b', 'o/r', c, run(4, '', 'To get started with GitHub CLI, please run:  gh auth login')).state).toBe(
      'unchecked',
    );
    expect(verifyAttestation('/b', 'o/r', c, () => ({ status: null, stdout: '', stderr: '', error: new Error('ENOENT') }))).toEqual({
      state: 'unchecked',
      why: '`gh` introuvable',
    });
  });
});

describe('the review', () => {
  it('reads an update on its diff, the permissions that grow first, and writes outside the repository', async () => {
    const done = await review(options('dev.exemple.slack@1.1.0'));
    expect(done.code).toBe(0);
    expect(done.report).toBe(join(root, 'reports', 'dev.exemple.slack', '1.1.0.md'));
    expect(done.data.folder!.startsWith(join(root, 'work', 'dev.exemple.slack-1.1.0-'))).toBe(true);
    expect(done.data.tagCheck).toEqual({ state: 'same', commit: done.data.commit });
    expect(done.data.base).toMatchObject({
      version: '1.0.0',
      grown: [
        { what: 'permissions.events', added: 'ticket.created' },
        { what: 'permissions.network', added: 'api.slack.com' },
      ],
      files: 2,
    });
    expect(readFileSync(done.data.base!.diff, 'utf8')).toContain('+const api = ');
    const md = readFileSync(done.report, 'utf8');
    expect(md.split('\n')[2]).toMatch(/^> \*\*Le banc de conformité n’a pas été lancé\*\* : aucun moteur de conteneur \(test\)/);
    expect(md).toContain('## Depuis la v1.0.0');
    expect(md).toContain('- **`permissions.network`** : api.slack.com');
    expect(md).toContain('## Constats de la revue');
    expect(readFileSync(done.summary, 'utf8')).toContain('- Réseau vers : hooks.slack.com, api.slack.com');
    expect(JSON.parse(readFileSync(done.json, 'utf8')).proposed).toEqual({
      verdict: null,
      why: 'à décider à la lecture du code : rien de mécanique ne s’y oppose',
    });
  });

  it('reviews the other examples without a blocker', async () => {
    for (const t of ['dev.exemple.team-rules@1.0.0', 'dev.exemple.high-contrast@1.0.0']) {
      const done = await review(options(t));
      expect(done.code).toBe(0);
      expect(blockers(done.data)).toEqual([]);
      expect(done.data.base).toBeNull();
      expect(readFileSync(done.report, 'utf8')).toContain('## Première revue');
    }
  });

  it('points at the trapped plugin: the tag moved, the entries refused, every lead', async () => {
    const done = await review(options('io.github.q7x.slak-notify@0.0.1'));
    expect(done.code).toBe(0);
    expect(done.data.tagCheck?.state).toBe('moved');
    expect(done.data.refusedEntries).toEqual([
      'lien symbolique non suivi : ssh',
      'chemin qui remonte refusé : ../../escape.txt',
      'chemin absolu refusé : /etc/evil',
    ]);
    expect(existsSync(join(done.data.work!, 'escape.txt'))).toBe(false);
    expect(proposed(done.data).verdict).toBe('Demander des changements');
    const md = readFileSync(done.report, 'utf8');
    for (const title of [
      'Hôtes hors de `permissions.network`',
      'Secrets et dossiers personnels',
      'Programmes lancés',
      'Code dynamique',
      'Dépendances',
      'Consignes aux agents',
    ])
      expect(md).toContain(`### ${title}`);
    expect(md).toContain('**l’étiquette `v0.0.1` a été déplacée**');
  });

  it('refuses a version whose archive is not the one fingerprinted, before reading anything', async () => {
    const done = await review(options('io.github.q7x.slak-notify@0.0.2'));
    expect(done.code).toBe(1);
    expect(done.data.integrity).toMatch(/^Archive : /);
    expect(done.data.folder).toBeUndefined();
    expect(readdirSync(done.data.work!)).toEqual([]);
    expect(existsSync(done.summary)).toBe(false);
    expect(readFileSync(done.report, 'utf8')).toContain('**Refusé avant toute lecture**');
    expect(proposed(done.data).verdict).toBe('Refuser');
  });

  it('plays the bench on the program, and keeps a report a reader already wrote in', async () => {
    process.env.FAKE_ENGINE_MODE = 'refused';
    const bench: BenchSetup = {
      engine: { program: process.execPath, prefix: [ENGINE] },
      cli: '/opt/escouade-plugin',
      image: 'img',
      timeout: 30_000,
    };
    const done = await review(options('io.github.q7x.slak-notify@0.0.1', { bench, now: new Date('2026-10-29T11:00:00Z') }));
    expect(done.data.bench).toMatchObject({ played: true, passed: false });
    const md = readFileSync(done.report, 'utf8');
    expect(md).not.toContain('n’a pas été lancé');
    expect(md).toContain('Refusé par le bac à sable');
    expect(md).toContain('`Error: getaddrinfo EAI_AGAIN collect.q7x-telemetry.net`');
    expect(blockers(done.data)).toContain('le banc de conformité échoue');
    // The report of the run before stays beside it.
    expect(readdirSync(join(root, 'reports', 'io.github.q7x.slak-notify')).filter((f) => /^0\.0\.1\.2026-/.test(f)).length).toBeGreaterThan(
      0,
    );
  });

  it('never works nor writes its report inside the repository', async () => {
    await expect(review(options('dev.exemple.slack@1.0.0', { reports: join(REPO, 'reviews') }))).rejects.toThrow(/est dans le dépôt/);
    await expect(review(options('dev.exemple.slack@1.0.0', { work: join(REPO, 'store', '.data') }))).rejects.toThrow(/est dans le dépôt/);
  });

  it('renders the report of a refusal alone', () => {
    const md = renderReport({ id: 'x', version: '1.0.0', date: 'd', store: 's', integrity: 'Archive : non' });
    expect(md).toContain('Verdict proposé : **Refuser**');
    expect(md).not.toContain('## Pistes');
  });
});

// The bench in a real container, asked for by ESCOUADE_REVIEW_REAL_BENCH=1 with an engine (docker or podman, on the PATH or
// ESCOUADE_REVIEW_ENGINE) and a Linux escouade-plugin (ESCOUADE_PLUGIN_LINUX); skipped otherwise, so the CI never
// depends on a container engine. Played on the poste on 2026-10-29 with podman 5.8.
const real = process.env.ESCOUADE_REVIEW_REAL_BENCH === '1';
describe.runIf(real)('the bench in a real container', () => {
  const setup = (): BenchSetup => {
    const { engine, why } = findEngine();
    const cli = findCli();
    if (!engine || !cli) throw new Error(`ESCOUADE_REVIEW_REAL_BENCH=1 wants an engine and ESCOUADE_PLUGIN_LINUX: ${why ?? 'no CLI'}`);
    return { engine, cli, image: process.env.ESCOUADE_REVIEW_IMAGE ?? DEFAULT_IMAGE, timeout: 5 * 60_000 };
  };

  it('passes the examples of PLG-10', async () => {
    for (const t of ['dev.exemple.slack@1.0.0', 'dev.exemple.team-rules@1.0.0', 'dev.exemple.high-contrast@1.0.0']) {
      const done = await review(options(t, { bench: setup() }));
      expect(done.data.bench, t).toMatchObject({ played: true, passed: true });
    }
  }, 900_000);

  it('fails the trapped plugin, the network and the writes refused by the sandbox', async () => {
    const done = await review(options('io.github.q7x.slak-notify@0.0.1', { bench: setup() }));
    const b = done.data.bench!;
    expect(b).toMatchObject({ played: true, passed: false });
    const refused = b.played ? b.refused.join('\n') : '';
    expect(refused).toMatch(/EAI_AGAIN|ENETUNREACH/);
    expect(refused).toMatch(/EROFS/);
  }, 300_000);
});
