import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import { onboarding, takeView } from '../../lib/onboarding.svelte';
import { app } from '../../lib/state.svelte';
import { OS } from '../../lib/platform';
import type { Locks, OnboardingStep, OnboardingView, StepStatus } from '../../lib/types';
import { BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, resetApp } from '../../test/ipc';
import Onboarding from './Onboarding.svelte';

const STEPS: OnboardingStep[] = ['harness', 'account', 'defaults', 'project', 'agent', 'telemetry'];

/** The core's view: every station to do but those said. */
function view(over: Partial<Record<OnboardingStep, StepStatus>> = {}, trigger: OnboardingView['trigger'] = 'first'): OnboardingView {
  const steps = STEPS.map((id) => ({ id, status: over[id] ?? 'todo' }));
  return {
    open: true,
    trigger,
    steps,
    resumeAt: trigger === 'redo' ? 'harness' : (steps.find((s) => s.status === 'todo')?.id ?? null),
    started: true,
    finished: false,
    askTelemetryAgain: false,
    demo: 'C:/Users/ada/.escouade/demo/bac-a-sable',
  };
}

/** The stations of the line, as buttons. */
const stops = () =>
  within(screen.getByRole('navigation'))
    .getAllByRole('listitem')
    .map((li) => within(li).getByRole('button'));

const machine = (over: object = {}) => ({ found: true, added: false, connected: false, signsIn: true, email: null, dir: '', ...over });

/**
 * A core that keeps the journey as `onboarding.rs` does, enough for the window: a station done or passed, the end, and
 * the real state read again (an account, a mode, a project, an answer).
 */
function core(start: OnboardingView, extra: Record<string, (a: any) => unknown> = {}) {
  let v = start;
  const statusOf = (id: OnboardingStep): StepStatus => {
    const own = v.steps.find((s) => s.id === id)!.status;
    if (own === 'policy') return own;
    const real =
      id === 'harness' || id === 'account'
        ? (app.settings.accounts ?? []).length > 0
        : id === 'defaults'
          ? !!app.settings.agentMode
          : id === 'project'
            ? app.projects.length > 0
            : id === 'agent'
              ? Object.values(app.agents).some((a) => a.prompts > 0 && a.status === 'idle')
              : app.settings.telemetry !== null && app.settings.telemetry !== undefined;
    return real ? 'done' : own;
  };
  const read = () => {
    v = { ...v, steps: v.steps.map((s) => ({ id: s.id, status: statusOf(s.id) })) };
    return v;
  };
  return fakeBackend({
    onboarding_view: read,
    onboarding_act: ({ act }) => {
      if (act.kind === 'step') v = { ...v, steps: v.steps.map((s) => (s.id === act.step ? { ...s, status: act.outcome } : s)) };
      if (act.kind === 'skipAll')
        v = {
          ...v,
          finished: true,
          open: false,
          steps: v.steps.map((s) => ({ ...s, status: s.status === 'todo' ? 'skipped' : s.status })),
        };
      if (act.kind === 'finish') v = { ...v, finished: true, open: false };
      return read();
    },
    engines: () =>
      app.engines.map((e) => ({ ...e, located: null, version: e.id === 'claude' ? '2.1.301' : null, state: 'ready', note: null })),
    account_machine: ({ engine }) => machine({ connected: engine === 'claude', email: engine === 'claude' ? 'toi@exemple.dev' : null }),
    account_add_machine: ({ engine }) => ({
      id: engine === 'claude' ? 'principal' : `principal-${engine}`,
      name: 'Principal',
      engine,
      configDir: '',
      claudePath: '',
      active: true,
    }),
    engine_order_set: ({ order }) => order,
    ...extra,
  });
}

/** The gestures told to the core, but what Escouade found of the harnesses (the funnel's profile, ESC-507). */
const acts = (b: { called: (cmd: string) => { args: Record<string, any> }[] }) =>
  b
    .called('onboarding_act')
    .map((c) => c.args.act)
    .filter((a) => a.kind !== 'profile');

describe('the first launch (ESC-504)', () => {
  beforeEach(() => {
    resetApp({ projects: [] });
    app.settings = { ...app.settings, accounts: [], agentMode: '', telemetry: null };
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.enginesFound = ['claude'];
    setLocks(undefined);
  });
  afterEach(() => {
    onboarding.shown = false;
    setLang('fr');
  });

  it('draws the line of the stations, the one on screen marked as the current step, and its question focused', async () => {
    core(view());
    takeView(view());
    render(Onboarding);
    const line = screen.getByRole('navigation', { name: 'Les stations du premier lancement' });
    const items = stops();
    expect(items.map((b) => b.querySelector('.nm')?.textContent)).toEqual([
      'Harness',
      'Compte',
      'Réglages de départ',
      'Premier projet',
      'Premier agent',
      'Statistiques',
    ]);
    expect(items[0]).toHaveAttribute('aria-current', 'step');
    expect(items[0]).toHaveTextContent('maintenant');
    expect(screen.getByText('6 stations · environ 2 minutes')).toBeInTheDocument();
    const q = screen.getByRole('heading', { level: 1, name: 'Avec quel harness tes agents travaillent-ils ?' });
    await waitFor(() => expect(q).toHaveFocus());
    expect(q).toHaveAccessibleDescription(/Un harness est le programme d’agent de code de ta machine/);
    expect(screen.getByText('Station 1 · le harness')).toBeInTheDocument();
  });

  it('recommends the harness signed in on the machine, writes « Ordre des harness » and adds its account: the account station is folded in', async () => {
    const backend = core(view());
    takeView(view());
    render(Onboarding);
    const group = screen.getByRole('radiogroup', { name: 'Avec quel harness tes agents travaillent-ils ?' });
    const claude = await within(group).findByRole('radio', { name: /Claude Code/ });
    await waitFor(() => expect(claude).toHaveTextContent('Connecté · toi@exemple.dev'));
    expect(claude).toHaveAttribute('aria-checked', 'true');
    expect(claude).toHaveTextContent('Recommandé');
    expect(claude).toHaveTextContent('2.1.301');
    expect(screen.getByText(/Ses agents partent sur le compte déjà connecté/)).toBeInTheDocument();
    // The harness not found: not recommended, its command shown once chosen.
    const bare = within(group).getByRole('radio', { name: /Harness nu/ });
    expect(bare).toHaveTextContent('Introuvable');
    await userEvent.click(screen.getByRole('button', { name: /^Continuer/ }));
    await waitFor(() => expect(onboarding.at).toBe('defaults'));
    expect(backend.called('account_add_machine')[0].args).toEqual({ engine: 'claude' });
    expect(app.settings.accounts.map((a) => a.id)).toEqual(['principal']);
    expect(acts(backend)).toContainEqual({ kind: 'step', step: 'harness', outcome: 'done', choice: 'recommended' });
    // What Escouade found, told once for the funnel of the statistics (ESC-507).
    expect(
      backend
        .called('onboarding_act')
        .filter((c) => c.args.act.kind === 'profile')
        .map((c) => c.args.act),
    ).toEqual([{ kind: 'profile', profile: 'ready' }]);
  });

  it('with no harness found, shows the command of the one chosen, copied in one click, and waits', async () => {
    core(view());
    app.enginesFound = [];
    takeView(view());
    render(Onboarding);
    expect(await screen.findByText(/Aucun harness n’est installé/)).toBeInTheDocument();
    const claude = screen.getByRole('radio', { name: /Claude Code/ });
    expect(claude).toHaveAttribute('aria-checked', 'true');
    const cmd = OS === 'windows' ? 'irm https://claude.ai/install.ps1 | iex' : 'curl -fsSL https://claude.ai/install.sh | bash';
    expect(screen.getByText(cmd)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Copier la commande de Claude Code' }));
    expect(app.toasts.map((x) => x.text)).toContain('Commande copiée');
    expect(screen.getByRole('button', { name: /^Continuer/ })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Chercher à nouveau' })).toBeEnabled();
  });

  it('greys a harness a policy denies, with why, and never recommends it', async () => {
    core(view());
    app.enginesFound = ['claude', 'codex'];
    const locks: Locks = {
      locks: {
        'harnesses.allowed': {
          key: 'harnesses.allowed',
          value: null,
          by: null,
          bounds: [{ by: { kind: 'device', source: 'x' }, allowed: ['codex'] }],
        },
      },
      rows: {},
      warnings: [],
    } as unknown as Locks;
    setLocks(locks);
    takeView(view());
    render(Onboarding);
    const claude = await screen.findByRole('radio', { name: /Claude Code/ });
    expect(claude).toHaveAttribute('aria-disabled', 'true');
    expect(claude).toHaveTextContent('non permis par la stratégie de ce poste');
    expect(claude).not.toHaveTextContent('Recommandé');
    await waitFor(() => expect(screen.getByRole('radio', { name: /Harness nu/ })).toHaveAttribute('aria-checked', 'true'));
  });

  it('explains each mode, recommends Édits auto, shows Bypass off with why, and writes « Mode par défaut des agents »', async () => {
    const backend = core(view({ harness: 'done', account: 'done' }));
    app.settings = {
      ...app.settings,
      accounts: [{ id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true }],
    };
    takeView(view({ harness: 'done', account: 'done' }));
    render(Onboarding);
    expect(await screen.findByRole('heading', { name: 'Quand tes agents doivent-ils te demander ?' })).toBeInTheDocument();
    expect(screen.getByText(/Modèle Sonnet( [\d.]+)?, effort moyen : ceux de ton compte/)).toBeInTheDocument();
    const radios = screen.getAllByRole('radio');
    expect(radios.map((r) => r.querySelector('b')?.textContent)).toEqual(['Édits auto', 'Demander', 'Plan', 'Auto', 'Bypass']);
    expect(radios[0]).toHaveAttribute('aria-checked', 'true');
    expect(radios[0]).toHaveTextContent('chaque commande demande ton accord');
    expect(radios[4]).toHaveAttribute('aria-disabled', 'true');
    expect(radios[4]).toHaveTextContent('Éteint : l’agent ferait tout sans rien te montrer');
    // ↓ moves to « Demander », ↓ again past the others, never onto Bypass.
    radios[0].focus();
    await userEvent.keyboard('{ArrowDown}');
    expect(radios[1]).toHaveAttribute('aria-checked', 'true');
    expect(radios[1]).toHaveFocus();
    await userEvent.keyboard('{ArrowUp}{ArrowUp}');
    expect(radios[3]).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('button', { name: /Garder Auto/ }));
    await waitFor(() => expect(onboarding.at).toBe('project'));
    expect(backend.called('save_settings')[0].args.settings.agentMode).toBe('auto');
    expect(app.settings.agentMode).toBe('auto');
  });

  it('says a harness that runs only in Bypass (ESC-489)', async () => {
    core(view({ harness: 'done', account: 'done' }));
    app.engines = [{ ...BARE_ENGINE, caps: { ...BARE_ENGINE.caps, modes: [{ id: 'bypassPermissions', meaning: null }] } }];
    onboarding.engine = BARE_ENGINE.id;
    takeView(view({ harness: 'done', account: 'done' }));
    render(Onboarding);
    expect(await screen.findByText('Harness nu ne tourne qu’en Bypass : ses agents agiront sans demander.')).toBeInTheDocument();
    onboarding.engine = null;
  });

  it('shows a mode a policy imposes, locked, with who imposes it', async () => {
    core(view({ harness: 'done', account: 'done', defaults: 'policy' }));
    setLocks({
      locks: {
        'permissions.defaultMode': { key: 'permissions.defaultMode', value: 'plan', by: { kind: 'device', source: 'x' }, bounds: [] },
      },
      rows: { agentMode: ['permissions.defaultMode'] },
      warnings: [],
    } as unknown as Locks);
    app.settings = { ...app.settings, agentMode: 'plan' };
    takeView({ ...view({ harness: 'done', account: 'done', defaults: 'policy' }), resumeAt: 'defaults' });
    render(Onboarding);
    expect(await screen.findByText('Imposé par la stratégie de ce poste')).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /Plan/ })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: /Édits auto/ })).toHaveAttribute('aria-disabled', 'true');
    expect(stops()[2]).toHaveTextContent('maintenant');
  });

  it('passes a station with « Plus tard » or Échap, marks it « passé · à reprendre », and goes on', async () => {
    const backend = core(view());
    takeView(view());
    render(Onboarding);
    await screen.findByRole('radio', { name: /Claude Code/ });
    await userEvent.click(screen.getByRole('button', { name: 'Plus tard' }));
    await waitFor(() => expect(onboarding.at).toBe('account'));
    expect(stops()[0]).toHaveTextContent('passé · à reprendre');
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(onboarding.at).toBe('defaults'));
    expect(acts(backend)).toEqual([
      { kind: 'step', step: 'harness', outcome: 'skipped' },
      { kind: 'step', step: 'account', outcome: 'skipped' },
    ]);
  });

  it('goes on with Entrée from the question, and back with « Retour »', async () => {
    core(view({ harness: 'done', account: 'done' }));
    app.settings = {
      ...app.settings,
      accounts: [{ id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true }],
    };
    takeView(view({ harness: 'done', account: 'done' }));
    render(Onboarding);
    const q = await screen.findByRole('heading', { name: 'Quand tes agents doivent-ils te demander ?' });
    await waitFor(() => expect(q).toHaveFocus());
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(onboarding.at).toBe('project'));
    await userEvent.click(screen.getByRole('button', { name: 'Retour' }));
    expect(onboarding.at).toBe('defaults');
  });

  it('« Tout passer » passes every station not done and closes the journey', async () => {
    const backend = core(view());
    takeView(view());
    render(Onboarding);
    await userEvent.click(screen.getByRole('button', { name: 'Tout passer' }));
    await waitFor(() => expect(onboarding.shown).toBe(false));
    expect(acts(backend)).toEqual([{ kind: 'skipAll' }]);
  });

  it('asks the statistics as before, neither answer put forward, then ends', async () => {
    const backend = core(view({ harness: 'done', account: 'done', defaults: 'done', project: 'done', agent: 'done' }), {
      telemetry_set: () => null,
    });
    takeView(view({ harness: 'done', account: 'done', defaults: 'done', project: 'done', agent: 'done' }));
    render(Onboarding);
    expect(await screen.findByRole('heading', { name: 'Aider à améliorer Escouade ?' })).toBeInTheDocument();
    expect(screen.getByText(/restent sur ta machine, dans ~\/.escouade/)).toBeInTheDocument();
    const no = screen.getByRole('button', { name: 'Non merci' });
    const yes = screen.getByRole('button', { name: 'Partager les statistiques' });
    expect(no.className).toBe(yes.className);
    await userEvent.click(no);
    expect(backend.called('telemetry_set')[0].args).toEqual({ consent: false });
    expect(await screen.findByRole('heading', { name: 'Escouade est prêt' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Commencer/ }));
    expect(onboarding.shown).toBe(false);
  });

  it('opened again, shows every station from the first with what is set in place, and closes with « Fermer »', async () => {
    const backend = core(
      view({ harness: 'done', account: 'done', defaults: 'done', project: 'done', agent: 'done', telemetry: 'done' }, 'redo'),
    );
    app.settings = {
      ...app.settings,
      agentMode: 'plan',
      telemetry: true,
      accounts: [{ id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true }],
    };
    takeView(view({ harness: 'done', account: 'done', defaults: 'done', project: 'done', agent: 'done', telemetry: 'done' }, 'redo'));
    render(Onboarding);
    const items = stops();
    expect(items[0]).toHaveTextContent('maintenant');
    expect(items[1]).toHaveTextContent('Principal');
    expect(items[2]).toHaveTextContent('Plan');
    expect(items[5]).toHaveTextContent('Partager les statistiques');
    await userEvent.click(items[2]);
    expect(onboarding.at).toBe('defaults');
    expect(screen.getByRole('radio', { name: /Plan/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Fermer' }));
    await waitFor(() => expect(onboarding.shown).toBe(false));
    expect(acts(backend)).toEqual([{ kind: 'finish' }]);
  });

  it('is in English too', async () => {
    setLang('en');
    core(view());
    takeView(view());
    render(Onboarding);
    expect(screen.getByRole('heading', { name: 'Which harness will your agents run on?' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Skip all' })).toBeInTheDocument();
    expect(screen.getByText('6 stops · about 2 minutes')).toBeInTheDocument();
  });
});
