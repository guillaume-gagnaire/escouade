import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { PluginView } from '../../lib/plugins';
import { plugins } from '../../lib/plugins-view.svelte';
import { proposedPlugins, recommendedOf } from '../../lib/proposed-plugins';
import { app } from '../../lib/state.svelte';
import type { CatalogPlugin, StoreView } from '../../lib/store';
import type { Project } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

// The plugins a project proposes (PLG-14): the band at the head of Settings › Project names those of
// `plugins.recommended` not installed, each with its page in the store; nothing installed from it; « Pas pour ce projet »
// silences it for what it named, kept on this machine.

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

function listing(id: string, name: string, over: Partial<CatalogPlugin['versions'][number]> = {}): CatalogPlugin {
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
    versions: [
      {
        version: '1.0.0',
        published_at: '',
        sha256: '',
        archive: { sha256: '', size: 1 },
        platforms: [],
        escouade: { api: '1', min: '2.1.0' },
        verified: false,
        yanked: false,
        permissions: {},
        run: false,
        contributes: [],
        files: { version_json: '', archive: '', binaries: {} },
        ...over,
      },
    ],
  };
}

const CATALOG = [
  listing('dev.exemple.slack', 'Slack', { run: true, permissions: { network: ['hooks.slack.com'], secrets: ['webhook'] } }),
  listing('dev.exemple.team-rules', 'Règles de l’équipe', { run: true, permissions: { decide: true } }),
  listing('dev.exemple.high-contrast', 'Contraste élevé'),
];

const STORE: StoreView = {
  url: 'https://store.escouade.dev',
  catalog: { format: 1, updated_at: '', installs_until: '', plugins: CATALOG },
  readAt: '2026-10-30T10:42:00Z',
  error: null,
  best: Object.fromEntries(CATALOG.map((p) => [p.id, '1.0.0'])),
  revocations: null,
  platform: 'windows-x64',
};

function proposing(ids: unknown, over: Partial<Project> = {}): Project {
  return project({
    id: 'p1',
    name: 'demo-api',
    ...over,
    shared: {
      local: [],
      trusted: '',
      file: {
        keys: ['plugins.recommended'],
        values: { 'plugins.recommended': ids },
        warnings: [],
        error: null,
        version: 1,
        pending: null,
      },
    },
  });
}

const installed = (id: string) => ({ id, name: id, origin: { kind: 'store' } }) as unknown as PluginView;

beforeEach(() => {
  setLang('fr');
  localStorage.clear();
  plugins.list = [];
  plugins.loaded = false;
  plugins.store = null;
  plugins.notForProject = {};
  plugins.page = { kind: 'installed' };
  app.modal = { kind: 'settings' };
});

describe('what the file proposes', () => {
  it('reads plugins.recommended, leaves out what is installed or set aside', () => {
    const p = proposing(['dev.exemple.slack', 'dev.exemple.team-rules', 'dev.exemple.high-contrast']);
    expect(recommendedOf(p)).toEqual(['dev.exemple.slack', 'dev.exemple.team-rules', 'dev.exemple.high-contrast']);
    expect(proposedPlugins(p, (id) => id === 'dev.exemple.slack', ['dev.exemple.high-contrast'])).toEqual(['dev.exemple.team-rules']);
    expect(recommendedOf(proposing('dev.exemple.slack'))).toEqual([]);
    expect(recommendedOf(project({}))).toEqual([]);
    const unreadable = proposing(['dev.exemple.slack']);
    unreadable.shared!.file!.error = { line: 1, text: 'illisible' };
    expect(recommendedOf(unreadable)).toEqual([]);
  });
});

describe('the band of Settings › Project', () => {
  it('names the plugins proposed and not installed, with their page in the store, and installs nothing', async () => {
    resetApp({ projects: [proposing(['dev.exemple.slack', 'dev.exemple.team-rules', 'dev.exemple.high-contrast', 'dev.autre.absent'])] });
    const backend = fakeBackend({ plugins_list: () => [installed('dev.exemple.high-contrast')], store_view: () => STORE });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const band = await screen.findByRole('region', { name: 'Plugins proposés par ce projet' });
    expect(band.textContent).toContain('Ce projet propose 3 plugins');
    const names = [...band.querySelectorAll('li .what b')].map((b) => b.textContent);
    expect(names).toEqual(['Slack', 'Règles de l’équipe', 'dev.autre.absent']);
    expect(band.textContent).toContain('Lance un programme · hooks.slack.com · Un secret');
    expect(band.textContent).toContain('Absent du catalogue du store.');
    expect(band.textContent).not.toContain('Projet privé');

    await userEvent.click(within(band.querySelectorAll('li')[0] as HTMLElement).getByRole('button', { name: 'Voir dans le store' }));
    expect(app.modal).toBeNull();
    expect(app.ui.view).toBe('plugins');
    expect(plugins.page).toEqual({ kind: 'storePlugin', id: 'dev.exemple.slack', tab: 'overview' });
    // Nothing read for an install, nothing installed.
    expect(backend.calls.filter((c) => c.cmd.startsWith('plugin_inspect') || c.cmd === 'plugin_install')).toEqual([]);
  });

  it('« Pas pour ce projet » silences it, kept on this machine; a plugin proposed later shows again', async () => {
    resetApp({ projects: [proposing(['dev.exemple.slack'], { private: true })] });
    fakeBackend({ plugins_list: () => [], store_view: () => STORE });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const band = await screen.findByRole('region', { name: 'Plugins proposés par ce projet' });
    expect(band.textContent).toContain('Ce projet propose un plugin');
    expect(band.textContent).toContain('Projet privé : un plugin installé n’y est actif qu’une fois activé pour lui');
    await userEvent.click(within(band).getByRole('button', { name: 'Pas pour ce projet' }));
    expect(screen.queryByRole('region', { name: 'Plugins proposés par ce projet' })).toBeNull();
    expect(JSON.parse(localStorage.getItem('escouade.plugins.notForProject') ?? '{}')).toEqual({ p1: ['dev.exemple.slack'] });

    app.replaceProject(proposing(['dev.exemple.slack', 'dev.exemple.team-rules'], { private: true }));
    const again = await screen.findByRole('region', { name: 'Plugins proposés par ce projet' });
    expect([...again.querySelectorAll('li .what b')].map((b) => b.textContent)).toEqual(['Règles de l’équipe']);
  });

  it('says nothing when all are installed, or before the installed ones are known', async () => {
    resetApp({ projects: [proposing(['dev.exemple.slack'])] });
    let answer: (v: PluginView[]) => void = () => {};
    fakeBackend({ plugins_list: () => new Promise<PluginView[]>((r) => (answer = r)), store_view: () => STORE });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await screen.findByText('Exporter vers .escouade.yml');
    expect(screen.queryByRole('region', { name: 'Plugins proposés par ce projet' })).toBeNull();
    answer([installed('dev.exemple.slack')]);
    await vi.waitFor(() => expect(plugins.loaded).toBe(true));
    expect(screen.queryByRole('region', { name: 'Plugins proposés par ce projet' })).toBeNull();
  });
});
