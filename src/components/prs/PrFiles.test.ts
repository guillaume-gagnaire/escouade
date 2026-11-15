import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { LARGE_LINES, viewedKey } from '../../lib/prfiles';
import { app } from '../../lib/state.svelte';
import type { PrContents, PrContentsFile, UiEvent } from '../../lib/types';
import { agent, fakeBackend, project, pullRequest as pr, resetApp } from '../../test/ipc';
import PrView from './PrView.svelte';

// The tab « Fichiers » of a pull request's view (ESC-467): its tree, filtered; the files folded and « Afficher »; « Vu »
// kept by pull request and by commit, gone once a commit changes the file; one commit's files; « Ouvrir dans l'éditeur ».

const PR = pr({ number: 214, title: 'Connexion par lien magique', head: 'ticket/atl-42', base: 'main', author: 'bob' });

const LINK: PrContentsFile = {
  path: 'src/auth/magic-link.ts',
  status: 'modified',
  additions: 2,
  deletions: 1,
  lines: [
    { kind: '@', text: '@@ -1,3 +1,4 @@' },
    { kind: ' ', old: 1, new: 1, text: 'export const ttl = 10;' },
    { kind: '-', old: 2, text: 'return mailer.send(email);' },
    { kind: '+', new: 2, text: 'await mailer.send(email, { locale });' },
    { kind: '+', new: 3, text: 'return { sent: true };' },
    { kind: ' ', old: 3, new: 4, text: '}' },
  ],
  binary: false,
  generated: false,
  cut: false,
};
const SESSION: PrContentsFile = {
  path: 'src/auth/session.ts',
  status: 'added',
  additions: 1,
  deletions: 0,
  lines: [
    { kind: '@', text: '@@ -0,0 +1 @@' },
    { kind: '+', new: 1, text: 'export const session = 1;' },
  ],
  binary: false,
  generated: false,
  cut: false,
};
const LOCK: PrContentsFile = { ...SESSION, path: 'package-lock.json', status: 'modified', generated: true };
const LOGO: PrContentsFile = { ...SESSION, path: 'public/logo.png', status: 'added', binary: true, lines: [], additions: 0 };
const BIG: PrContentsFile = {
  ...SESSION,
  path: 'docs/big.md',
  additions: LARGE_LINES + 10,
  lines: [
    { kind: '@', text: '@@' },
    ...Array.from({ length: LARGE_LINES + 10 }, (_, i) => ({ kind: '+' as const, new: i + 1, text: `ligne ${i + 1}` })),
  ],
};

const contents = (files: PrContentsFile[], over: Partial<PrContents> = {}): PrContents => ({
  thirdParty: true,
  word: 'PR',
  pr: PR,
  commits: [
    { sha: 'a1b2c3d4e5', message: 'feat: lien magique', author: 'bob' },
    { sha: '9e8f7a6b5c', message: 'fix: la session', author: 'bob' },
  ],
  files,
  threads: [],
  reading: { description: false, commits: true, files: true, threads: false },
  diff: true,
  tip: '9e8f7a6b5c',
  readAt: 0,
  checkedAt: 0,
  ...over,
});

const emit = (e: UiEvent) => (app as unknown as { onEvent: (e: UiEvent) => void }).onEvent(e);

async function openFiles() {
  const main = await screen.findByRole('main', { name: 'La PR #214' });
  const tab = await within(main).findByRole('tab', { name: /^Fichiers/ });
  await userEvent.click(tab);
  return main;
}

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [project()] });
});

describe('the tab « Fichiers » (ESC-467)', () => {
  it('shows the tree and the diffs, filters them, folds the generated, binary and large files until « Afficher »', async () => {
    fakeBackend({ pr_contents: () => ({ data: contents([LINK, SESSION, LOCK, LOGO, BIG]) }), pr_detail: () => ({ data: null }) });
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    expect(within(main).getByRole('tab', { name: 'Fichiers, 0/5 vus' })).toHaveAttribute('aria-selected', 'true');
    expect(main).toHaveTextContent('0/5 vus');

    const tree = within(main).getByRole('navigation', { name: 'Fichiers modifiés' });
    const names = within(tree)
      .getAllByRole('button')
      .map((b) => b.textContent?.replace(/\s+/g, ' ').trim());
    expect(names[0]).toMatch(/^docs/);
    expect(names).toContainEqual(expect.stringMatching(/^src\/auth/));
    expect(names).toContainEqual(expect.stringMatching(/^A ?session\.ts/));

    // The diff, git's: its lines, its numbers.
    const link = within(main).getByRole('region', { name: LINK.path });
    expect(within(link).getByRole('table', { name: `Diff de ${LINK.path}` })).toHaveTextContent('await mailer.send(email, { locale });');
    expect(link).toHaveTextContent('+2−1');

    // Folded: the generated, the binary, the large; « Afficher » unfolds.
    const lock = within(main).getByRole('region', { name: 'package-lock.json' });
    expect(lock).toHaveTextContent('Fichier généré, replié (1 ligne).');
    expect(lock).toHaveTextContent('généré');
    expect(within(main).getByRole('region', { name: 'public/logo.png' })).toHaveTextContent('Fichier binaire');
    expect(within(within(main).getByRole('region', { name: 'public/logo.png' })).queryByRole('button', { name: 'Afficher' })).toBeNull();
    const big = within(main).getByRole('region', { name: 'docs/big.md' });
    expect(big).toHaveTextContent(`Gros fichier, replié (${LARGE_LINES + 10} lignes).`);
    await userEvent.click(within(lock).getByRole('button', { name: 'Afficher' }));
    expect(lock).toHaveTextContent('export const session = 1;');

    // The filter: the tree and the diffs.
    await userEvent.type(within(tree).getByRole('searchbox', { name: 'Filtrer les fichiers' }), 'auth');
    expect(
      within(main)
        .getAllByRole('region')
        .map((r) => r.getAttribute('aria-label')),
    ).toEqual([LINK.path, SESSION.path]);
    await userEvent.type(within(tree).getByRole('searchbox'), ' rien');
    expect(main).toHaveTextContent('Aucun fichier ne correspond.');
  });

  it('keeps « Vu » by pull request: a file marked folds, and comes back unseen once a commit changes it', async () => {
    let files = [LINK, SESSION];
    fakeBackend({ pr_contents: () => ({ data: contents(files) }), pr_detail: () => ({ data: null }) });
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const link = within(main).getByRole('region', { name: LINK.path });
    await userEvent.click(within(link).getByRole('checkbox', { name: `${LINK.path} vu` }));
    await userEvent.click(within(within(main).getByRole('region', { name: SESSION.path })).getByRole('checkbox'));
    expect(within(main).getByRole('tab', { name: 'Fichiers, 2/2 vus' })).toBeInTheDocument();
    // Marked « Vu »: folded, kept in the interface's state.
    expect(within(link).queryByRole('table')).toBeNull();
    expect(Object.keys(app.ui.prViewed?.[viewedKey('p1', undefined, 214)]?.pr ?? {})).toEqual([LINK.path, SESSION.path]);

    // A new commit changes magic-link.ts: read again, it is no longer « Vu »; session.ts is.
    files = [{ ...LINK, additions: 3, lines: [...LINK.lines, { kind: '+', new: 5, text: 'export const done = true;' }] }, SESSION];
    emit({ type: 'prChanged', projectId: 'p1', number: 214 });
    await waitFor(() => expect(within(main).getByRole('tab', { name: 'Fichiers, 1/2 vus' })).toBeInTheDocument());
    const again = within(main).getByRole('region', { name: LINK.path });
    expect(within(again).getByRole('checkbox')).not.toBeChecked();
    expect(within(again).getByRole('table')).toHaveTextContent('export const done = true;');
    expect(within(within(main).getByRole('region', { name: SESSION.path })).getByRole('checkbox')).toBeChecked();
  });

  it('shows one commit’s files, from the bar or the tab « Commits », with its own « Vu »', async () => {
    const backend = fakeBackend({
      pr_contents: () => ({ data: contents([LINK, SESSION]) }),
      pr_detail: () => ({ data: null }),
      pr_commit_files: () => ({ data: [SESSION] }),
    });
    render(PrView, { project: project(), number: 214 });
    const main = await screen.findByRole('main', { name: 'La PR #214' });
    await userEvent.click(await within(main).findByRole('tab', { name: 'Commits 2' }));
    await userEvent.click(within(main).getByRole('button', { name: 'Voir les fichiers du commit 9e8f7a6' }));
    expect(within(main).getByRole('tab', { name: /^Fichiers/ })).toHaveAttribute('aria-selected', 'true');
    await waitFor(() =>
      expect(
        within(main)
          .getAllByRole('region')
          .map((r) => r.getAttribute('aria-label')),
      ).toEqual([SESSION.path]),
    );
    expect(backend.called('pr_commit_files').map((c) => c.args)).toEqual([{ projectId: 'p1', number: 214, sha: '9e8f7a6b5c' }]);
    expect(within(main).getByRole('combobox', { name: 'Commits montrés' })).toHaveValue('9e8f7a6b5c');
    // « Vu » in the commit: the pull request's count does not move.
    await userEvent.click(within(within(main).getByRole('region', { name: SESSION.path })).getByRole('checkbox'));
    expect(main).toHaveTextContent('1/1 vus');
    expect(within(main).getByRole('tab', { name: 'Fichiers, 0/2 vus' })).toBeInTheDocument();
    // Back to the whole pull request.
    await userEvent.selectOptions(within(main).getByRole('combobox', { name: 'Commits montrés' }), 'pr');
    expect(within(main).getAllByRole('region')).toHaveLength(2);
    expect(main).toHaveTextContent('0/2 vus');
  });

  it('opens a file, and a line, in the editor: the worktree of the agent that holds the branch, else the project', async () => {
    const open = vi.spyOn(app, 'openEditor').mockResolvedValue();
    fakeBackend({
      pr_contents: () => ({ data: contents([LINK, { ...SESSION, path: 'gone.ts', status: 'removed' }]) }),
      pr_detail: () => ({ data: null }),
    });
    const { unmount } = render(PrView, { project: project(), number: 214 });
    let main = await openFiles();
    const link = within(main).getByRole('region', { name: LINK.path });
    await userEvent.click(within(link).getByRole('button', { name: `Ouvrir ${LINK.path} à la ligne 3 dans l’éditeur` }));
    expect(open).toHaveBeenLastCalledWith({ projectId: 'p1', source: 'project', path: LINK.path, line: 3 });
    await userEvent.click(within(link).getByRole('button', { name: `Ouvrir ${LINK.path} dans l’éditeur` }));
    expect(open).toHaveBeenLastCalledWith({ projectId: 'p1', source: 'project', path: LINK.path, line: 2 });
    // A removed file has nothing to open.
    expect(within(within(main).getByRole('region', { name: 'gone.ts' })).queryByRole('button', { name: /dans l’éditeur/ })).toBeNull();
    unmount();

    // The agent that holds it, in its worktree.
    resetApp({
      projects: [project()],
      agents: [agent({ id: 'a1', name: 'Lien magique', prs: [PR], worktree: { path: 'C:/wt/a1', branch: 'ticket/atl-42' } as never })],
    });
    fakeBackend({ pr_contents: () => ({ data: contents([LINK]) }), pr_detail: () => ({ data: null }) });
    render(PrView, { project: project(), number: 214 });
    main = await openFiles();
    await userEvent.click(within(main).getByRole('button', { name: `Ouvrir ${LINK.path} à la ligne 1 dans l’éditeur` }));
    expect(open).toHaveBeenLastCalledWith({ projectId: 'p1', source: 'a1', path: LINK.path, line: 1 });
  });

  it('says why there are no lines: git could not read them from this project', async () => {
    fakeBackend({
      pr_contents: () => ({ data: contents([], { diff: true, diffFailure: 'Les lignes de x/y ne se lisent pas depuis ce projet.' }) }),
      pr_detail: () => ({ data: null }),
    });
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    expect(main).toHaveTextContent('Les lignes de la PR ne se lisent pas par git depuis ce projet.');
    expect(main).toHaveTextContent('Les lignes de x/y');
  });
});
