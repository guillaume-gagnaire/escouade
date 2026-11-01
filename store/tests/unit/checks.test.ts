import { describe, expect, it } from 'vitest';
import { judgeAttestations } from '../../server/utils/attest';
import {
  CHECK_KEYS,
  closeNames,
  platformOf,
  rejectionText,
  runChecks,
  verdict,
  type CheckKey,
  type CheckResult,
  type FetchedBinary,
  type Snapshot,
} from '../../server/utils/checks';
import { GithubError } from '../../server/utils/github';
import type { Source } from '../../server/utils/source';
import { isFirstParty } from '../../server/utils/submissions';
import { readArchive } from '../../server/utils/tar';
import { attestation, MIT, pluginFiles, png, tarball } from '../fake/plugins.mjs';

// The eleven checks (docs/STORE.md §3.3) on plugins valid and booby-trapped: each trap fails its own check, says
// where and how to fix it in both languages, and the others still say what they found.

const REPO = 'sroux/escouade-ntfy';
const COMMIT = 'a41c9d2'.padEnd(40, '0');

function snapshot(files: Record<string, unknown>, over: Partial<Snapshot> = {}, raw = {}): Snapshot {
  const gz = tarball(files as Parameters<typeof tarball>[0], { commit: COMMIT, raw });
  return {
    repository: REPO,
    tag: 'v0.3.0',
    commit: COMMIT,
    archive: { size: gz.length, read: readArchive(gz) },
    license: 'MIT',
    binaries: [],
    releaseAssets: null,
    existing: null,
    taken: [],
    latest: null,
    others: [],
    ...over,
  };
}

const plugin = (o: Parameters<typeof pluginFiles>[1] = {}) => pluginFiles(REPO, o);
const by = (results: CheckResult[], key: CheckKey) => results.find((r) => r.key === key)!;
const failed = (results: CheckResult[]) => results.filter((r) => r.status === 'fail').map((r) => r.key);

describe('a valid plugin', () => {
  it('passes the eleven checks, in their order, and may be published', () => {
    const results = runChecks(snapshot(plugin()));
    expect(results.map((r) => r.key)).toEqual([...CHECK_KEYS]);
    expect(results.filter((r) => r.status !== 'pass')).toEqual([]);
    expect(verdict(results)).toBe('ready');
    expect(by(results, 'version').detail.fr).toBe('0.3.0 dans le manifeste, comme l’étiquette ; la première');
    expect(by(results, 'permissions').detail.fr).toBe('Se connecte à ntfy.sh ; reçoit des événements');
    expect(by(results, 'archive').detail.en).toMatch(/^0\.0 MB, 4 files, no link nor path that leaves$/);
    expect(by(results, 'binaries').detail.en).toBe('none: the plugin runs no program');
    expect(by(results, 'compile').detail.en).toBe('nothing to compile: the plugin is not run.js');
  });

  it('with its icon, a square PNG of 512 px of its repository', () => {
    const results = runChecks(snapshot(plugin({ manifest: { icon: './store/icon.png' }, files: { 'store/icon.png': png(512, 512) } })));
    expect(results.filter((r) => r.status !== 'pass')).toEqual([]);
  });

  it('with a program built for every system, attested from its commit', () => {
    const run = { command: 'bin/ntfy', windows: 'bin/ntfy.exe' };
    const bin = (platform: FetchedBinary['platform']): FetchedBinary => ({
      platform,
      name: `ntfy-0.3.0-${platform}.tar.gz`,
      sha256: 'f'.repeat(64),
      size: 10,
      attestation: { state: 'attested', workflow: '.github/workflows/release.yml' },
    });
    const results = runChecks(
      snapshot(plugin({ manifest: { run } }), {
        binaries: [bin('windows-x64'), bin('macos-arm64'), bin('linux-x64')],
        releaseAssets: ['ntfy-0.3.0-windows-x64.tar.gz', 'ntfy-0.3.0-macos-arm64.tar.gz', 'ntfy-0.3.0-linux-x64.tar.gz'],
      }),
    );
    expect(failed(results)).toEqual([]);
    expect(by(results, 'binaries').detail.fr).toBe(
      'windows-x64, macos-arm64, linux-x64 · attestés par le workflow .github/workflows/release.yml',
    );
    expect(by(results, 'permissions').detail.fr).toMatch(/^Lance un programme ; se connecte à ntfy.sh/);
  });

  it('with a later version of its own, from the same repository', () => {
    const results = runChecks(snapshot(plugin(), { existing: { mine: true, repository: REPO }, latest: '0.2.4', taken: ['0.2.4'] }));
    expect(failed(results)).toEqual([]);
    expect(by(results, 'version').detail.en).toBe('0.3.0 in the manifest, as the tag; greater than 0.2.4');
    expect(by(results, 'id').detail.fr).toBe('io.github.sroux.escouade-ntfy est à toi');
  });
});

describe('a booby-trapped plugin', () => {
  const cases: [string, CheckKey, () => Snapshot, string][] = [
    ['without a manifest', 'manifest', () => snapshot(plugin({ files: { 'escouade-plugin.json': null } })), 'escouade-plugin.json'],
    [
      'with a manifest that is no JSON',
      'manifest',
      () => snapshot(plugin({ files: { 'escouade-plugin.json': '{ nope' } })),
      'escouade-plugin.json',
    ],
    ['with a field wrong', 'manifest', () => snapshot(plugin({ manifest: { escouade: { api: 'one', min: '2.1.0' } } })), 'escouade.api'],
    ['whose version is not its tag’s', 'version', () => snapshot(plugin({ manifest: { version: '0.3.1' } })), 'version'],
    [
      'whose version is not past the last',
      'version',
      () => snapshot(plugin(), { existing: { mine: true, repository: REPO }, latest: '0.4.0' }),
      'version',
    ],
    [
      'whose number is taken',
      'version',
      () => snapshot(plugin(), { existing: { mine: true, repository: REPO }, taken: ['0.3.0'] }),
      'version',
    ],
    ['with an id of Escouade’s', 'id', () => snapshot(plugin({ manifest: { id: 'dev.escouade.ntfy' } })), 'id'],
    ['with an id of another author', 'id', () => snapshot(plugin(), { existing: { mine: false, repository: 'other/ntfy' } }), 'id'],
    [
      'naming another repository',
      'repository',
      () => snapshot(plugin({ manifest: { repository: 'https://github.com/evil/ntfy' } })),
      'repository',
    ],
    ['naming no repository', 'repository', () => snapshot(plugin({ manifest: { repository: undefined } })), 'repository'],
    ['whose license is not its file’s', 'license', () => snapshot(plugin(), { license: 'Apache-2.0' }), 'license'],
    [
      'whose license is not free',
      'license',
      () => snapshot(plugin({ manifest: { license: 'BUSL-1.1' } }), { license: 'BUSL-1.1' }),
      'license',
    ],
    ['without a license file', 'license', () => snapshot(plugin({ files: { LICENSE: null } }), { license: null }), 'LICENSE'],
    ['without a README', 'readme', () => snapshot(plugin({ files: { 'README.md': null } })), 'README.md'],
    ['with an empty README', 'readme', () => snapshot(plugin({ files: { 'README.md': ' \n' } })), 'README.md'],
    ['with an icon missing', 'manifest', () => snapshot(plugin({ manifest: { icon: 'icon.png' } })), 'icon'],
    [
      'with an icon not square',
      'manifest',
      () => snapshot(plugin({ manifest: { icon: 'icon.png' }, files: { 'icon.png': png(300, 256) } })),
      'icon',
    ],
    [
      'with an icon too small',
      'manifest',
      () => snapshot(plugin({ manifest: { icon: 'icon.png' }, files: { 'icon.png': png(128, 128) } })),
      'icon',
    ],
    [
      'with an icon too heavy',
      'manifest',
      () => snapshot(plugin({ manifest: { icon: 'icon.png' }, files: { 'icon.png': png(800, 800, { noise: true }) } })),
      'icon',
    ],
    [
      'with an icon that is a JPEG named .png',
      'manifest',
      () => snapshot(plugin({ manifest: { icon: 'icon.png' }, files: { 'icon.png': 'ÿØÿà JFIF' } })),
      'icon',
    ],
    [
      'with an SVG icon',
      'manifest',
      () => snapshot(plugin({ manifest: { icon: 'icon.svg' }, files: { 'icon.svg': '<svg><script>alert(1)</script></svg>' } })),
      'icon',
    ],
    ['with an icon out of the repository', 'manifest', () => snapshot(plugin({ manifest: { icon: '../icon.png' } })), 'icon'],
    ['with a remote icon', 'manifest', () => snapshot(plugin({ manifest: { icon: 'https://exemple.dev/icon.png' } })), 'icon'],
    [
      'with an icon too large to decode',
      'manifest',
      () => {
        const big = png(1, 1);
        big.writeUInt32BE(5000, 16);
        big.writeUInt32BE(5000, 20);
        return snapshot(plugin({ manifest: { icon: 'icon.png' }, files: { 'icon.png': big } }));
      },
      'icon',
    ],
    [
      'with an icon whose header reads but which does not decode',
      'manifest',
      () => snapshot(plugin({ manifest: { icon: 'icon.png' }, files: { 'icon.png': png(512, 512) } }), { iconUndecodable: true }),
      'icon',
    ],
    ['with a screenshot missing', 'screenshots', () => snapshot(plugin({ files: { 'store/1.png': null } })), 'store.screenshots[0]'],
    [
      'with a GIF named .png',
      'screenshots',
      () => snapshot(plugin({ files: { 'store/1.png': 'GIF89a\x01\x00\x01\x00' } })),
      'store.screenshots[0]',
    ],
    [
      'with a screenshot too wide',
      'screenshots',
      () => snapshot(plugin({ files: { 'store/1.png': png(2000, 10) } })),
      'store.screenshots[0]',
    ],
    [
      'with a screenshot too heavy',
      'screenshots',
      () => snapshot(plugin({ files: { 'store/1.png': png(620, 600, { noise: true }) } })),
      'store.screenshots[0]',
    ],
    [
      'with a symbolic link',
      'archive',
      () => snapshot(plugin({ files: { 'docs/key': { symlink: '/home/me/.ssh/id_ed25519' } } })),
      'docs/key',
    ],
    [
      'with a path that climbs out',
      'archive',
      () => snapshot(plugin(), {}, { 'owner-repo-0000000/../../evil.sh': 'x' }),
      'owner-repo-0000000/../../evil.sh',
    ],
    ['with an absolute path', 'archive', () => snapshot(plugin(), {}, { '/etc/cron.d/evil': 'x' }), '/etc/cron.d/evil'],
    [
      'with a submodule',
      'archive',
      () => snapshot(plugin({ files: { '.gitmodules': '[submodule "lib"]\n\tpath = lib\n' } })),
      '.gitmodules',
    ],
    [
      'with a Git LFS file not resolved',
      'archive',
      () => snapshot(plugin({ files: { 'bin/data.bin': 'version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 12\n' } })),
      'bin/data.bin',
    ],
    [
      'asking for the network without a program or webhook',
      'permissions',
      () => snapshot(plugin({ manifest: { contributes: { themes: [{ id: 'dark' }] } } })),
      'permissions.network[0]',
    ],
    [
      'with a secret it does not declare',
      'permissions',
      () => snapshot(plugin({ manifest: { permissions: { secrets: ['token'] } } })),
      'permissions.secrets[0]',
    ],
    [
      'naming a host as an address',
      'permissions',
      () => snapshot(plugin({ manifest: { permissions: { network: ['https://ntfy.sh/x'] } } })),
      'permissions.network[0]',
    ],
    [
      'whose program is nowhere',
      'binaries',
      () => snapshot(plugin({ manifest: { run: { command: 'bin/ntfy' } } }), { releaseAssets: ['notes.txt'] }),
      'run.command',
    ],
    [
      'whose binary was built from another commit',
      'binaries',
      () =>
        snapshot(plugin({ manifest: { run: { command: 'bin/ntfy' } }, files: { 'bin/ntfy': 'x' } }), {
          binaries: [
            {
              platform: 'linux-x64',
              name: 'ntfy-linux-x64.tar.gz',
              sha256: 'f'.repeat(64),
              size: 1,
              attestation: { state: 'mismatch', commit: 'b'.repeat(40), repository: 'sroux/escouade-ntfy' },
            },
          ],
        }),
      'ntfy-linux-x64.tar.gz',
    ],
  ];

  for (const [name, key, make, path] of cases)
    it(`${name}: “${key}” fails, at ${path}`, () => {
      const results = runChecks(make());
      const r = by(results, key);
      expect(r.status, JSON.stringify(r)).toBe('fail');
      expect(r.problems.map((p) => p.path)).toContain(path);
      for (const p of r.problems) for (const t of [p.why, p.fix]) expect(t.fr && t.en).toBeTruthy();
      expect(verdict(results)).toBe('rejected');
    });

  it('a bomb fails the archive, and the checks that read files wait', () => {
    const results = runChecks(snapshot({ ...plugin(), zeros: Buffer.alloc(220 * 1024 * 1024) }));
    expect(by(results, 'archive')).toMatchObject({
      status: 'fail',
      problems: [{ why: { en: 'The archive unpacks to more than 200 MB.' } }],
    });
    expect(by(results, 'manifest').status).toBe('skip');
    expect(verdict(results)).toBe('rejected');
  }, 30_000);

  it('a name too close to another author’s holds it for a person, without failing', () => {
    const results = runChecks(
      snapshot(plugin({ manifest: { id: 'io.github.q7x.slak-notify', name: 'Slak notify' } }), {
        others: [{ id: 'dev.lmartin.slack-notify', name: 'Slack notify' }],
      }),
    );
    expect(by(results, 'id').status).toBe('hold');
    expect(failed(results)).toEqual([]);
    expect(verdict(results)).toBe('held');
  });

  it('a rejection says each failure, where, why and how to fix it, with the documentation', () => {
    const results = runChecks(snapshot(plugin(), { license: 'Apache-2.0' }));
    expect(rejectionText(results, 'fr')).toBe(
      [
        '✗ Licence — Le manifeste dit MIT, le fichier LICENSE est sous Apache-2.0.',
        '  license : Le manifeste dit MIT, le fichier LICENSE est sous Apache-2.0.',
        '  → `license` doit nommer la licence du fichier, en identifiant SPDX : mets les deux d’accord, puis pousse une nouvelle étiquette.',
        '',
        'La documentation : https://escouade.dev/docs/plugins/publishing',
      ].join('\n'),
    );
    expect(rejectionText(results, 'en')).toContain('The documentation: https://escouade.dev/en/docs/plugins/publishing');
  });
});

describe('the binaries and their attestations', () => {
  it('reads the platform from a release file’s name', () => {
    expect(platformOf('ntfy-0.3.0-windows-x64.zip')).toBe('windows-x64');
    expect(platformOf('ntfy_macos_arm64.tar.gz')).toBe('macos-arm64');
    expect(platformOf('ntfy-linux-x64')).toBe('linux-x64');
    expect(platformOf('ntfy-0.3.0.tar.gz')).toBeNull();
    expect(platformOf('ntfy-darwinx-x64.zip')).toBeNull();
  });

  it('ties a binary to the commit and repository its attestation names', () => {
    const want = { sha256: 'e'.repeat(64), repository: REPO, commit: COMMIT };
    expect(judgeAttestations([attestation(want.sha256, { repo: REPO, commit: COMMIT })], want)).toEqual({
      state: 'attested',
      workflow: '.github/workflows/release.yml',
    });
    expect(judgeAttestations([attestation(want.sha256, { repo: 'evil/fork', commit: COMMIT })], want)).toEqual({
      state: 'mismatch',
      commit: COMMIT,
      repository: 'evil/fork',
    });
    expect(judgeAttestations([attestation('0'.repeat(64), { repo: REPO, commit: COMMIT })], want)).toEqual({ state: 'none' });
    expect(
      judgeAttestations([{ bundle: { dsseEnvelope: { payload: 'not json', payloadType: 'application/vnd.in-toto+json' } } }], want),
    ).toEqual({
      state: 'none',
    });
    expect(judgeAttestations([], want)).toEqual({ state: 'none' });
  });
});

describe('the names too close', () => {
  it('catches a letter changed, dashes moved, and lets distinct names be', () => {
    const others = [
      { id: 'dev.lmartin.slack', name: 'Slack' },
      { id: 'dev.sroux.ntfy', name: 'Ntfy' },
    ];
    expect(closeNames('io.github.x.slak', 'Slak', others).map((o) => o.id)).toEqual(['dev.lmartin.slack']);
    expect(closeNames('io.github.x.s-lack', 'S-lack', others).map((o) => o.id)).toEqual(['dev.lmartin.slack']);
    expect(closeNames('io.github.x.teams', 'Teams', others)).toEqual([]);
    expect(closeNames('io.github.x.git', 'Git', [{ id: 'dev.y.gif', name: 'Gif' }])).toEqual([]);
  });

  it('does not count the MIT text of the fixtures as a README', () => {
    const results = runChecks(snapshot({ ...plugin({ files: { 'README.md': null } }), 'docs/README.md': MIT }));
    expect(by(results, 'readme').status).toBe('fail');
  });
});

describe('the ids of Escouade’s own hand (ESC-314)', () => {
  const own = () => plugin({ manifest: { id: 'dev.escouade.ntfy' } });

  it('pass from the app’s repository, the manifest and the id checks alike', () => {
    const results = runChecks(snapshot(own(), { firstParty: true }));
    expect(by(results, 'id').status).toBe('pass');
    expect(by(results, 'manifest').status).toBe('pass');
  });

  it('are refused to any other repository, as before', () => {
    const results = runChecks(snapshot(own(), { firstParty: false }));
    expect(by(results, 'id')).toMatchObject({ status: 'fail', detail: { en: 'dev.escouade.ntfy is reserved' } });
    expect(verdict(results)).toBe('rejected');
  });

  it('take the app’s repository by its GitHub id, from STORE_FIRST_PARTY_REPO or its default', async () => {
    const asked: string[] = [];
    const source = (repos: Record<string, number>) =>
      ({
        repo: async (name: string) => {
          asked.push(name);
          if (!(name in repos)) throw new GithubError('Not Found', 404);
          return { id: repos[name]!, fullName: name, private: false };
        },
      }) as unknown as Source;
    const app = source({ 'guillaume-gagnaire/escouade': 1001 });
    expect(await isFirstParty(app, {}, 1001)).toBe(true);
    // Another repository, whatever its name (a fork, or one that took the app's old name after a rename): its id is not.
    expect(await isFirstParty(app, {}, 2002)).toBe(false);
    expect(await isFirstParty(source({ 'escouade-dev/escouade': 3003 }), { STORE_FIRST_PARTY_REPO: ' escouade-dev/escouade ' }, 3003)).toBe(
      true,
    );
    expect(asked).toEqual(['guillaume-gagnaire/escouade', 'guillaume-gagnaire/escouade', 'escouade-dev/escouade']);
    // A name GitHub does not know is no repository of the app's; GitHub down throws, the check is tried again.
    expect(await isFirstParty(source({}), {}, 1001)).toBe(false);
    const down = {
      repo: async () => {
        throw new GithubError('Bad Gateway', 502);
      },
    } as unknown as Source;
    await expect(isFirstParty(down, {}, 1001)).rejects.toThrow('Bad Gateway');
  });
});

describe('the run.js plugins and the programs of a third party (ESC-323)', () => {
  const BUNDLE = 'console.log(1);\n';
  const js = (over: Partial<Snapshot> = {}, files: Record<string, unknown> = { 'dist/plugin.mjs': BUNDLE }) =>
    snapshot(plugin({ manifest: { run: { js: 'dist/plugin.mjs' } }, files }), over);
  const compiler = { javy: '9.1.0', engine: 'e'.repeat(64), prelude: '2.2.0' };

  it('a run.js plugin brings no binary, and passes once compiled twice to the same module', () => {
    const results = runChecks(js({ compiled: { state: 'built', sha256: 'a'.repeat(64), size: 1_500_000, compiler } }));
    expect(results.filter((r) => r.status !== 'pass')).toEqual([]);
    expect(by(results, 'binaries').detail.fr).toBe('aucun à fournir : un plugin run.js est compilé en WASM par le store');
    expect(by(results, 'permissions').detail.fr).toBe('Tourne dans Escouade, isolé ; se connecte à ntfy.sh ; reçoit des événements');
    expect(by(results, 'compile').detail.fr).toBe(
      'compilé en WASM par le store, deux fois, même empreinte · 1465 Ko · sha256 aaaaaaaaaaaa… · Javy 9.1.0',
    );
    expect(verdict(results)).toBe('ready');
  });

  it('is not published before it is compiled', () => {
    const results = runChecks(js());
    expect(by(results, 'compile').status).toBe('skip');
    expect(verdict(results)).toBe('rejected');
  });

  it('two modules that differ refuse the version: the compilation is not reproducible', () => {
    const results = runChecks(js({ compiled: { state: 'different', first: '1'.repeat(64), second: '2'.repeat(64) } }));
    const c = by(results, 'compile');
    expect(c).toMatchObject({
      status: 'fail',
      detail: { fr: 'la compilation n’est pas reproductible', en: 'the compilation is not reproducible' },
    });
    expect(c.problems[0]!.why.fr).toMatch(/^La compilation n’est pas reproductible : .*111111111111… et 222222222222…/);
    expect(verdict(results)).toBe('rejected');
    expect(rejectionText(results, 'en')).toContain('✗ Compilation — the compilation is not reproducible');
  });

  it('a bundle Javy refuses, a bundle missing or too heavy fail the compilation, where and why', () => {
    const refused = by(runChecks(js({ compiled: { state: 'refused', message: 'import of node:net refused' } })), 'compile');
    expect(refused.problems[0]).toMatchObject({ path: 'dist/plugin.mjs' });
    expect(refused.problems[0]!.why.en).toBe('`dist/plugin.mjs` does not compile to WASM: import of node:net refused');
    const missing = by(runChecks(js({}, {})), 'compile');
    expect(missing.problems[0]).toMatchObject({ path: 'run.js' });
    expect(missing.problems[0]!.fix.fr).toContain('npm install');
    const heavy = by(runChecks(js({}, { 'dist/plugin.mjs': 'x'.repeat(5 * 1024 * 1024 + 1) })), 'compile');
    expect(heavy.problems[0]!.why.en).toBe('`dist/plugin.mjs` weighs more than 5 MB.');
  });

  const third = (command: Record<string, string>, files: Record<string, unknown> = {}, over: Partial<Snapshot> = {}) =>
    runChecks(snapshot(plugin({ manifest: { run: command }, files }), over));

  it.each([
    ['node', { command: 'node' }, {}, 'names an interpreter (node)'],
    ['npx.exe on Windows', { command: 'bin/ntfy', windows: 'npx.exe' }, { 'bin/ntfy': 'x' }, 'names an interpreter (npx)'],
    ['python3', { command: 'python3' }, {}, 'names an interpreter (python3)'],
    ['a .cmd', { command: 'bin/ntfy.cmd' }, { 'bin/ntfy.cmd': '@node %~dp0\\x.mjs' }, 'is a script'],
    ['a .ps1', { command: 'start.ps1' }, { 'start.ps1': 'node x.mjs' }, 'is a script'],
    ['a #! script', { command: 'bin/ntfy' }, { 'bin/ntfy': '#!/usr/bin/env node\nimport "./x.mjs";\n' }, 'is a script (it starts with'],
  ])('a third party whose run is %s is refused, with the reason in French and in English', (_, run, files, why) => {
    const results = third(run, files);
    const b = by(results, 'binaries');
    expect(b.status).toBe('fail');
    const p = b.problems.find((x) => x.why.en.includes(why))!;
    expect(p, JSON.stringify(b.problems)).toBeTruthy();
    expect(p.path).toMatch(/^run\.(command|windows)$/);
    expect(p.why.fr).toContain('un programme en JS/TS se publie en run.js, le store le compile en WASM');
    expect(p.why.en).toContain('a JS/TS program is published as run.js, the store compiles it to WASM');
    expect(p.fix.fr).toContain('"run": { "js": "dist/plugin.mjs" }');
    expect(verdict(results)).toBe('rejected');
  });

  it('a program of its own still passes, in the repository or from the release', () => {
    expect(by(third({ command: 'bin/ntfy' }, { 'bin/ntfy': 'ELF' }), 'binaries').status).toBe('pass');
  });

  it('a plugin of Escouade’s own hand from the app’s repository keeps its programs', () => {
    const own = plugin({
      manifest: { id: 'dev.escouade.ntfy', run: { command: 'bin/ntfy', windows: 'ntfy.cmd' } },
      files: { 'bin/ntfy': '#!/usr/bin/env node', 'ntfy.cmd': '@node x' },
    });
    expect(by(runChecks(snapshot(own, { firstParty: true })), 'binaries').status).toBe('pass');
    expect(by(runChecks(snapshot(own, { firstParty: false })), 'binaries').status).toBe('fail');
  });
});
