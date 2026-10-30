import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { Manifest, PluginPreview, PluginView } from '../../lib/plugins';
import { plugins } from '../../lib/plugins-view.svelte';
import { app } from '../../lib/state.svelte';
import { askedOf, fold, matches, type CatalogPlugin, type CatalogVersion, type StoreView } from '../../lib/store';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import InstallModal from './InstallModal.svelte';
import PluginsView from './PluginsView.host.test.svelte';

// The store in the app (STORE-9): the catalogue kept, searched on this machine (accents and case ignored), filtered by
// category and « Vérifiés seulement », each line with its seal or « Non vérifié » and what it asks for in one line;
// « Pour <projet> » with its reasons, « Pas maintenant »; installing goes through the window, which says the seal checked
// here or the band of an unverified version; the installed ones say a version withdrawn by Escouade, the seal, an update
// not verified, and keep « N'installer que des versions vérifiées ».

function version(v: string, over: Partial<CatalogVersion> = {}): CatalogVersion {
  return {
    version: v,
    published_at: '2026-10-29T08:00:00.000Z',
    sha256: 'a'.repeat(64),
    archive: { sha256: 'b'.repeat(64), size: 10 },
    platforms: [],
    escouade: { api: '1', min: '2.1.0' },
    verified: false,
    yanked: false,
    permissions: {},
    run: false,
    contributes: [],
    files: { version_json: '', archive: '', binaries: {} },
    ...over,
  };
}

function listing(id: string, name: string, over: Partial<CatalogPlugin> = {}): CatalogPlugin {
  return {
    id,
    name,
    description: { fr: `${name}, en français`, en: `${name}, in English` },
    author: 'escouade-dev',
    repository: `https://github.com/escouade-dev/${id}`,
    license: 'MIT',
    categories: [],
    keywords: [],
    installs: 0,
    latest: '1.0.0',
    versions: [version('1.0.0')],
    ...over,
  };
}

const CATALOG: CatalogPlugin[] = [
  listing('dev.exemple.high-contrast', 'Contraste élevé', {
    categories: ['themes'],
    keywords: ['thème'],
    installs: 2310,
    versions: [version('1.0.0', { verified: true, review: { signed_at: '2026-10-24T10:00:00Z', summary: { fr: 'Relu.', en: 'Read.' } } })],
  }),
  listing('dev.exemple.slack', 'Slack', {
    categories: ['notifications'],
    installs: 1284,
    versions: [version('1.0.0', { run: true, permissions: { network: ['hooks.slack.com'], secrets: ['webhook'] } })],
  }),
  listing('dev.exemple.team-rules', 'Règles de l’équipe', {
    categories: ['permissions'],
    versions: [version('1.0.0', { verified: true, run: true, permissions: { decide: true } })],
  }),
  listing('dev.exemple.vibe', 'Mistral Vibe', {
    categories: ['harness'],
    keywords: ['vibe'],
    versions: [version('1.0.2', { verified: true, run: true })],
  }),
];

function store(over: Partial<StoreView> = {}): StoreView {
  return {
    url: 'https://store.escouade.dev',
    catalog: { format: 1, updated_at: '', installs_until: '', plugins: CATALOG },
    readAt: '2026-10-30T10:42:00Z',
    error: null,
    best: Object.fromEntries(CATALOG.map((p) => [p.id, p.versions[0].version])),
    revocations: { serial: 18, readAt: '2026-10-30T10:42:00Z', from: 'store', error: null },
    platform: 'windows-x64',
    ...over,
  };
}

const MANIFEST: Manifest = {
  id: 'dev.exemple.high-contrast',
  name: 'Contraste élevé',
  version: '1.0.0',
  escouade: { api: 1, min: '2.1.0' },
  activation: [],
  permissions: { events: [], network: [], secrets: [], commands: [], decide: false },
  contributes: { themes: [{ id: 'hc' }] },
};

function installed(over: Partial<PluginView> = {}): PluginView {
  return {
    id: MANIFEST.id,
    name: 'Contraste élevé',
    version: '1.0.0',
    description: '',
    origin: {
      kind: 'store',
      version: '1.0.0',
      sha256: 'a'.repeat(64),
      verified: true,
      repository: 'https://github.com/x/y',
      tag: 'v1.0.0',
      commit: 'c'.repeat(40),
    },
    fingerprint: `store:${'a'.repeat(64)}`,
    folder: '',
    declarative: true,
    state: { kind: 'ready' },
    ignored: [],
    manifest: MANIFEST,
    installedAt: '2026-10-29T10:00:00Z',
    choices: { enabled: true, projects: {}, revoked: [], settings: {} },
    asks: [],
    secretKeys: [],
    update: null,
    commands: [],
    commandsApproved: true,
    verified: true,
    withdrawn: null,
    inStore: null,
    ...over,
  };
}

function preview(over: Partial<PluginPreview> = {}): PluginPreview {
  return {
    token: 'tok',
    icon: null,
    id: MANIFEST.id,
    name: 'Contraste élevé',
    version: '1.0.0',
    description: '',
    origin: installed().origin,
    fingerprint: `store:${'a'.repeat(64)}`,
    declarative: true,
    refusal: null,
    asks: [],
    previous: null,
    grown: [],
    manifest: MANIFEST,
    verified: true,
    unverified: null,
    ...over,
  };
}

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [project({ id: 'p1', name: 'demo-api' })] });
  app.ui.activeProject = 'p1';
  plugins.list = [];
  plugins.install = null;
  plugins.error = null;
  plugins.page = { kind: 'store' };
  plugins.store = null;
  plugins.query = '';
  plugins.verifiedFilter = false;
  plugins.suggestions = [];
  plugins.suggestionsFor = null;
  plugins.dismissed = {};
  localStorage.clear();
});

describe('the store’s search', () => {
  it('folds accents and case, and looks in the name, the description, the keywords and the author', () => {
    expect(fold('Thème Élevé')).toBe('theme eleve');
    const hc = CATALOG[0];
    expect(matches(hc, 'CONTRASTE eleve', 'fr')).toBe(true);
    expect(matches(hc, 'theme', 'fr')).toBe(true);
    expect(matches(hc, 'in english', 'fr')).toBe(true);
    expect(matches(hc, 'escouade-dev', 'fr')).toBe(true);
    expect(matches(hc, 'slack', 'fr')).toBe(false);
  });

  it('says what a version asks for in one line, the strongest first', () => {
    const words = (v: CatalogVersion) => askedOf(v).map((a) => a.text);
    expect(words(CATALOG[1].versions[0])).toEqual(['Lance un programme', 'hooks.slack.com', 'Un secret']);
    expect(words(CATALOG[2].versions[0])).toEqual(['Lance un programme', 'Décide des autorisations']);
    expect(words(CATALOG[0].versions[0])).toEqual(['Sans programme']);
  });
});

describe('the store page', () => {
  it('shows the catalogue kept, its seals, and searches it without asking the core', async () => {
    const backend = fakeBackend({ store_view: () => store(), plugins_list: () => [] });
    render(PluginsView);
    expect(await screen.findByText(/^store\.escouade\.dev · catalogue lu à \d\d:\d\d, la recherche reste sur ta machine$/)).toBeTruthy();
    const verified = await screen.findByRole('heading', { name: /Vérifiés par Escouade/ });
    const column = verified.closest('.sec') as HTMLElement;
    expect(column.textContent).toContain('Contraste élevé');
    expect(column.textContent).toContain('Vérifiév1.0.0');
    expect(column.textContent).not.toContain('Slack');
    expect(column.textContent).toMatch(/escouade-dev · 2\s310/);

    const before = backend.calls.length;
    await userEvent.type(screen.getByRole('searchbox', { name: 'Chercher dans le store' }), 'règles');
    expect(screen.getByRole('heading', { name: '1 plugin' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Règles de l’équipe' })).toBeTruthy();
    expect(backend.calls.slice(before).filter((c) => c.cmd.startsWith('store'))).toEqual([]);
    await userEvent.clear(screen.getByRole('searchbox'));

    // « Vérifiés seulement »: the lines without a seal go.
    await userEvent.click(screen.getByRole('switch', { name: 'Vérifiés seulement' }));
    const rows = [...document.querySelectorAll('.rowp .nm b')].map((b) => b.textContent);
    expect(rows).toEqual(['Contraste élevé', 'Règles de l’équipe', 'Mistral Vibe']);
    // A category chip.
    await userEvent.click(screen.getByRole('switch', { name: 'Vérifiés seulement' }));
    await userEvent.click(within(screen.getByRole('group', { name: 'Catégories' })).getByRole('button', { name: /^Notifications/ }));
    expect([...document.querySelectorAll('.rowp .nm b')].map((b) => b.textContent)).toEqual(['Slack']);
    expect(document.querySelector('.rowp')?.textContent).toContain('Non vérifié');
  });

  it('suggests for the project open, with what it saw; « Pas maintenant » puts one away', async () => {
    const backend = fakeBackend({
      store_view: () => store(),
      plugins_list: () => [],
      store_suggestions: () => [{ id: 'dev.exemple.vibe', reason: 'vibe est installé sur ce poste' }],
    });
    render(PluginsView);
    const box = await screen.findByRole('region', { name: 'Pour demo-api' });
    expect(box.textContent).toContain('Mistral Vibe');
    expect(box.textContent).toContain('vibe est installé sur ce poste');
    expect(backend.called('store_suggestions')[0].args).toEqual({ project: 'p1' });
    await userEvent.click(within(box).getByRole('button', { name: 'Pas maintenant' }));
    expect(screen.queryByRole('region', { name: 'Pour demo-api' })).toBeNull();
    expect(JSON.parse(localStorage.getItem('escouade.store.suggestions') ?? '{}')['dev.exemple.vibe']).toBeGreaterThan(Date.now());
  });

  it('installs through the window, which the core fills after checking the version', async () => {
    const backend = fakeBackend({
      store_view: () => store(),
      plugins_list: () => [],
      plugin_inspect_store: () => preview({ verified: false, origin: { ...installed().origin, verified: false } as never }),
    });
    render(PluginsView);
    const row = (await screen.findAllByRole('button', { name: 'Slack' }))[0].closest('.rowp') as HTMLElement;
    await userEvent.click(within(row).getByRole('button', { name: 'Installer' }));
    expect(backend.called('plugin_inspect_store')[0].args).toEqual({ id: 'dev.exemple.slack', version: null });
    expect(plugins.install?.kind).toBe('preview');
  });

  it('opens the page of a plugin not installed, with its seal, its review and its versions', async () => {
    fakeBackend({ store_view: () => store(), plugins_list: () => [] });
    render(PluginsView);
    await userEvent.click((await screen.findAllByRole('button', { name: 'Contraste élevé' }))[0]);
    expect(plugins.page).toEqual({ kind: 'storePlugin', id: 'dev.exemple.high-contrast', tab: 'overview' });
    expect(await screen.findByRole('button', { name: 'Installer la v1.0.0' })).toBeTruthy();
    expect(document.querySelector('.phead')?.textContent).toContain('Vérifié par Escouade');
    expect(document.querySelector('.review')?.textContent).toContain('Relu par Escouade le');
    expect(document.querySelector('.review')?.textContent).toContain('Relu.');
    await userEvent.click(screen.getByRole('tab', { name: /Versions/ }));
    expect(document.querySelector('.vers')?.textContent).toContain('celle que cette version d’Escouade installe');
  });

  it('says when the store does not answer, the catalogue kept still shown', async () => {
    fakeBackend({
      store_view: () => store({ error: 'Le store ne répond pas (timeout) : le catalogue gardé sur ce poste est montré.' }),
      plugins_list: () => [],
    });
    render(PluginsView);
    expect(await screen.findByText(/Le store ne répond pas/)).toBeTruthy();
    expect(screen.getAllByRole('button', { name: 'Slack' }).length).toBeGreaterThan(0);
  });
});

describe('the install window, from the store', () => {
  it('says the seal checked here, and « Installer »', () => {
    plugins.install = { kind: 'preview', preview: preview(), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Installer Contraste élevé ?' });
    expect(dialog.textContent).toContain('Vérifié par Escouade');
    expect(dialog.textContent).toContain('vérifiée ici, par l’app');
    expect(dialog.textContent).toContain('archive reçue identique');
    expect(dialog.textContent).toContain('v1.0.0 → ccccccc');
    expect(dialog.textContent).not.toContain('Personne n’a relu');
    expect(within(dialog).getByRole('button', { name: 'Installer' })).toBeTruthy();
  });

  it('an unverified version: the band that says it, no signature, « Installer quand même »', () => {
    plugins.install = { kind: 'preview', preview: preview({ verified: false }), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog');
    expect(dialog.textContent).toContain('Personne n’a relu cette version.');
    expect(dialog.textContent).toContain('son archive est bien celle que l’auteur a publiée');
    expect(dialog.textContent).toContain('Aucune : version non vérifiée');
    expect(within(dialog).getByRole('button', { name: 'Installer quand même' })).toBeTruthy();
  });
});

describe('the installed ones, from the store', () => {
  it('show the seal, an update not verified, a version withdrawn by Escouade and the revocation list', async () => {
    plugins.page = { kind: 'installed' };
    fakeBackend({
      store_view: () => store(),
      plugins_list: () => [
        installed({ update: { version: '1.1.0', verified: false } }),
        installed({
          id: 'dev.exemple.team-rules',
          name: 'Règles de l’équipe',
          verified: false,
          state: { kind: 'disabled', reason: 'Retiré par Escouade le 2026-10-30 : Envoyait les commandes ailleurs.' },
          withdrawn: { action: 'disable', version: '1.0.0', reason: 'Envoyait les commandes ailleurs.', date: '2026-10-30' },
        }),
      ],
    });
    render(PluginsView);
    const table = await screen.findByRole('table', { name: 'Ajoutés' });
    expect(table.textContent).toContain('v1.1.0 · non vérifiée');
    expect(table.querySelector('.seal')?.textContent).toContain('v1.0.0');
    expect(table.textContent).toContain('store.escouade.dev');
    expect(table.textContent).toContain('Désactivé par Escouade');
    const band = screen.getByRole('alert');
    expect(band.textContent).toContain('Règles de l’équipe v1.0.0 a été retiré par Escouade le 2026-10-30.');
    expect(band.textContent).toContain('ses réglages et ses secrets sont encore là');
    expect(screen.getByText(/Liste de révocation lue à \d\d:\d\d · nº 18/)).toBeTruthy();
  });

  it('keeps « N’installer que des versions vérifiées » with the settings', async () => {
    plugins.page = { kind: 'installed' };
    const backend = fakeBackend({ store_view: () => store(), plugins_list: () => [] });
    render(PluginsView);
    const toggle = await screen.findByRole('switch', { name: 'N’installer que des versions vérifiées' });
    expect(toggle.getAttribute('aria-checked')).toBe('false');
    await userEvent.click(toggle);
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    expect(backend.called('save_settings').at(-1)?.args.settings.pluginsVerifiedOnly).toBe(true);
  });

  it('a plugin from git whose id the store has offers to move to the store’s version', async () => {
    plugins.page = { kind: 'plugin', id: MANIFEST.id, tab: 'overview' };
    const backend = fakeBackend({
      store_view: () => store(),
      plugins_list: () => [
        installed({
          origin: { kind: 'git', url: 'https://github.com/x/y', tag: 'v1.0.0' },
          fingerprint: 'git:abc',
          verified: false,
          inStore: { version: '1.0.0', verified: true },
        }),
      ],
      plugin_inspect_store: () => preview({ previous: { version: '1.0.0', fingerprint: 'git:abc', asks: [] } }),
    });
    render(PluginsView);
    await userEvent.click(await screen.findByRole('button', { name: 'Passer à la v1.0.0 du store' }));
    expect(backend.called('plugin_inspect_store')[0].args).toEqual({ id: MANIFEST.id, version: null });
  });
});

describe('the link escouade://plugin/<id> (STORE-11)', () => {
  it('opens the page of the plugin in the view Plugins, and installs nothing', async () => {
    const backend = fakeBackend({
      store_view: () => store(),
      plugins_list: () => [],
      deep_links_take: () => [{ kind: 'plugin', id: 'dev.exemple.high-contrast' }],
    });
    app.ui.view = 'project';
    await plugins.takeLinks();
    expect(app.ui.view).toBe('plugins');
    expect(plugins.page).toEqual({ kind: 'storePlugin', id: 'dev.exemple.high-contrast', tab: 'overview' });
    render(PluginsView);
    expect(await screen.findByRole('button', { name: 'Installer la v1.0.0' })).toBeTruthy();
    expect(plugins.install).toBeNull();
    expect(backend.called('plugin_inspect_store')).toEqual([]);
    expect(backend.called('plugin_install')).toEqual([]);
  });

  it('opens the page of an installed plugin as installed', async () => {
    fakeBackend({ deep_links_take: () => [{ kind: 'plugin', id: MANIFEST.id }] });
    plugins.list = [installed()];
    await plugins.takeLinks();
    expect(plugins.page).toEqual({ kind: 'plugin', id: MANIFEST.id, tab: 'overview' });
  });

  it('says an id the store does not have, and a link it cannot read, nothing else done', async () => {
    fakeBackend({
      store_view: () => store(),
      plugins_list: () => [],
      deep_links_take: () => [
        { kind: 'plugin', id: 'dev.exemple.nowhere' },
        { kind: 'bad', url: 'escouade://plugin/nope' },
      ],
    });
    await plugins.takeLinks();
    render(PluginsView);
    expect(await screen.findByText('« dev.exemple.nowhere » n’est pas au catalogue du store.')).toBeTruthy();
    expect(app.toasts.map((t) => [t.kind, t.text])).toContainEqual([
      'error',
      'Lien Escouade illisible, rien n’a été fait : escouade://plugin/nope',
    ]);
    expect(plugins.install).toBeNull();
  });
});
