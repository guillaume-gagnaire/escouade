import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { Manifest, PluginView, SeedOffer } from '../../lib/plugins';
import { plugins } from '../../lib/plugins-view.svelte';
import { app } from '../../lib/state.svelte';
import type { StoreView } from '../../lib/store';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

// Réglages › Plugins (R7, ESC-384; design/settings-2-2/index.html#a-plugins): the installed plugins, what each adds and
// where it is set, on how many projects it is on, « Réglages » to its page in the view, its switch; the seed's not
// installed, each installed in a click with no window; « N'installer que des versions vérifiées », saved at the gesture.

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

function manifest(id: string, name: string, over: Partial<Manifest> = {}): Manifest {
  return {
    id,
    name,
    version: '1.0.0',
    escouade: { api: 1, min: '2.2.0' },
    activation: [],
    permissions: { events: [], network: [], secrets: [], commands: [], decide: false },
    contributes: {},
    ...over,
  };
}

function view(m: Manifest, over: Partial<PluginView> = {}): PluginView {
  return {
    id: m.id,
    name: textName(m),
    version: m.version,
    description: `${textName(m)}, sa description`,
    origin: { kind: 'store', version: m.version, sha256: '', verified: true, repository: '', tag: '', commit: '', seed: true },
    fingerprint: 'store:abcd',
    folder: '',
    declarative: false,
    state: { kind: 'ready' },
    ignored: [],
    manifest: m,
    installedAt: '2026-11-08T10:00:00Z',
    verified: true,
    withdrawn: null,
    inStore: null,
    choices: { enabled: true, projects: {}, revoked: [], settings: {} },
    asks: [],
    secretKeys: [],
    update: null,
    commands: [],
    commandsApproved: true,
    ...over,
  };
}
const textName = (m: Manifest) => (typeof m.name === 'string' ? m.name : m.name.fr);

const CLAUDE = view(
  manifest('dev.escouade.claude-code', 'Claude Code', { contributes: { harnesses: [{ id: 'claude', name: 'Claude Code' }] } }),
);
const GITHUB = view(
  manifest('dev.escouade.github', 'GitHub', {
    contributes: {
      accounts: [{ id: 'github', name: 'GitHub' }],
      ticketSources: [{ id: 'github', name: 'GitHub Issues' }],
      forges: [{ id: 'github', name: 'GitHub' }],
      settings: [{ key: 'labels', type: 'string' }],
    },
  }),
  // Off for p2 (private: off unless switched on), on for p1 by default, on for p3 by choice.
  { choices: { enabled: true, projects: { p3: true }, revoked: [], settings: {} } },
);
const DEEPGRAM = view(manifest('dev.escouade.deepgram', 'Deepgram', { activation: ['voice:deepgram'] }), { secretKeys: ['key'] });
const THEME = view(
  manifest('dev.escouade.high-contrast', 'Contraste élevé', { contributes: { themes: [{ id: 'hc', name: 'Contraste élevé' }] } }),
  {
    verified: false,
  },
);

const TRELLO: SeedOffer = { id: 'dev.escouade.trello', name: 'Trello', version: '2.2.0', removed: '1.3.0', refused: null };
const COPILOT: SeedOffer = { id: 'dev.escouade.copilot-cli', name: 'Copilot CLI', version: '2.2.0', removed: null, refused: null };

const STORE: StoreView = {
  url: 'https://store.escouade.dev',
  catalog: null,
  readAt: '2026-11-08T13:02:00Z',
  error: null,
  best: {},
  revocations: { serial: 418, readAt: '2026-11-08T13:02:00Z', error: null } as StoreView['revocations'],
  platform: 'windows-x64',
};

beforeEach(() => {
  setLang('fr');
  resetApp({
    projects: [
      project({ id: 'p1', name: 'demo-api' }),
      project({ id: 'p2', name: 'compta', private: true }),
      project({ id: 'p3', name: 'site' }),
    ],
  });
  plugins.list = [];
  plugins.offers = [];
  plugins.loaded = false;
  plugins.error = null;
  plugins.store = STORE;
  plugins.page = { kind: 'installed' };
});

const item = (region: HTMLElement, name: string) => within(region).getByRole('group', { name });

describe('Réglages › Plugins', () => {
  it('lists the installed ones: what each adds and where it is set, on how many projects, its seal, « Réglages » when it has some', async () => {
    fakeBackend({ plugins_list: () => [CLAUDE, GITHUB, DEEPGRAM, THEME], plugin_seed_offers: () => [] });
    render(SettingsModal, { page: 'plugins' });
    const installed = await screen.findByRole('region', { name: 'Installés' });
    await within(installed).findByRole('group', { name: 'Claude Code' });

    expect(item(installed, 'Claude Code')).toHaveTextContent('Un harness, dans « Nouvel agent » : Claude Code · Actif sur 2 projets sur 3');
    // A private project has it off until it is switched on there (§3.3).
    // Its accounts go without saying: its source and its host say where.
    expect(item(installed, 'GitHub')).toHaveTextContent(
      'Une source du Kanban, dans Réglages › Services connectés : GitHub Issues · Un hébergeur de code, dans Réglages › Services connectés : GitHub · Actif sur 2 projets sur 3',
    );
    expect(item(installed, 'Deepgram')).toHaveTextContent('Un service de voix, dans Réglages › Voix · Actif sur');
    expect(item(installed, 'Contraste élevé')).toHaveTextContent(
      'Un thème, dans Réglages › Général › Couleurs : Contraste élevé · Pour toute l’app',
    );
    expect(item(installed, 'GitHub')).toHaveTextContent('v1.0.0');

    // The seal on a verified version alone; « Réglages » on a plugin with settings or secrets alone.
    expect(within(item(installed, 'GitHub')).getByText('Vérifié')).toBeInTheDocument();
    expect(within(item(installed, 'Contraste élevé')).queryByText('Vérifié')).toBeNull();
    expect(within(item(installed, 'GitHub')).getByRole('button', { name: 'Réglages de GitHub, dans la vue Plugins' })).toBeInTheDocument();
    expect(within(item(installed, 'Deepgram')).getByRole('button', { name: /^Réglages de Deepgram/ })).toBeInTheDocument();
    expect(within(item(installed, 'Claude Code')).queryByRole('button', { name: /^Réglages de/ })).toBeNull();
  });

  it('« Réglages » opens its page in the view Plugins, on its settings; « Ouvrir la vue Plugins » the installed ones', async () => {
    fakeBackend({ plugins_list: () => [GITHUB], plugin_seed_offers: () => [] });
    app.modal = { kind: 'settings', page: 'plugins' };
    render(SettingsModal, { page: 'plugins' });
    const installed = await screen.findByRole('region', { name: 'Installés' });
    await userEvent.click(await within(installed).findByRole('button', { name: /^Réglages de GitHub/ }));
    expect(app.modal).toBeNull();
    expect(app.ui.view).toBe('plugins');
    expect(plugins.page).toEqual({ kind: 'plugin', id: GITHUB.id, tab: 'settings' });

    app.modal = { kind: 'settings', page: 'plugins' };
    app.ui.view = 'project';
    await userEvent.click(within(installed).getByRole('button', { name: 'Ouvrir la vue Plugins' }));
    expect(app.modal).toBeNull();
    expect(app.ui.view).toBe('plugins');
    expect(plugins.page).toEqual({ kind: 'installed' });
  });

  it('switches a plugin off and on again in Escouade: what « Réglages › Plugins le rallume » says', async () => {
    let on = false;
    const b = fakeBackend({
      plugins_list: () => [view(GITHUB.manifest!, { choices: { ...GITHUB.choices, enabled: on } })],
      plugin_seed_offers: () => [],
      plugin_set_enabled: (a: { on: boolean }) => ((on = a.on), view(GITHUB.manifest!, { choices: { ...GITHUB.choices, enabled: on } })),
    });
    render(SettingsModal, { page: 'plugins' });
    const sw = await screen.findByRole('switch', { name: 'Actif dans Escouade : GitHub' });
    expect(sw).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(sw);
    expect(b.called('plugin_set_enabled')[0].args).toEqual({ id: GITHUB.id, on: true });
    await expect.poll(() => sw.getAttribute('aria-checked')).toBe('true');
  });

  it('proposes the seed’s not installed, says why, and installs one in a click from the seed, with no window', async () => {
    let offers = [COPILOT, TRELLO];
    let list: PluginView[] = [CLAUDE];
    const trello = view(manifest(TRELLO.id, 'Trello', { contributes: { ticketSources: [{ id: 'trello', name: 'Trello' }] } }));
    const b = fakeBackend({
      plugins_list: () => list,
      plugin_seed_offers: () => offers,
      plugin_install_seed: () => ((offers = [COPILOT]), (list = [CLAUDE, trello]), trello),
    });
    render(SettingsModal, { page: 'plugins' });
    const seed = await screen.findByRole('region', { name: 'Proposés par Escouade' });
    expect(seed).toHaveTextContent('Dans l’installeur, rien à télécharger');
    expect(item(seed, 'Copilot CLI')).toHaveTextContent('Dans l’app, sans téléchargement.');
    expect(item(seed, 'Trello')).toHaveTextContent('Tu l’as désinstallé (v1.3.0).');

    await userEvent.click(within(item(seed, 'Trello')).getByRole('button', { name: 'Installer Trello' }));
    expect(b.called('plugin_install_seed')[0].args).toEqual({ id: TRELLO.id });
    // Never the store's window from here: the seed's bytes, at once.
    expect(b.called('plugin_inspect_store')).toHaveLength(0);
    expect(plugins.install).toBeNull();
    const installed = screen.getByRole('region', { name: 'Installés' });
    await within(installed).findByRole('group', { name: 'Trello' });
    expect(within(seed).queryByRole('group', { name: 'Trello' })).toBeNull();
  });

  it('says when nothing is installed, and has no seed group when the seed has nothing more', async () => {
    fakeBackend({ plugins_list: () => [], plugin_seed_offers: () => [] });
    render(SettingsModal, { page: 'plugins' });
    expect(await screen.findByText(/^Aucun plugin installé\./)).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Proposés par Escouade' })).toBeNull();
  });

  it('« N’installer que des versions vérifiées » says the revocation list read, and is saved at the gesture', async () => {
    const b = fakeBackend({ plugins_list: () => [], plugin_seed_offers: () => [], save_settings: () => [] });
    render(SettingsModal, { page: 'plugins' });
    const security = screen.getByRole('region', { name: 'Sécurité' });
    expect(security).toHaveTextContent(/Liste de révocation lue à .* · nº 418, signée, relue toutes les 6 heures/);
    const sw = within(security).getByRole('switch', { name: 'N’installer que des versions vérifiées' });
    expect(sw).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(sw);
    expect(sw).toHaveAttribute('aria-checked', 'true');
    await expect.poll(() => b.called('save_settings').length).toBe(1);
    expect(b.called('save_settings')[0].args.settings).toMatchObject({ pluginsVerifiedOnly: true });
    expect(app.settings.pluginsVerifiedOnly).toBe(true);
  });
});
