import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// The conversation framed at the first agent's station has an interface and logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
  openAgentTerminal: async () => ({ id: 't1', projectId: '', name: 'x', shell: 'claude' }),
  getXTerm: () => undefined,
  roomOf: () => null,
  mountTerminal() {},
  disposeTerminal() {},
  openTerminal: () => Promise.resolve({ id: 't', projectId: '', name: '', shell: '' }),
}));

import { dropConversation } from '../../lib/conversations.svelte';
import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import { onboarding, takeView } from '../../lib/onboarding.svelte';
import { app } from '../../lib/state.svelte';
import type { Agent, ConvItem, OnboardingStep, OnboardingView, Project, StepStatus } from '../../lib/types';
import { agent, CLAUDE_ENGINE, fakeBackend, project, resetApp } from '../../test/ipc';
import DemoBand from './DemoBand.svelte';
import Onboarding from './Onboarding.svelte';

const STEPS: OnboardingStep[] = ['harness', 'account', 'defaults', 'project', 'agent', 'telemetry'];
const DEMO = 'C:\\Users\\ada\\.escouade\\demo\\bac-a-sable';

/** The core's view, at `at`: the stations before it done. */
function view(at: OnboardingStep): OnboardingView {
  const i = STEPS.indexOf(at);
  const steps = STEPS.map((id, k) => ({ id, status: (k < i ? 'done' : 'todo') as StepStatus }));
  return { open: true, trigger: 'resume', steps, resumeAt: at, started: true, finished: false, askTelemetryAgain: false, demo: DEMO };
}

const PRINCIPAL = { id: 'principal', name: 'Principal', engine: 'claude' as const, configDir: '', claudePath: '', active: true };
const WT = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\w1', branch: 'escouade/w1', baseBranch: 'main' };

/** The backend of the stations: the journey kept, the rest as each test says. */
function core(at: OnboardingStep, extra: Record<string, (a: any) => unknown> = {}) {
  let v = view(at);
  return fakeBackend({
    onboarding_view: () => v,
    onboarding_act: ({ act }) => {
      if (act.kind === 'step') v = { ...v, steps: v.steps.map((s) => (s.id === act.step ? { ...s, status: act.outcome } : s)) };
      return v;
    },
    engines: () => app.engines.map((e) => ({ ...e, located: null, version: '2.1.301', state: 'ready', note: null })),
    ...extra,
  });
}

beforeEach(() => {
  resetApp({ projects: [] });
  app.settings = { ...app.settings, accounts: [PRINCIPAL], agentMode: 'acceptEdits', telemetry: null };
  app.engines = [CLAUDE_ENGINE];
  app.enginesFound = ['claude'];
  onboarding.project = null;
  onboarding.engine = null;
  setLocks(undefined);
});
afterEach(() => {
  onboarding.shown = false;
  dropConversation('a1');
  setLang('fr');
});

describe('the first project (ESC-505)', () => {
  const made = (over: Partial<Project> = {}) => project({ id: 'p9', name: 'atlas-web', path: 'C:\\dev\\atlas-web', ...over });

  it('says what a project, an agent, a worktree and a branch are, and recommends a folder of the machine', async () => {
    core('project');
    takeView(view('project'));
    render(Onboarding);
    expect(screen.getByRole('heading', { level: 1, name: 'Où travaille ton premier agent ?' })).toBeInTheDocument();
    const words = document.querySelector('dl.words')!;
    expect(words).toHaveTextContent('Un projet : un dossier, avec ses agents, son Kanban et ses docs.');
    expect(words).toHaveTextContent('Un agent : une conversation avec ton harness, qui lit et modifie le code.');
    expect(words).toHaveTextContent('Un worktree : une copie de travail à l’agent seul : deux agents ne se gênent pas.');
    expect(words).toHaveTextContent('Une branche par agent : son travail reste à part jusqu’à ce que tu l’intègres.');
    const radios = screen.getAllByRole('radio');
    expect(radios.map((r) => r.textContent?.trim().split('\n')[0])).toHaveLength(3);
    expect(screen.getByRole('radio', { name: /Un dossier de ta machine/ })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: /Un dossier de ta machine/ })).toHaveTextContent('Recommandé');
    expect(screen.getByRole('radio', { name: /Cloner un dépôt/ })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /Un projet de démonstration/ })).toBeInTheDocument();
    // Nothing opened before a folder is given.
    expect(screen.getByRole('button', { name: /Ouvrir le projet/ })).toBeDisabled();
  });

  it('refuses a folder Escouade cannot write to, offers a first commit to one with no git, says its .escouade.yml, then opens it', async () => {
    const backend = core('project', {
      inspect_folder: ({ path }) => ({
        exists: true,
        isRepo: false,
        branch: '',
        dirty: 0,
        name: 'x',
        repos: [],
        reposWithoutCommit: [],
        history: null,
        path,
      }),
      onboarding_folder: ({ path }) => ({ exists: true, writable: !path.includes('lecture'), tests: false }),
      shared_preview: ({ path }) =>
        path.includes('atlas') ? { shared: { file: { keys: ['board.merge_strategy', 'launch.commands'] } } } : null,
      create_project: () => made(),
      first_commit: () => null,
      create_agent: () => null,
    });
    takeView(view('project'));
    render(Onboarding);
    const field = screen.getByRole('textbox', { name: 'Le dossier du projet' });
    await userEvent.type(field, 'C:\\lecture-seule');
    expect(
      await screen.findByText('Escouade ne peut pas écrire dans ce dossier (C:\\lecture-seule) : choisis-en un autre.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Ouvrir le projet/ })).toBeDisabled();
    await userEvent.clear(field);
    await userEvent.type(field, 'C:\\dev\\atlas-web');
    expect(
      await screen.findByText('Ce dossier n’est pas un dépôt git : Escouade peut en faire un, avec un premier commit.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'Faire un premier commit' })).toBeChecked();
    expect(screen.getByTestId('onboarding-yml')).toHaveTextContent(
      'Ce dépôt a un .escouade.yml : il règle board.merge_strategy, launch.commands. Ses commandes se relisent avant de tourner.',
    );
    await userEvent.click(screen.getByRole('button', { name: /Ouvrir le projet/ }));
    await waitFor(() => expect(onboarding.at).toBe('agent'));
    expect(backend.called('create_project')[0].args).toMatchObject({
      path: 'C:\\dev\\atlas-web',
      name: 'atlas-web',
      worktreePerAgent: true,
    });
    expect(backend.called('first_commit')[0].args).toEqual({ path: 'C:\\dev\\atlas-web' });
    expect(onboarding.project).toBe('p9');
  });

  it('clones by an address: a clone that fails says what to do and lets try again; offline, it says so', async () => {
    let tries = 0;
    const backend = core('project', {
      onboarding_clone: () => {
        tries += 1;
        if (tries === 1) return { path: null, failure: { kind: 'access', detail: 'remote: Repository not found.' } };
        if (tries === 2) return { path: null, failure: { kind: 'offline', detail: 'Could not resolve host: github.com' } };
        return { path: 'C:\\dev\\api', failure: null };
      },
      create_project: () => made({ name: 'api', path: 'C:\\dev\\api' }),
      create_agent: () => null,
    });
    takeView(view('project'));
    render(Onboarding);
    await userEvent.click(screen.getByRole('radio', { name: /Cloner un dépôt/ }));
    await userEvent.type(screen.getByRole('combobox', { name: 'Adresse du dépôt' }), 'https://github.com/acme/api.git');
    expect(screen.getByRole('textbox', { name: 'Nom du dossier' })).toHaveValue('api');
    await userEvent.clear(screen.getByRole('textbox', { name: 'Dans le dossier' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Dans le dossier' }), 'C:\\dev');
    await userEvent.click(screen.getByRole('button', { name: /Cloner et ouvrir/ }));
    expect(await screen.findByTestId('onboarding-clone-failed')).toHaveTextContent(
      'Le clonage a échoué : remote: Repository not found. Vérifie l’adresse et ton accès, puis réessaie.',
    );
    await userEvent.click(screen.getByRole('button', { name: /Réessayer/ }));
    await waitFor(() =>
      expect(screen.getByTestId('onboarding-clone-failed')).toHaveTextContent(
        'Cloner demande le réseau. Choisis un dossier ou la démonstration, ou réessaie une fois en ligne.',
      ),
    );
    await userEvent.click(screen.getByRole('button', { name: /Réessayer/ }));
    await waitFor(() => expect(onboarding.at).toBe('agent'));
    expect(backend.called('onboarding_clone')[0].args).toEqual({
      url: 'https://github.com/acme/api.git',
      parent: 'C:\\dev',
      folder: 'api',
    });
    expect(backend.called('create_project')[0].args).toMatchObject({ path: 'C:\\dev\\api' });
  });

  it('makes the demonstration project, saying where and that nothing leaves the machine', async () => {
    const backend = core('project', { demo_create: () => made({ name: 'bac-a-sable', path: DEMO }), create_agent: () => null });
    takeView(view('project'));
    render(Onboarding);
    await userEvent.click(screen.getByRole('radio', { name: /Un projet de démonstration/ }));
    expect(
      screen.getByText(/une liste de tâches en TypeScript, deux tests qui échouent exprès. Rien ne sort de ta machine./),
    ).toHaveTextContent('~/.escouade/demo/bac-a-sable');
    await userEvent.click(screen.getByRole('button', { name: /Créer la démonstration/ }));
    await waitFor(() => expect(onboarding.at).toBe('agent'));
    expect(backend.called('demo_create')).toHaveLength(1);
    expect(app.projects.map((p) => p.name)).toEqual(['bac-a-sable']);
  });
});

describe('the first agent (ESC-505)', () => {
  function setup(a: Partial<Agent>, items: ConvItem[] = [], extra: Record<string, (a: any) => unknown> = {}, p: Partial<Project> = {}) {
    const proj = project({ worktreePerAgent: true, ...p });
    resetApp({
      projects: [proj],
      agents: [
        agent({
          id: 'a1',
          name: 'tests-qui-cassent',
          status: 'idle',
          prompts: 0,
          mode: 'acceptEdits',
          pending: [],
          requests: [],
          touchedFiles: [],
          worktree: WT,
          ...a,
        }),
      ],
    });
    app.settings = { ...app.settings, accounts: [PRINCIPAL], agentMode: 'acceptEdits' };
    const backend = core('agent', {
      get_conversation: () => items,
      onboarding_folder: () => ({ exists: true, writable: true, tests: false }),
      ...extra,
    });
    takeView(view('agent'));
    render(Onboarding);
    return backend;
  }
  const composer = () => within(screen.getByTestId('onboarding-frame')).getByRole('textbox');
  const user: ConvItem = { kind: 'user', id: 'u1', text: 'Explique-moi ce dépôt en cinq lignes.', images: 0, ts: 1, queued: false };
  const end = (over: object = {}) =>
    ({
      kind: 'turn',
      id: 't1',
      ts: 2,
      durationMs: 1,
      cost: 0,
      tokens: 1,
      isError: false,
      interrupted: false,
      error: null,
      ...over,
    }) as ConvItem;

  it('proposes a message in the real composer, changes it at a click, and sends nothing: the user’s click sends it', async () => {
    const backend = setup({}, [], { send_message: () => null });
    expect(screen.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeInTheDocument();
    expect(
      screen.getByText('Choisis-en un, modifie-le, puis envoie-le. Il part sur ton compte Claude Code et compte dans son quota.'),
    ).toBeInTheDocument();
    // No test command recognized: no « Lance les tests ».
    const starters = within(screen.getByRole('radiogroup', { name: 'Les messages proposés' })).getAllByRole('radio');
    expect(starters.map((r) => r.textContent?.trim())).toEqual([
      'Explique-moi ce dépôt en cinq lignes.',
      'Trouve un petit bug et propose un correctif.',
    ]);
    await waitFor(() => expect(composer()).toHaveValue('Explique-moi ce dépôt en cinq lignes.'));
    expect(screen.getByTestId('onboarding-agent-ready')).toHaveTextContent('tests-qui-cassent est prêt.');
    await userEvent.click(starters[1]);
    await waitFor(() => expect(composer()).toHaveValue('Trouve un petit bug et propose un correctif.'));
    expect(backend.called('send_message')).toHaveLength(0);
    expect(screen.getByRole('button', { name: /Continuer/ })).toBeDisabled();
    // Edited, then sent by its own button.
    await userEvent.type(composer(), ' Sois bref.');
    await userEvent.click(within(screen.getByTestId('onboarding-frame')).getByRole('button', { name: 'Envoyer' }));
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    expect(backend.called('send_message')[0].args).toMatchObject({
      id: 'a1',
      text: 'Trouve un petit bug et propose un correctif. Sois bref.',
    });
  });

  it('proposes « Lance les tests » for a project whose tests can run, first for the demonstration', async () => {
    setup({}, [], { onboarding_folder: () => ({ exists: true, writable: true, tests: true }) }, { path: DEMO });
    await waitFor(() => expect(composer()).toHaveValue('Lance les tests et dis-moi ce qui casse.'));
    const starters = within(screen.getByRole('radiogroup', { name: 'Les messages proposés' })).getAllByRole('radio');
    expect(starters).toHaveLength(3);
    expect(starters[0]).toHaveAttribute('aria-checked', 'true');
  });

  it('explains the first permission beside the real card: Autoriser, Toujours, Refuser', async () => {
    const perm = {
      kind: 'permission',
      id: 'p1',
      toolUseId: 'tu1',
      toolName: 'Bash',
      input: { command: 'npm test' },
      canAlways: true,
      defaultNo: false,
      decision: null,
      ts: 2,
    };
    setup({ status: 'waiting', prompts: 1, pending: ['p1'] }, [user, perm as unknown as ConvItem]);
    const h = await screen.findByRole('heading', { level: 1, name: 'Ton agent te demande la permission' });
    const why = document.getElementById('onb-why')!;
    expect(h).toHaveAttribute('aria-describedby', 'onb-why');
    expect(why).toHaveTextContent(
      'Il veut lancer une commande. En Édits auto, chaque commande te demande : Autoriser cette fois, Toujours pour cette commande dans ce projet, ou Refuser pour qu’il cherche autrement.',
    );
    expect([...why.querySelectorAll('b')].map((b) => b.textContent)).toEqual(['Autoriser', 'Toujours', 'Refuser']);
    expect(
      screen.getByText(
        'Au clavier : Ctrl+Entrée autorise, Ctrl+Maj+Entrée autorise toujours. Ctrl+J t’amène au prochain agent qui attend.',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/Ton message est parti sur ton compte Claude Code et compte dans son quota./)).toBeInTheDocument();
    // The real card, in the frame.
    expect(within(screen.getByTestId('onboarding-frame')).getByRole('button', { name: /^Autoriser/ })).toBeInTheDocument();
  });

  it('once done, shows the diff, where to find the agent and how to stop it, and lets go on', async () => {
    setup({ prompts: 1, touchedFiles: ['src/todo.ts'] }, [user, end()]);
    expect(await screen.findByRole('heading', { level: 1, name: 'Ton agent a répondu' })).toBeInTheDocument();
    expect(screen.getByText('L’agent a fini, dans son worktree, sur sa branche : ton dépôt n’a pas bougé.')).toBeInTheDocument();
    expect(screen.getByText('1 fichier changé')).toBeInTheDocument();
    expect(screen.getByText(/sous son nom « tests-qui-cassent »/)).toBeInTheDocument();
    expect(screen.getByText('Pour l’arrêter : Échap, ou ■ sous le message.')).toBeInTheDocument();
    await userEvent.click(screen.getByTestId('onboarding-diff'));
    expect(app.modal).toMatchObject({ kind: 'diff', agentId: 'a1', paths: [], title: 'Ce que tests-qui-cassent a changé' });
    app.modal = null;
    expect(screen.getByRole('button', { name: /Continuer/ })).toBeEnabled();
  });

  it('at the usage limit, says until when and offers another account', async () => {
    setup({ prompts: 1, resumeAt: new Date(2026, 10, 17, 18, 5).getTime() }, [user, end({ isError: true, limited: true })]);
    expect(await screen.findByRole('heading', { level: 1, name: 'Ce compte est à sa limite' })).toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Ce compte a atteint sa limite d’usage jusqu’à 18:05. Ajoute un autre compte, ou reviens après.',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter un compte…' }));
    expect(app.modal).toMatchObject({ kind: 'account', engine: 'claude', onboarding: true });
  });

  it('offline, keeps the message ready and puts it back', async () => {
    setup({ prompts: 1, status: 'error' }, [user, end({ isError: true, error: 'API Error: Connection error.' })]);
    expect(await screen.findByRole('heading', { level: 1, name: 'Hors ligne' })).toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Claude Code n’a pas pu joindre son service : tu sembles hors ligne. Le message reste prêt.',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Remettre le message' }));
    await waitFor(() => expect(composer()).toHaveValue('Explique-moi ce dépôt en cinq lignes.'));
  });

  it('not signed in, goes back to the account station', async () => {
    setup({ prompts: 1, status: 'error' }, [user, end({ isError: true, error: 'Invalid API key · Please run /login' })]);
    expect(await screen.findByRole('heading', { level: 1, name: 'Claude Code n’est pas connecté' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Revenir à l’étape du compte/ }));
    expect(onboarding.at).toBe('account');
  });

  it('with no account, asks for one first and makes no agent', async () => {
    resetApp({ projects: [project()] });
    app.settings = { ...app.settings, accounts: [] };
    const backend = core('agent');
    takeView(view('agent'));
    render(Onboarding);
    expect(screen.getByText('Ton premier agent a besoin d’un compte connecté : ajoute-le à la station du compte.')).toBeInTheDocument();
    expect(backend.called('create_agent')).toHaveLength(0);
    await userEvent.click(screen.getByRole('button', { name: /Revenir à l’étape du compte/ }));
    expect(onboarding.at).toBe('account');
  });

  it('is in English too', async () => {
    setLang('en');
    setup({});
    expect(screen.getByRole('heading', { level: 1, name: 'Your first message' })).toBeInTheDocument();
    await waitFor(() => expect(composer()).toHaveValue('Explain this repository in five lines.'));
  });
});

describe('the band of the demonstration (ESC-505)', () => {
  it('says what it is and where, and deletes it after a confirmation', async () => {
    const demo = project({ id: 'pd', name: 'bac-a-sable', path: DEMO });
    resetApp({ projects: [demo] });
    app.ui.activeProject = 'pd';
    const backend = fakeBackend({ demo_remove: () => null });
    takeView({ ...view('agent'), open: false });
    onboarding.shown = false;
    render(DemoBand);
    const band = screen.getByTestId('demo-band');
    expect(band).toHaveTextContent(
      'Bac à sable · un dépôt de démonstration sur ta machine (~/.escouade/demo/bac-a-sable) · rien ne sort d’ici',
    );
    await userEvent.click(within(band).getByRole('button', { name: 'Supprimer' }));
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Supprimer le projet de démonstration ?', danger: true });
    expect(backend.called('demo_remove')).toHaveLength(0);
    await (app.modal as { onConfirm: (o: boolean) => Promise<void> }).onConfirm(false);
    expect(backend.called('demo_remove')[0].args).toEqual({ id: 'pd' });
    expect(app.projects).toHaveLength(0);
  });

  it('is not shown on another project', () => {
    resetApp({ projects: [project()] });
    app.ui.activeProject = 'p1';
    fakeBackend();
    takeView({ ...view('agent'), open: false });
    onboarding.shown = false;
    render(DemoBand);
    expect(screen.queryByTestId('demo-band')).toBeNull();
  });
});
