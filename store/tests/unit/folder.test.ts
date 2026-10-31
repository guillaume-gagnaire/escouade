import { describe, expect, it } from 'vitest';
import { normalizePath, parseTag, tagFor, tagsOf } from '../../shared/folder';
import { folderOfUrl, runChecks, verdict, type CheckKey, type CheckResult, type Snapshot } from '../../server/utils/checks';
import { folderList, folderProblem, judgeFolder, linkOn, manifestFolders, packFolder, scope } from '../../server/utils/folder';
import { insidePath, renderDocs } from '../../server/utils/readme';
import { pluginsOfTag } from '../../server/utils/submissions';
import { readArchive, type ArchiveRead } from '../../server/utils/tar';
import { MIT, monorepoFiles, tarball } from '../fake/plugins.mjs';

// A plugin in a folder of its repository (ESC-313, docs/STORE.md §3.6): the folder typed and refused, the folders
// with a manifest, the tags of each folder, the archive of a folder alone, and the ten checks on two plugins of one
// repository, with the root's README and LICENSE standing in.

const REPO = 'sroux/escouade-plugins';
const COMMIT = 'b52e7f1'.padEnd(40, '0');
const by = (results: CheckResult[], key: CheckKey) => results.find((r) => r.key === key)!;
const notPassed = (results: CheckResult[]) => results.filter((r) => r.status !== 'pass').map((r) => r.key);

function repoRead(files = monorepoFiles(REPO)): ArchiveRead {
  return readArchive(tarball(files, { commit: COMMIT, top: `sroux-escouade-plugins-${COMMIT.slice(0, 7)}` }));
}

/** The snapshot of the folder `path` of the repository, as the worker builds it. */
function snapshot(path: string, tag: string, over: Partial<Snapshot> = {}, files = monorepoFiles(REPO)): Snapshot {
  const whole = repoRead(files);
  const pathLink = linkOn(whole, path);
  const read = pathLink ? { files: new Map(), dirs: new Set<string>(), problems: [], comment: null } : scope(whole, path);
  const gz = packFolder(read, 'top', COMMIT);
  return {
    repository: REPO,
    path,
    root: path ? whole : null,
    pathLink,
    tag,
    commit: COMMIT,
    archive: { size: gz.length, read },
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

describe('a folder typed by the author', () => {
  it('is normalized, the root by default', () => {
    expect(normalizePath(undefined)).toEqual({ path: '' });
    expect(normalizePath('')).toEqual({ path: '' });
    expect(normalizePath('.')).toEqual({ path: '' });
    expect(normalizePath(' ./plugins//slack/ ')).toEqual({ path: 'plugins/slack' });
  });

  it('is refused when absolute, climbing, or unreadable', () => {
    for (const p of ['/etc', '~/x', 'C:/x', 'c:\\x', '\\x']) expect(normalizePath(p)).toEqual({ problem: 'absolute' });
    for (const p of ['..', '../x', 'plugins/../..', 'a/../b']) expect(normalizePath(p)).toEqual({ problem: 'parent' });
    for (const p of ['a\\b', 'a\u0000b', 'x'.repeat(201), 42]) expect(normalizePath(p)).toEqual({ problem: 'bad' });
  });
});

describe('the tags of a repository with several plugins', () => {
  it('reads the three ways of naming a folder, and the root’s plain tags', () => {
    expect(parseTag('v1.2.3')).toEqual({ version: '1.2.3', folder: null, name: null });
    expect(parseTag('1.2.3-beta.1')).toEqual({ version: '1.2.3-beta.1', folder: null, name: null });
    expect(parseTag('plugins/slack/v1.2.3')).toEqual({ version: '1.2.3', folder: 'plugins/slack', name: null });
    expect(parseTag('slack@1.2.3')).toEqual({ version: '1.2.3', folder: null, name: 'slack' });
    expect(parseTag('my-slack-v1.2.3')).toEqual({ version: '1.2.3', folder: null, name: 'my-slack' });
    expect(parseTag('latest')).toBeNull();
  });

  it('gives each folder its tags, and none of another’s', () => {
    expect(tagFor('v1.2.3', '')).toBe('1.2.3');
    expect(tagFor('plugins/slack/v1.2.3', '')).toBeNull();
    expect(tagFor('plugins/slack/v1.2.3', 'plugins/slack')).toBe('1.2.3');
    expect(tagFor('slack@1.2.3', 'plugins/slack')).toBe('1.2.3');
    expect(tagFor('slack-v1.2.3', 'plugins/slack')).toBe('1.2.3');
    expect(tagFor('v1.2.3', 'plugins/slack')).toBeNull();
    expect(tagFor('plugins/ntfy/v1.2.3', 'plugins/slack')).toBeNull();
    expect(tagFor('ntfy@1.2.3', 'plugins/slack')).toBeNull();
    expect(tagsOf('0.1.0', 'plugins/slack')).toEqual(['plugins/slack/v0.1.0', 'slack@0.1.0', 'slack-v0.1.0']);
    expect(tagsOf('0.1.0', '')).toEqual(['v0.1.0', '0.1.0']);
  });

  it('ties a tag pushed to its plugin, and to none when two folders share a name', () => {
    const two = [{ path: 'plugins/ntfy' }, { path: 'plugins/slack' }];
    expect(pluginsOfTag(two, 'slack@0.3.0')).toEqual([{ path: 'plugins/slack' }]);
    expect(pluginsOfTag(two, 'plugins/ntfy/v0.4.0')).toEqual([{ path: 'plugins/ntfy' }]);
    expect(pluginsOfTag(two, 'v1.0.0')).toEqual([]);
    const same = [{ path: 'a/slack' }, { path: 'b/slack' }];
    expect(pluginsOfTag(same, 'slack@1.0.0')).toHaveLength(2);
    expect(pluginsOfTag(same, 'a/slack/v1.0.0')).toEqual([{ path: 'a/slack' }]);
    // A repository with its one plugin at the root takes every tag, as before: its checks say why.
    expect(pluginsOfTag([{ path: '' }], 'latest')).toEqual([{ path: '' }]);
  });
});

describe('the folders of an archive', () => {
  const read = repoRead();

  it('lists those that hold a manifest, not those that do not, nor the links', () => {
    expect(manifestFolders(read)).toEqual(['plugins/ntfy', 'plugins/slack']);
    expect(folderProblem(read, 'plugins/ntfy')).toBeNull();
    expect(folderProblem(read, 'docs')).toBe('no-manifest');
    expect(folderProblem(read, '')).toBe('no-manifest');
    expect(folderProblem(read, 'linked')).toBe('link');
    expect(folderProblem(read, 'escape/passwd')).toBe('link');
    const list = folderList(read);
    expect(judgeFolder(list, 'plugins/slack')).toBeNull();
    expect(judgeFolder(list, 'linked')).toBe('link');
    expect(judgeFolder(list, 'docs')).toBe('no-manifest');
  });

  it('takes a folder out alone: its files, its own links, never another folder’s', () => {
    const slack = scope(read, 'plugins/slack');
    expect([...slack.files.keys()].sort()).toEqual(['LICENSE', 'escouade-plugin.json', 'icon.png']);
    expect(slack.problems).toEqual([]);
    const inner = scope(readArchive(tarball({ 'p/escouade-plugin.json': '{}', 'p/key': { symlink: '/home/me/.ssh/id' } })), 'p');
    expect(inner.problems).toEqual([{ code: 'symlink', path: 'key' }]);
  });

  it('packs a folder as the store keeps it: the same bytes every time, read back as GitHub’s archives', () => {
    const slack = scope(read, 'plugins/slack');
    const a = packFolder(slack, 'sroux-escouade-plugins-b52e7f1', COMMIT);
    const b = packFolder(scope(repoRead(), 'plugins/slack'), 'sroux-escouade-plugins-b52e7f1', COMMIT);
    expect(a.equals(b)).toBe(true);
    const back = readArchive(a);
    expect(back.problems).toEqual([]);
    expect(back.comment).toBe(COMMIT);
    expect([...back.files.keys()].sort()).toEqual(['LICENSE', 'escouade-plugin.json', 'icon.png']);
    expect(back.files.get('LICENSE')!.toString()).toBe(MIT);
    // A long name goes through a pax header.
    const long = `${'d/'.repeat(60)}f.txt`;
    expect([...readArchive(packFolder({ ...slack, files: new Map([[long, Buffer.from('x')]]) }, 't', COMMIT)).files.keys()]).toEqual([
      long,
    ]);
  });
});

describe('two plugins of one repository', () => {
  it('each passes the ten checks from its own folder and tag', () => {
    const ntfy = runChecks(snapshot('plugins/ntfy', 'plugins/ntfy/v0.3.0'));
    expect(notPassed(ntfy)).toEqual([]);
    expect(verdict(ntfy)).toBe('ready');
    const slack = runChecks(snapshot('plugins/slack', 'slack@0.2.0'));
    expect(notPassed(slack)).toEqual([]);
    expect(runChecks(snapshot('plugins/slack', 'slack-v0.2.0')).filter((r) => r.status !== 'pass')).toEqual([]);
  });

  it('reads the README and the LICENSE in the folder, the root’s standing in', () => {
    const ntfy = runChecks(snapshot('plugins/ntfy', 'plugins/ntfy/v0.3.0'));
    expect(by(ntfy, 'readme').detail.fr).toBe('README.md');
    expect(by(ntfy, 'license').detail.fr).toBe('MIT, comme le fichier LICENSE de la racine');
    const slack = runChecks(snapshot('plugins/slack', 'slack@0.2.0'));
    expect(by(slack, 'readme').detail.en).toBe('README.md of the repository’s root (the folder has none)');
    expect(by(slack, 'license').detail.en).toBe('MIT, as the folder’s LICENSE file');
    // The icon is read in the folder: `./icon.png` of plugins/slack.
    expect(by(slack, 'manifest').status).toBe('pass');
  });

  it('refuses a folder’s own LICENSE GitHub cannot read, unless it says its SPDX id', () => {
    const files = { ...monorepoFiles(REPO), 'plugins/slack/LICENSE': 'Copyright 2026 Sam. Do what you like.\n' };
    const refused = by(runChecks(snapshot('plugins/slack', 'slack@0.2.0', {}, files)), 'license');
    expect(refused.status).toBe('fail');
    expect(refused.problems[0]!.fix.fr).toContain('SPDX-License-Identifier: MIT');
    const said = { ...files, 'plugins/slack/LICENSE': `SPDX-License-Identifier: MIT\n\n${MIT}` };
    expect(by(runChecks(snapshot('plugins/slack', 'slack@0.2.0', {}, said)), 'license').status).toBe('pass');
  });

  it('refuses a tag of another folder, and says which ones to push', () => {
    const r = by(runChecks(snapshot('plugins/slack', 'v0.2.0')), 'version');
    expect(r.status).toBe('fail');
    expect(r.problems[0]!.why.fr).toContain('`plugins/slack/v1.2.3`');
    expect(r.problems[0]!.fix.en).toBe('Push the tag `plugins/slack/v0.2.0`.');
    expect(by(runChecks(snapshot('plugins/slack', 'ntfy@0.2.0')), 'version').status).toBe('fail');
  });

  it('a folder moved by its own author passes, says so and goes to the review; another author’s is refused', () => {
    const moved = runChecks(
      snapshot('plugins/slack', 'slack@0.2.0', { existing: { mine: true, repository: REPO, path: 'slack' }, latest: '0.1.0' }),
    );
    const id = by(moved, 'id');
    expect(id.status).toBe('pass');
    expect(id.detail.fr).toBe(
      'io.github.sroux.mono-slack est à toi ; sa source change, de sroux/escouade-plugins/slack à sroux/escouade-plugins/plugins/slack : noté à la publication, cette version repasse en revue',
    );
    const same = runChecks(
      snapshot('plugins/slack', 'slack@0.2.0', { existing: { mine: true, repository: REPO, path: 'plugins/slack' }, latest: '0.1.0' }),
    );
    expect(by(same, 'id').detail.en).toBe('io.github.sroux.mono-slack is yours');
    // From another repository, at its root: the source changes too.
    expect(
      by(runChecks(snapshot('plugins/slack', 'slack@0.2.0', { existing: { mine: true, repository: 'sroux/slack' } })), 'id').detail.en,
    ).toContain('from sroux/slack to sroux/escouade-plugins/plugins/slack');
    expect(
      by(runChecks(snapshot('plugins/slack', 'slack@0.2.0', { existing: { mine: false, repository: REPO, path: 'plugins/slack' } })), 'id')
        .status,
    ).toBe('fail');
  });
});

describe('the folder trapped', () => {
  it('a folder through a link fails the archive and the manifest, the link named', () => {
    const results = runChecks(snapshot('linked', 'linked/v0.3.0'));
    expect(by(results, 'archive').status).toBe('fail');
    expect(by(results, 'archive').problems[0]!.why.fr).toBe(
      'Le dossier `linked` passe par `linked`, un lien : le store ne suit pas de lien.',
    );
    expect(by(results, 'manifest').status).toBe('fail');
    expect(verdict(results)).toBe('rejected');
  });

  it('a folder without a manifest says where the store looked, in both languages', () => {
    const m = by(runChecks(snapshot('docs', 'docs/v0.3.0')), 'manifest');
    expect(m.status).toBe('fail');
    expect(m.problems[0]!.path).toBe('docs/escouade-plugin.json');
    expect(m.problems[0]!.why.fr).toBe(
      'Le store lit `escouade-plugin.json` dans le dossier `docs`, au commit de l’étiquette ; il n’y est pas.',
    );
    expect(m.problems[0]!.why.en).toBe(
      'The store reads `escouade-plugin.json` in the folder `docs`, at the tag’s commit; it is not there.',
    );
  });

  it('a link elsewhere in the repository does not fail a folder that does not go through it', () => {
    expect(by(runChecks(snapshot('plugins/ntfy', 'plugins/ntfy/v0.3.0')), 'archive').status).toBe('pass');
  });
});

describe('the manifest’s repository with a folder', () => {
  it('names the repository, or its folder by /tree/<ref>/<folder>', () => {
    expect(folderOfUrl(`https://github.com/${REPO}`, 'plugins/slack')).toBeNull();
    expect(folderOfUrl(`https://github.com/${REPO}/tree/main/plugins/slack`, 'plugins/slack')).toBe('plugins/slack');
    expect(folderOfUrl(`https://github.com/${REPO}/tree/plugins/slack/v1.0.0/plugins/slack`, 'plugins/slack')).toBe('plugins/slack');
    expect(folderOfUrl(`https://github.com/${REPO}/tree/main/plugins/ntfy`, 'plugins/slack')).toBe('plugins/ntfy');
    expect(folderOfUrl(`https://github.com/${REPO}/tree/main`, '')).toBe('');
  });

  it('passes the repository’s URL or the folder’s, and fails another folder’s', () => {
    const at = (repository: string, path = 'plugins/slack') => {
      const files = monorepoFiles(REPO);
      const m = JSON.parse(String(files[`${path}/escouade-plugin.json`]));
      return by(
        runChecks(snapshot(path, 'slack@0.2.0', {}, { ...files, [`${path}/escouade-plugin.json`]: JSON.stringify({ ...m, repository }) })),
        'repository',
      );
    };
    expect(at(`https://github.com/${REPO}`).status).toBe('pass');
    expect(at(`https://github.com/${REPO}/tree/main/plugins/slack`).status).toBe('pass');
    const other = at(`https://github.com/${REPO}/tree/main/plugins/ntfy`);
    expect(other.status).toBe('fail');
    expect(other.problems[0]!.why.fr).toBe(
      'Le manifeste nomme le dossier `plugins/ntfy` du dépôt, la soumission vient du dossier `plugins/slack`.',
    );
    expect(at(`https://github.com/someone/else/tree/main/plugins/slack`).status).toBe('fail');
  });
});

describe('the README of a plugin in a folder', () => {
  it('links its files on GitHub inside the folder, and drops a path that leaves it', async () => {
    expect(insidePath('store/1.png', 'README.md', REPO, 'plugins/ntfy')).toBe('store/1.png');
    expect(insidePath('/plugins/ntfy/store/1.png', 'README.md', REPO, 'plugins/ntfy')).toBe('store/1.png');
    expect(insidePath(`https://github.com/${REPO}/blob/main/plugins/ntfy/store/1.png`, 'README.md', REPO, 'plugins/ntfy')).toBe(
      'store/1.png',
    );
    expect(insidePath('/LICENSE', 'README.md', REPO, 'plugins/ntfy')).toBeNull();
    const ntfy = scope(repoRead(), 'plugins/ntfy');
    const docs = await renderDocs(
      { ...ntfy, files: new Map([...ntfy.files, ['README.md', Buffer.from('[the guide](docs/a.md)')]]) },
      { repo: REPO, commit: COMMIT, path: 'plugins/ntfy', keep: async () => 'x.png' },
    );
    expect(docs.readme.en).toContain(`href="https://github.com/${REPO}/blob/${COMMIT}/plugins/ntfy/docs/a.md"`);
  });
});
