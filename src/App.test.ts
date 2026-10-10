import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { setLang } from './lib/i18n';
import { app } from './lib/state.svelte';
import type { Agent, InitialState, Project, UiEvent } from './lib/types';
import { agent, fakeBackend, project, resetApp, SETTINGS } from './test/ipc';

const DIFF = `diff --git a/src/auth.ts b/src/auth.ts
--- a/src/auth.ts
+++ b/src/auth.ts
@@ -1 +1 @@
-const b = 2;
+const b = 3;
`;

/** The backend's events, to the app started last. */
let channel: { onmessage: (e: UiEvent) => void } | null = null;
const emit = (e: UiEvent) => channel!.onmessage(e);

function start(
  layout: '' | 'split',
  over: { projects?: Project[]; agents?: Agent[]; handlers?: Record<string, (args: any) => unknown> } = {},
) {
  const initial: InitialState = {
    projects: over.projects ?? [project()],
    agents: over.agents ?? [agent()],
    ui: { activeProject: 'p1', view: 'project', selectedAgent: {}, layout },
    settings: SETTINGS,
    usage: { fiveHour: null, sevenDay: null, todayCost: 0, updatedAt: 0 },
    git: {},
    shells: [],
    terminals: [],
    tickets: [],
    claudeFound: true,
    version: '0.1.0',
    models: [],
  };
  fakeBackend({
    subscribe: (args: any) => {
      channel = args.channel;
      return initial;
    },
    get_conversation: () => [],
    git_files: () => [{ path: 'src/auth.ts', status: 'M', add: 1, del: 1, agentId: 'a1' }],
    git_diff: () => DIFF,
    git_branches: () => ['main'],
    ...over.handlers,
  });
  // Nothing rendered from a previous test's state until the snapshot is in.
  resetApp();
  app.ready = false;
  return render(App);
}

describe('App layout', () => {
  it('shows the uncommitted files and the diff next to the conversation in the split layout', async () => {
    start('split');
    expect(await screen.findByText('const b = 3;')).toBeInTheDocument();
    expect(screen.getByRole('main')).toBeInTheDocument();
  });

  it('opens the direct commit from the files panel when the project commits directly', async () => {
    start('split', {
      projects: [project({ commitMode: 'direct' })],
      handlers: {
        commit_preview: () => ({
          files: [{ path: 'src/auth.ts', status: 'M', add: 1, del: 1, agentId: 'a1', inWorktree: false }],
          leftOut: [],
        }),
        commit_propose: () => 'fix(auth): un jeton plus court',
      },
    });
    await screen.findByText('const b = 3;');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    const dialog = await screen.findByRole('dialog', { name: 'Commit' });
    const field = within(dialog).getByRole('textbox', { name: 'Message' });
    await waitFor(() => expect(field).toHaveValue('fix(auth): un jeton plus court'));
    expect(field).toHaveFocus();
  });

  it('asks before Escape drops the commit message written, and comes back to it, the message kept, on « Annuler »', async () => {
    let proposals = 0;
    start('split', {
      projects: [project({ commitMode: 'direct' })],
      handlers: {
        commit_preview: () => ({
          files: [{ path: 'src/auth.ts', status: 'M', add: 1, del: 1, agentId: 'a1', inWorktree: false }],
          leftOut: [],
        }),
        commit_propose: () => (proposals++, 'fix(auth): un jeton plus court'),
      },
    });
    await screen.findByText('const b = 3;');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    const field = () => within(screen.getByRole('dialog', { name: 'Commit' })).getByRole('textbox', { name: 'Message' });
    await waitFor(() => expect(field()).toHaveValue('fix(auth): un jeton plus court'));
    await userEvent.type(field(), ' (relu)');
    await userEvent.keyboard('{Escape}');
    const ask = await screen.findByRole('dialog', { name: 'Abandonner le message ?' });
    expect(screen.queryByRole('dialog', { name: 'Commit' })).not.toBeInTheDocument();
    await userEvent.click(within(ask).getByRole('button', { name: 'Annuler' }));
    await waitFor(() => expect(field()).toHaveValue('fix(auth): un jeton plus court (relu)'));
    expect(proposals).toBe(1);
    // Given up once agreed.
    await userEvent.keyboard('{Escape}');
    await userEvent.click(
      within(await screen.findByRole('dialog', { name: 'Abandonner le message ?' })).getByRole('button', { name: 'Abandonner' }),
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(app.modal).toBeNull();
  });

  it('comes back to the commit being written, its message kept, once quitting with files not saved is cancelled', async () => {
    start('split', {
      projects: [project({ commitMode: 'direct' })],
      handlers: {
        commit_preview: () => ({
          files: [{ path: 'src/auth.ts', status: 'M', add: 1, del: 1, agentId: 'a1', inWorktree: false }],
          leftOut: [],
        }),
        commit_propose: () => 'fix(auth): un jeton plus court',
      },
    });
    await screen.findByText('const b = 3;');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    const field = () => within(screen.getByRole('dialog', { name: 'Commit' })).getByRole('textbox', { name: 'Message' });
    await waitFor(() => expect(field()).toHaveValue('fix(auth): un jeton plus court'));
    await userEvent.type(field(), ' (relu)');
    emit({ type: 'quitRequested', unsaved: 1 });
    const ask = await screen.findByRole('dialog', { name: 'Quitter Escouade ?' });
    await userEvent.click(within(ask).getByRole('button', { name: 'Annuler' }));
    await waitFor(() => expect(field()).toHaveValue('fix(auth): un jeton plus court (relu)'));
  });

  it('does not bring back, once quitting is cancelled, a commit made while it asked, nor a choice already made', async () => {
    let done: (hash: string) => void = () => {};
    start('split', {
      projects: [project({ commitMode: 'direct' })],
      handlers: {
        commit_preview: () => ({
          files: [{ path: 'src/auth.ts', status: 'M', add: 1, del: 1, agentId: 'a1', inWorktree: false }],
          leftOut: [],
        }),
        commit_propose: () => 'fix(auth): un jeton plus court',
        commit_direct: () => new Promise<string>((r) => (done = r)),
      },
    });
    await screen.findByText('const b = 3;');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    const commit = await screen.findByRole('dialog', { name: 'Commit' });
    await waitFor(() => expect(within(commit).getByRole('textbox', { name: 'Message' })).toHaveValue('fix(auth): un jeton plus court'));
    await userEvent.click(within(commit).getByRole('button', { name: 'Commiter' }));
    emit({ type: 'quitRequested', unsaved: 1 });
    let ask = await screen.findByRole('dialog', { name: 'Quitter Escouade ?' });
    done('abc1234');
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'Commit abc1234 créé' }));
    await userEvent.click(within(ask).getByRole('button', { name: 'Annuler' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(app.modal).toBeNull();

    // A confirmation whose choice is still running.
    const onConfirm = vi.fn(() => new Promise<void>(() => {}));
    app.modal = { kind: 'confirm', title: 'Supprimer « a.ts » ?', body: 'Il part dans la corbeille.', confirm: 'Supprimer', onConfirm };
    await userEvent.click(await screen.findByRole('button', { name: 'Supprimer' }));
    emit({ type: 'quitRequested', unsaved: 1 });
    ask = await screen.findByRole('dialog', { name: 'Quitter Escouade ?' });
    await userEvent.click(within(ask).getByRole('button', { name: 'Annuler' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it('brings back, once quitting is cancelled, a commit whose commit git refused while it asked, its message kept', async () => {
    let refuse: (e: string) => void = () => {};
    start('split', {
      projects: [project({ commitMode: 'direct' })],
      handlers: {
        commit_preview: () => ({
          files: [{ path: 'src/auth.ts', status: 'M', add: 1, del: 1, agentId: 'a1', inWorktree: false }],
          leftOut: [],
        }),
        commit_propose: () => 'fix(auth): un jeton plus court',
        commit_direct: () => new Promise<string>((_, reject) => (refuse = reject)),
      },
    });
    await screen.findByText('const b = 3;');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    const field = () => within(screen.getByRole('dialog', { name: 'Commit' })).getByRole('textbox', { name: 'Message' });
    await waitFor(() => expect(field()).toHaveValue('fix(auth): un jeton plus court'));
    await userEvent.type(field(), ' (relu)');
    await userEvent.click(screen.getByRole('button', { name: 'Commiter' }));
    emit({ type: 'quitRequested', unsaved: 1 });
    const ask = await screen.findByRole('dialog', { name: 'Quitter Escouade ?' });
    refuse('lint en échec');
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'lint en échec', kind: 'error' }));
    await userEvent.click(within(ask).getByRole('button', { name: 'Annuler' }));
    await waitFor(() => expect(field()).toHaveValue('fix(auth): un jeton plus court (relu)'));
  });

  it('asks the next question of a confirmation in a dialog of its own, the focus in it', async () => {
    start('');
    await screen.findByRole('main');
    const onConfirm = vi.fn();
    app.modal = {
      kind: 'confirm',
      title: 'Enregistrer « a.ts » ?',
      body: 'b',
      confirm: 'Enregistrer',
      onConfirm: vi.fn(),
      alt: {
        label: 'Ne pas enregistrer',
        onClick: () => {
          app.modal = {
            kind: 'confirm',
            title: 'Supprimer « a.ts » ?',
            body: 'Il part dans la corbeille.',
            confirm: 'Supprimer',
            onConfirm,
          };
        },
      },
    };
    await userEvent.click(await screen.findByRole('button', { name: 'Ne pas enregistrer' }));
    const dialog = await screen.findByRole('dialog', { name: 'Supprimer « a.ts » ?' });
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
    await userEvent.keyboard('{Tab}');
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
    await userEvent.click(within(dialog).getByRole('button', { name: 'Supprimer' }));
    expect(onConfirm).toHaveBeenCalled();
  });

  it('keeps the files panel closed in the classic layout until asked for', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    expect(screen.queryByText('Non commités')).not.toBeInTheDocument();
  });

  it('shows the files of the agents’ worktrees in the diff of the whole project, as the list does', async () => {
    const diffOf = (path: string, line: string) =>
      `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n@@ -1 +1 @@\n-old\n+${line}\n`;
    const change = (path: string, agentId: string | null, inWorktree: boolean) => ({
      path,
      status: 'M',
      add: 1,
      del: 1,
      agentId,
      inWorktree,
    });
    const wt = { path: 'C:/wt', branch: 'escouade/a2', baseBranch: 'main' };
    start('', {
      agents: [agent(), agent({ id: 'a2', name: 'tests-e2e', worktree: wt })],
      handlers: {
        git_files: () => [change('README.md', null, false), change('src/wt.ts', 'a2', true)],
        // What « Voir le diff » used to read: the project's checkout only.
        git_diff: () => diffOf('README.md', 'racine'),
        git_project_diff: () => [
          { agentId: null, inWorktree: false, diff: diffOf('README.md', 'racine') },
          { agentId: 'a2', inWorktree: true, diff: diffOf('src/wt.ts', 'dans le worktree') },
        ],
      },
    });
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.filesScope = 'project';
    app.filesOpen = true;
    await screen.findByText('wt.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Voir le diff' }));
    const dialog = await screen.findByRole('dialog', { name: 'Modifications de demo-api' });
    const rows = await within(dialog).findAllByRole('button', { name: /^M / });
    expect(rows.map((r) => r.textContent)).toEqual([expect.stringMatching(/README\.md/), expect.stringMatching(/src\/wt\.ts\s*tests-e2e/)]);
    await userEvent.click(rows[1]);
    expect(within(dialog).getByText('dans le worktree')).toBeInTheDocument();
  });

  it('opens « Ouvrir un fichier » with Ctrl+P, the WebView’s own key (printing) held back, and opens the file picked', async () => {
    start('', {
      handlers: {
        fs_tree: () => ({ root: 'C:/code/demo-api', files: ['README.md', 'src/app.ts'], truncated: false, ignored: [] }),
        fs_read: () => ({ kind: 'text', text: 'x\n', size: 2, hash: 'h', eol: 'lf', bom: false }),
        fs_base: () => null,
      },
    });
    expect(await screen.findByRole('main')).toBeInTheDocument();
    const press = new KeyboardEvent('keydown', { key: 'p', ctrlKey: true, bubbles: true, cancelable: true });
    window.dispatchEvent(press);
    expect(press.defaultPrevented).toBe(true);
    const dialog = await screen.findByRole('dialog', { name: 'Ouvrir un fichier' });
    await userEvent.type(within(dialog).getByRole('combobox'), 'app');
    await within(dialog).findByRole('option', { name: /app/ });
    await userEvent.keyboard('{Enter}');
    expect(await screen.findByRole('tab', { name: /app\.ts/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.queryByRole('dialog', { name: 'Ouvrir un fichier' })).not.toBeInTheDocument();
  });

  describe('Ctrl+P and the print dialog of the WebView', () => {
    const FILES = {
      fs_tree: () => ({ root: 'C:/code/demo-api', files: ['README.md', 'src/app.ts'], truncated: false, ignored: [] }),
      fs_read: () => ({ kind: 'text', text: 'x', size: 1, hash: 'h', eol: 'lf', bom: false }),
      fs_base: () => null,
    };
    /** Ctrl+P (or Ctrl+Shift+P) dispatched from `target`: whether the page may still print. */
    function press(target: Element | Window, shift = false) {
      const e = new KeyboardEvent('keydown', { key: shift ? 'P' : 'p', ctrlKey: true, shiftKey: shift, bubbles: true, cancelable: true });
      target.dispatchEvent(e);
      return e.defaultPrevented;
    }

    it('is held back from a field that keeps every key to itself', async () => {
      start('');
      expect(await screen.findByRole('main')).toBeInTheDocument();
      const field = document.body.appendChild(document.createElement('input'));
      field.addEventListener('keydown', (e) => e.stopPropagation());
      expect(press(field)).toBe(true);
      expect(press(field, true)).toBe(true);
      // Ctrl+Shift+P is held back and opens nothing.
      expect(app.modal).toBeNull();
      field.remove();
    });

    it('is held back from the field naming a new file', async () => {
      start('', { handlers: FILES });
      expect(await screen.findByRole('main')).toBeInTheDocument();
      await app.openEditor({ source: 'project', path: 'README.md' });
      await userEvent.click(await screen.findByRole('button', { name: 'Nouveau fichier' }));
      const field = await screen.findByRole('textbox', { name: 'Nom du nouveau fichier' });
      expect(press(field)).toBe(true);
      expect(press(field, true)).toBe(true);
    });

    it('is held back from the field renaming an agent', async () => {
      const { container } = start('');
      expect(await screen.findByRole('main')).toBeInTheDocument();
      const card = container.querySelector('aside.side')!;
      await userEvent.dblClick(within(card as HTMLElement).getByText('refacto-auth'));
      const field = within(card as HTMLElement).getByDisplayValue('refacto-auth');
      expect(press(field)).toBe(true);
      expect(press(field, true)).toBe(true);
    });

    it('is held back from the code, where it also opens the palette', async () => {
      const { container } = start('', { handlers: FILES });
      expect(await screen.findByRole('main')).toBeInTheDocument();
      await app.openEditor({ source: 'project', path: 'README.md' });
      await waitFor(() => expect(container.querySelector('.cm-content')).not.toBeNull());
      const code = container.querySelector('.cm-content')!;
      expect(press(code, true)).toBe(true);
      expect(app.modal).toBeNull();
      expect(press(code)).toBe(true);
      expect(await screen.findByRole('dialog', { name: 'Ouvrir un fichier' })).toBeInTheDocument();
    });

    it('is held back from the palette itself, from the window, and without a project', async () => {
      start('');
      expect(await screen.findByRole('main')).toBeInTheDocument();
      expect(press(window)).toBe(true);
      const dialog = await screen.findByRole('dialog', { name: 'Ouvrir un fichier' });
      expect(press(within(dialog).getByRole('combobox'))).toBe(true);
      await fireEvent.keyDown(window, { key: 'Escape' });
      app.ui.activeProject = null;
      expect(press(window)).toBe(true);
    });
  });

  it('holds back the print dialog on Ctrl+P behind another dialog too, which stays', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.modal = { kind: 'newProject' };
    const press = new KeyboardEvent('keydown', { key: 'p', ctrlKey: true, bubbles: true, cancelable: true });
    window.dispatchEvent(press);
    expect(press.defaultPrevented).toBe(true);
    expect(app.modal).toEqual({ kind: 'newProject' });
  });

  it('shows the board of the project in place of its agent, and the agent again when asked for', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'À faire' })).not.toBeInTheDocument();
    app.openBoard('p1');
    expect(await screen.findByRole('region', { name: 'À faire' })).toBeInTheDocument();
    expect(screen.getAllByRole('main')).toHaveLength(1);
    app.closeBoard('p1');
    await expect.poll(() => screen.queryByRole('region', { name: 'À faire' })).toBeNull();
  });

  it('opens the board settings from the board header', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.openBoard('p1');
    await userEvent.click(await screen.findByRole('button', { name: /Après validation/ }));
    expect(await screen.findByRole('dialog', { name: 'Réglages' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Kanban' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toBeInTheDocument();
  });

  it('proposes the launch commands from the empty Lancement section: the settings open on them and Claude reads the project', async () => {
    let answer!: (v: unknown) => void;
    const reading = new Promise((r) => (answer = r));
    start('', { handlers: { suggest_run_commands: () => reading } });
    await userEvent.click(await screen.findByRole('button', { name: '✦ Proposer des commandes' }));
    const dialog = await screen.findByRole('dialog', { name: 'Réglages' });
    expect(within(dialog).getByRole('tab', { name: 'Projets' })).toHaveAttribute('aria-selected', 'true');
    expect(await within(dialog).findByRole('button', { name: 'Claude lit le projet…' })).toBeDisabled();
    answer({ commands: [{ id: 's1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: '' }], refused: 0 });
    // Read in full, then taken.
    const proposal = within(await within(dialog).findByRole('region', { name: 'Commandes de lancement proposées' }));
    expect(proposal.getByText('npm run dev')).toBeInTheDocument();
    await userEvent.click(proposal.getByRole('button', { name: 'Remplacer les commandes' }));
    expect(await within(dialog).findByDisplayValue('npm run dev')).toBeInTheDocument();
    expect(within(dialog).queryByRole('button', { name: 'Claude lit le projet…' })).not.toBeInTheDocument();
  });

  it('comes back to the settings, unsaved changes kept, from closing a project there', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.modal = { kind: 'settings' };
    await userEvent.click(await screen.findByRole('tab', { name: 'Réseau' }));
    await userEvent.type(screen.getByPlaceholderText('aucun'), 'http://proxy:3128');
    await userEvent.click(screen.getByRole('tab', { name: 'Projets' }));
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet…' }));
    await userEvent.click(await screen.findByRole('button', { name: 'Annuler' }));
    expect(await screen.findByRole('tab', { name: 'Projets' })).toHaveAttribute('aria-selected', 'true');
    await userEvent.click(screen.getByRole('tab', { name: 'Réseau' }));
    expect(screen.getByPlaceholderText('aucun')).toHaveValue('http://proxy:3128');
    // Closed for good: still the settings, still the draft.
    await userEvent.click(screen.getByRole('tab', { name: 'Projets' }));
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet…' }));
    await userEvent.click(await screen.findByRole('button', { name: 'Fermer le projet' }));
    expect(await screen.findByText('Aucun projet ouvert.')).toBeInTheDocument();
    expect(app.projects).toEqual([]);
    await userEvent.click(screen.getByRole('tab', { name: 'Réseau' }));
    expect(screen.getByPlaceholderText('aucun')).toHaveValue('http://proxy:3128');
  });

  it('opens « Rechercher dans les conversations » with Ctrl+K, and the message of the result chosen', async () => {
    const a2 = agent({ id: 'a2', name: 'pagination', createdAt: 2 });
    const said = (id: string, text: string) => ({ kind: 'user', id, text, images: 0, ts: 1, queued: false });
    start('', {
      agents: [agent(), a2],
      handlers: {
        get_conversation: ({ id }: { id: string }) =>
          id === 'a2' ? [said('u0', 'Bonjour'), said('u1', 'Ajoute la pagination'), said('u2', 'Merci')] : [],
        search_conversations: () => ({
          hits: [
            {
              agentId: 'a2',
              projectId: 'p1',
              agentName: 'pagination',
              archived: false,
              eventIndex: 0,
              itemId: 'u1',
              snippet: 'Ajoute la pagination',
              mark: [10, 20],
              at: Date.now(),
            },
          ],
          capped: false,
          timedOut: false,
        }),
      },
    });
    expect(await screen.findByRole('main')).toBeInTheDocument();
    await userEvent.keyboard('{Control>}k{/Control}');
    const field = await screen.findByRole('combobox', { name: 'Rechercher dans les conversations' });
    await userEvent.type(field, 'pagination');
    await screen.findByRole('option', { name: /Ajoute la pagination/ });
    await userEvent.keyboard('{Enter}');
    await expect.poll(() => screen.queryByRole('dialog', { name: 'Rechercher dans les conversations' })).toBeNull();
    expect(app.agent?.id).toBe('a2');
    await expect.poll(() => [...document.querySelectorAll<HTMLElement>('.found')].map((e) => e.dataset.item)).toEqual(['u1']);
    expect(document.querySelector('.found')).toHaveTextContent('Ajoute la pagination');
  });

  it('shows « Vue d’ensemble » in place of the project on Ctrl+Shift+A, and the agent again on Escape', async () => {
    start('', { agents: [agent(), agent({ id: 'b1', name: 'landing', status: 'waiting', createdAt: 2 })] });
    expect(await screen.findByRole('main')).toHaveClass('conv');
    await userEvent.keyboard('{Control>}{Shift>}A{/Shift}{/Control}');
    const view = await screen.findByRole('main', { name: 'Vue d’ensemble' });
    expect(screen.getAllByRole('main')).toEqual([view]);
    // Every project's agents: the sidebar of the one on screen goes too.
    expect(screen.queryByRole('button', { name: /Nouvel agent/ })).not.toBeInTheDocument();
    expect(within(view).getByRole('region', { name: 'Attend ta réponse' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    await expect.poll(() => screen.queryByRole('main', { name: 'Vue d’ensemble' })).toBeNull();
    expect(screen.getByRole('main')).toHaveClass('conv');
  });

  it('counts an agent as seen once the board that hid it is closed', async () => {
    const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(true);
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.openBoard('p1');
    await screen.findByRole('region', { name: 'À faire' });
    app.attention.a1 = true;
    app.closeBoard('p1');
    await expect.poll(() => app.attention.a1).toBeUndefined();
    focus.mockRestore();
  });
});

describe('App frame', () => {
  it('says why the app could not start', async () => {
    start('', { handlers: { subscribe: () => Promise.reject('connexion perdue') } });
    expect(await screen.findByText('Impossible de démarrer : connexion perdue')).toBeInTheDocument();
  });

  it('offers a new agent in a project that has none', async () => {
    start('', { agents: [] });
    expect(await screen.findByText('Aucun agent dans ce projet.')).toBeInTheDocument();
    expect(
      within(screen.getByText('Aucun agent dans ce projet.').parentElement!).getByRole('button', { name: '+ Nouvel agent' }),
    ).toBeInTheDocument();
  });

  it('writes all of it in English: why it could not start, no agent, and the question asked on quitting', async () => {
    setLang('en');
    const { unmount } = start('', { handlers: { subscribe: () => Promise.reject('connection lost') } });
    expect(await screen.findByText('Can’t start: connection lost')).toBeInTheDocument();
    unmount();

    start('', { agents: [] });
    expect(await screen.findByText('No agents in this project.')).toBeInTheDocument();
    expect(
      within(screen.getByText('No agents in this project.').parentElement!).getByRole('button', { name: '+ New agent' }),
    ).toBeInTheDocument();
    emit({ type: 'quitRequested', unsaved: 2 });
    const ask = await screen.findByRole('dialog', { name: 'Quit Escouade?' });
    expect(within(ask).getByText('2 files in the editor aren’t saved: their changes will be lost.')).toBeInTheDocument();
    expect(within(ask).getByRole('button', { name: 'Quit anyway' })).toBeInTheDocument();
    expect(within(ask).getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });
});
