import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { dictation } from '../../lib/dictation.svelte';
import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import { onboarding, passedHookups, takeView } from '../../lib/onboarding.svelte';
import { plugins } from '../../lib/plugins-view.svelte';
import { app } from '../../lib/state.svelte';
import type { Hookup, HookupOffer, Locks, OnboardingStep, OnboardingView, StepStatus } from '../../lib/types';
import { CLAUDE_ENGINE, fakeBackend, resetApp } from '../../test/ipc';
import Onboarding from './Onboarding.svelte';

// ESC-512: « Ce que tu veux brancher », the station of the first launch whose cards (the sources of tickets, the voice,
// the code host) are none ticked, each with its « Plus tard »; what it offers is the core's list; choosing a thing
// installs its plugin (onboarding_need) and opens its connection by the components of Réglages.

const STEPS: OnboardingStep[] = ['harness', 'account', 'defaults', 'hookups', 'project', 'agent', 'telemetry'];
const BASE = 147_951_465;

/** The core's list, a source this version of the window does not know among them (it is the core's, never written here). */
function offers(over: Partial<HookupOffer>[] = []): HookupOffer[] {
  const o = (kind: Hookup, id: string, name: string, more: Partial<HookupOffer> = {}): HookupOffer => ({
    kind,
    id,
    name,
    plugin: `dev.escouade.${id}`,
    installed: false,
    seed: true,
    denied: null,
    ...more,
  });
  const list = [
    o('sources', 'jira', 'Jira'),
    o('sources', 'trello', 'Trello'),
    o('sources', 'youtrack', 'YouTrack', { seed: false, plugin: 'com.acme.youtrack' }),
    o('voice', 'openai', 'OpenAI'),
    o('voice', 'mistral', 'Mistral'),
    o('forge', 'github', 'GitHub'),
    o('forge', 'gitlab', 'GitLab'),
  ];
  return list.map((x) => ({ ...x, ...(over.find((y) => y.kind === x.kind && y.id === x.id) ?? {}) }));
}

/** A plugin view that brings `kind` `id`, as the installed plugins' list gives it. */
function pluginOf(id: string, contributes: Record<string, unknown>, activation: string[] = []) {
  return {
    id,
    name: id,
    version: '2.2.0',
    description: '',
    origin: { kind: 'store', seed: true },
    fingerprint: '',
    folder: '',
    declarative: false,
    state: { kind: 'running' },
    ignored: [],
    manifest: { id, contributes, activation, permissions: {} },
    installedAt: '',
    choices: { enabled: true, projects: {}, permissions: {}, settings: {} },
    asks: [],
    secretKeys: [],
    update: null,
    commands: [],
    commandsApproved: true,
    verified: true,
    withdrawn: null,
    inStore: null,
  };
}

/** The journey at « Ce que tu veux brancher », the stations before it done; its cards as `cards` says. */
function view(cards: Partial<Record<Hookup, StepStatus>> = {}): OnboardingView {
  const done = new Set<OnboardingStep>(['harness', 'account', 'defaults']);
  return {
    open: true,
    trigger: 'first',
    steps: STEPS.map((id) => ({ id, status: done.has(id) ? 'done' : 'todo' })),
    resumeAt: 'hookups',
    started: true,
    finished: false,
    askTelemetryAgain: false,
    demo: '',
    hookups: (['sources', 'voice', 'forge'] as Hookup[]).map((id) => ({ id, status: cards[id] ?? 'todo' })),
  };
}

/** A core that keeps the cards as `onboarding.rs` does, and installs from the seed what `onboarding_need` asks. */
function core(start: OnboardingView, list: HookupOffer[] = offers(), extra: Record<string, (a: any) => unknown> = {}) {
  let v = start;
  let all = list;
  const installed: ReturnType<typeof pluginOf>[] = [];
  const backend = fakeBackend({
    onboarding_view: () => v,
    onboarding_act: ({ act }) => {
      if (act.kind === 'hookup')
        v = { ...v, hookups: v.hookups!.map((c) => (c.id === act.hookup && c.status !== 'done' ? { ...c, status: act.outcome } : c)) };
      if (act.kind === 'step') v = { ...v, steps: v.steps.map((s) => (s.id === act.step ? { ...s, status: act.outcome } : s)) };
      return v;
    },
    onboarding_offers: () => all,
    onboarding_need: ({ kind, id }) => {
      all = all.map((o) => (o.kind === kind && o.id === id ? { ...o, installed: true } : o));
      const plugin = `dev.escouade.${id}`;
      installed.push(
        kind === 'voice'
          ? pluginOf(plugin, {}, [`voice:${id}`])
          : pluginOf(plugin, kind === 'sources' ? { ticketSources: [{ id }] } : { forges: [{ id }] }),
      );
      return installed.at(-1);
    },
    plugins_list: () => installed,
    plugin_seed_offers: () => [],
    store_view: () => ({ url: '', catalog: null, readAt: null, error: null, best: {}, revocations: null, platform: '' }),
    voice_info: () => ({ devices: [], models: [{ id: 'base', size: BASE, speed: 4, quality: 3, downloaded: false }] }),
    voice_online_keys: () => [],
    save_settings: () => [],
    forges: () => ({ providers: [], accounts: [], unknown: [], issues: [] }),
    forge_check: () => ({ providers: [], accounts: [], unknown: [], issues: [] }),
    ...extra,
  });
  return backend;
}

const cards = () => within(screen.getByRole('group', { name: 'Les branchements' })).getAllByRole('checkbox');
const card = (name: RegExp) => screen.getByRole('checkbox', { name });

describe('« Ce que tu veux brancher » (ESC-512)', () => {
  beforeEach(() => {
    resetApp({ projects: [] });
    app.settings = { ...app.settings, accounts: [], agentMode: 'acceptEdits', telemetry: null };
    app.engines = [CLAUDE_ENGINE];
    app.enginesFound = ['claude'];
    plugins.loaded = false;
    plugins.list = [];
    setLocks(undefined);
    dictation.clear();
  });
  afterEach(() => {
    onboarding.shown = false;
    setLang('fr');
  });

  it('has three cards on the line after the starting settings, none ticked, each saying what it is for and asks, its list the core’s', async () => {
    core(view());
    takeView(view());
    render(Onboarding);
    expect(screen.getByRole('heading', { level: 1, name: 'Que veux-tu brancher ?' })).toBeInTheDocument();
    const line = within(screen.getByRole('navigation')).getAllByRole('listitem');
    expect(line[3]).toHaveTextContent('Ce que tu veux brancher');
    expect(line[3].querySelector('[aria-current="step"]')).not.toBeNull();
    expect(screen.getByText('7 stations · environ 2 minutes')).toBeInTheDocument();
    expect(cards().map((c) => c.textContent?.trim())).toEqual(['Une source de tickets', 'La voix', 'Un hébergeur de code']);
    for (const c of cards()) expect(c).toHaveAttribute('aria-checked', 'false');
    // The real list of the core: a source of a plugin of the store among them, named as it is.
    await waitFor(() => expect(card(/source de tickets/)).toHaveAccessibleDescription(/Jira, Trello et YouTrack/));
    expect(card(/hébergeur/)).toHaveAccessibleDescription(/GitHub et GitLab.*jeton de l’hébergeur/);
    expect(card(/voix/)).toHaveAccessibleDescription(/Veux-tu dicter ou parler à tes agents/);
  });

  it('passes every card left unticked on « Continuer », installs nothing, and the cards passed wait in the discovery', async () => {
    const backend = core(view());
    takeView(view());
    render(Onboarding);
    await userEvent.click(screen.getByRole('button', { name: /^Continuer/ }));
    await waitFor(() =>
      expect(backend.called('onboarding_act').map((c) => c.args.act.kind)).toEqual(['hookup', 'hookup', 'hookup', 'step']),
    );
    expect(
      backend
        .called('onboarding_act')
        .slice(0, 3)
        .map((c) => [c.args.act.hookup, c.args.act.outcome]),
    ).toEqual([
      ['sources', 'skipped'],
      ['voice', 'skipped'],
      ['forge', 'skipped'],
    ]);
    expect(backend.called('onboarding_need')).toHaveLength(0);
    expect(backend.called('voice_download')).toHaveLength(0);
    expect(backend.called('save_settings')).toHaveLength(0);
    expect(passedHookups()).toEqual(['sources', 'voice', 'forge']);
    expect(onboarding.at).toBe('project');
  });

  it('a card’s own « Plus tard » passes it alone; Espace ticks a card, ↑ ↓ go from one to the next', async () => {
    const backend = core(view());
    takeView(view());
    render(Onboarding);
    card(/source de tickets/).focus();
    await userEvent.keyboard(' ');
    expect(card(/source de tickets/)).toHaveAttribute('aria-checked', 'true');
    await userEvent.keyboard('{ArrowDown}');
    expect(card(/voix/)).toHaveFocus();
    await userEvent.click(screen.getByRole('button', { name: 'Plus tard : Une source de tickets' }));
    expect(card(/source de tickets/)).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText('Passé · à reprendre')).toBeInTheDocument();
    await waitFor(() =>
      expect(backend.called('onboarding_act').map((c) => c.args.act)).toEqual([{ kind: 'hookup', hookup: 'sources', outcome: 'skipped' }]),
    );
    // The station is still on screen: the other cards wait.
    expect(onboarding.at).toBe('hookups');
  });

  it('a source chosen installs its plugin alone, from the seed, and opens Réglages’ form of it here', async () => {
    const backend = core(view(), offers(), {
      integration_connect: ({ service, account }) => ({ id: 'a1', service, connected: true, label: '@ada', ...account }),
    });
    takeView(view());
    render(Onboarding);
    await userEvent.click(card(/source de tickets/));
    const pick = screen.getByRole('radiogroup', { name: 'Quelle source ?' });
    expect(within(pick).getByRole('radio', { name: /Trello/ })).toHaveTextContent(
      'Les cartes d’un tableau Trello · une clé et un jeton d’API',
    );
    // A source the window does not know: its general line.
    expect(within(pick).getByRole('radio', { name: /YouTrack/ })).toHaveTextContent('un jeton de la source');
    await userEvent.click(within(pick).getByRole('radio', { name: /Trello/ }));
    await waitFor(() => expect(backend.called('onboarding_need').map((c) => c.args)).toEqual([{ kind: 'sources', id: 'trello' }]));
    const form = await screen.findByRole('form', { name: 'Connexion à Trello' });
    expect(screen.getByText('Plugin Trello installé.')).toBeInTheDocument();
    await userEvent.type(within(form).getByLabelText("Clé d'API"), 'cle');
    await userEvent.type(within(form).getByLabelText('Jeton'), 'jeton');
    await userEvent.click(within(form).getByRole('button', { name: 'Connecter' }));
    await waitFor(() => expect(backend.called('integration_connect').map((c) => c.args.service)).toEqual(['trello']));
    // A source of the store's alone: said so, never installed from here.
    await userEvent.click(within(pick).getByRole('radio', { name: /YouTrack/ }));
    expect(await screen.findByText(/Son plugin vient du store : installe-le depuis Plugins/)).toBeInTheDocument();
    expect(backend.called('onboarding_need')).toHaveLength(1);
  });

  it('the voice on the machine: the model’s weight said before, its download followed and cancelled, nothing else installed', async () => {
    let finish = () => {};
    const backend = core(view(), offers(), { voice_download: () => new Promise<void>((r) => (finish = r)) });
    takeView(view());
    render(Onboarding);
    await userEvent.click(card(/voix/));
    const how = screen.getByRole('radiogroup', { name: 'Qui transcrit ce que tu dis ?' });
    await userEvent.click(within(how).getByRole('radio', { name: /Whisper, sur ta machine/ }));
    expect(how).toHaveTextContent('Le modèle « base » : 141 Mo à télécharger une fois');
    expect(backend.called('voice_download')).toHaveLength(0);
    await userEvent.click(screen.getByRole('button', { name: 'Télécharger (141 Mo)' }));
    await waitFor(() => expect(backend.called('voice_download').map((c) => c.args.model)).toEqual(['base']));
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Annuler le téléchargement' }));
    expect(backend.called('voice_download_cancel').map((c) => c.args.model)).toEqual(['base']);
    finish();
    expect(backend.called('onboarding_need')).toHaveLength(0);
  });

  it('the voice on line: a service of the core’s list, its plugin, the settings Réglages › Voix writes, its key, what leaves said', async () => {
    const backend = core(view());
    takeView(view());
    render(Onboarding);
    await userEvent.click(card(/voix/));
    await userEvent.click(screen.getByRole('radio', { name: /Un service en ligne/ }));
    const services = screen.getByRole('radiogroup', { name: 'Le service' });
    expect(
      within(services)
        .getAllByRole('radio')
        .map((r) => r.textContent?.trim()),
    ).toEqual(['OpenAI', 'Mistral']);
    await userEvent.click(within(services).getByRole('radio', { name: 'Mistral' }));
    await waitFor(() => expect(backend.called('onboarding_need').map((c) => c.args)).toEqual([{ kind: 'voice', id: 'mistral' }]));
    await waitFor(() =>
      expect(backend.called('save_settings').at(-1)?.args.settings.voice).toMatchObject({
        transcription: 'online',
        onlineService: 'mistral',
      }),
    );
    expect(await screen.findByText(/Ce qui part chez Mistral \(api\.mistral\.ai\) : l’audio de tes dictées/)).toBeInTheDocument();
    expect(document.querySelector('[data-online-key="mistral"]')).not.toBeNull();
  });

  it('the voice ready: a sentence dictated into a field of the station, sent to nobody', async () => {
    core(view(), offers(), {
      voice_info: () => ({ devices: [], models: [{ id: 'base', size: BASE, speed: 4, quality: 3, downloaded: true }] }),
    });
    takeView(view());
    render(Onboarding);
    await userEvent.click(card(/voix/));
    await userEvent.click(screen.getByRole('radio', { name: /Whisper, sur ta machine/ }));
    expect(await screen.findByText('Modèle « base » prêt')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Ta phrase dictée' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Dicter' })).toBeInTheDocument();
    expect(screen.getByText(/Rien n’est envoyé à un agent/)).toBeInTheDocument();
  });

  it('a device that turns the dictation off shows the voice locked, with who; a plugin it denies is greyed with why', async () => {
    const locks: Locks = {
      locks: { 'voice.dictation': { key: 'voice.dictation', value: false, by: { kind: 'device', source: 'HKLM' }, bounds: [] } },
      rows: {},
      warnings: [],
    } as unknown as Locks;
    setLocks(locks);
    const denied = 'Le plugin « dev.escouade.trello » est interdit par la stratégie de ce poste.';
    core(view({ voice: 'policy' }), offers([{ kind: 'sources', id: 'trello', denied }]));
    takeView(view({ voice: 'policy' }));
    render(Onboarding);
    expect(card(/voix/)).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByText(/Verrouillé · Imposé par la stratégie de ce poste/)).toBeInTheDocument();
    await userEvent.click(card(/voix/));
    expect(card(/voix/)).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(card(/source de tickets/));
    await waitFor(() => expect(screen.getByRole('radio', { name: /Trello/ })).toHaveAttribute('aria-disabled', 'true'));
    expect(screen.getByRole('radio', { name: /Trello/ })).toHaveTextContent(denied);
  });

  it('a code host: its plugin, then the code hosts’ form of Réglages, here', async () => {
    const backend = core(view());
    takeView(view());
    render(Onboarding);
    await userEvent.click(card(/hébergeur/));
    await userEvent.click(screen.getByRole('radio', { name: /GitLab/ }));
    await waitFor(() => expect(backend.called('onboarding_need').map((c) => c.args)).toEqual([{ kind: 'forge', id: 'gitlab' }]));
    expect(await screen.findByText('Plugin GitLab installé.')).toBeInTheDocument();
  });

  it('offline, what needs the network says so and waits; the plugin of the seed installs all the same', async () => {
    const onLine = Object.getOwnPropertyDescriptor(Navigator.prototype, 'onLine');
    Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => false });
    try {
      const backend = core(view());
      takeView(view());
      render(Onboarding);
      await userEvent.click(card(/voix/));
      await userEvent.click(screen.getByRole('radio', { name: /Whisper, sur ta machine/ }));
      expect(await screen.findByRole('button', { name: 'Télécharger (141 Mo)' })).toBeDisabled();
      expect(screen.getByRole('status')).toHaveTextContent('Hors ligne : ceci demande le réseau. La carte t’attend dans À découvrir');
      await userEvent.click(card(/source de tickets/));
      await userEvent.click(screen.getByRole('radio', { name: /Jira/ }));
      await waitFor(() => expect(backend.called('onboarding_need').map((c) => c.args)).toEqual([{ kind: 'sources', id: 'jira' }]));
      expect(await screen.findByRole('form', { name: 'Connexion à Jira' })).toBeInTheDocument();
      expect(backend.called('voice_download')).toHaveLength(0);
    } finally {
      delete (navigator as unknown as Record<string, unknown>).onLine;
      if (onLine) Object.defineProperty(Navigator.prototype, 'onLine', onLine);
    }
  });

  it('speaks English', async () => {
    setLang('en');
    core(view());
    takeView(view());
    render(Onboarding);
    expect(screen.getByRole('heading', { level: 1, name: 'What would you like to plug in?' })).toBeInTheDocument();
    const group = screen.getByRole('group', { name: 'Hookups' });
    expect(
      within(group)
        .getAllByRole('checkbox')
        .map((c) => c.textContent?.trim()),
    ).toEqual(['A ticket source', 'Voice', 'A code host']);
    expect(screen.getByRole('button', { name: /^Continue/ })).toBeInTheDocument();
  });
});
