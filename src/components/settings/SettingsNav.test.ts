import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { plugins } from '../../lib/plugins-view.svelte';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

// The bar of the settings window, R8: the search and the amber dots (design/settings-2-2/PROPOSITION.md §2).

const nav = () => screen.getByRole('navigation', { name: 'Réglages' });
const search = () => within(nav()).getByRole('searchbox', { name: 'Rechercher un réglage' });
const hits = () => [...document.querySelectorAll<HTMLElement>('.s-nav [role=option]')];
const panel = () => document.getElementById('settings-panel')!;
const entry = (name: string | RegExp) => within(nav()).getByRole('button', { name });

describe('the search of the settings bar', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.modal = { kind: 'settings' };
    fakeBackend();
  });

  it('lists what answers in place of the pages, and opens a hit on its page, the row lit', async () => {
    render(SettingsModal);
    await userEvent.type(search(), 'proxy');
    expect(within(nav()).queryByRole('button', { name: 'Général' })).toBeNull();
    const [first] = hits();
    expect(first).toHaveTextContent('Proxy HTTP(S)');
    expect(first).toHaveTextContent('Avancé');
    expect(first).toHaveAttribute('aria-selected', 'true');
    await userEvent.keyboard('{Enter}');
    expect(settingsForm.page).toBe('advanced');
    await waitFor(() => expect(panel().querySelector('.s-found')).toHaveTextContent(/^Proxy HTTP\(S\)/));
  });

  it('ignores the case and the accents, and unfolds « Avancé » for a row folded there', async () => {
    app.settings.userId = 'u-1234';
    render(SettingsModal);
    await userEvent.type(search(), 'TON IDENTIFIANT');
    await userEvent.click(hits()[0]);
    expect(settingsForm.page).toBe('general');
    await waitFor(() => expect(within(panel()).getByRole('button', { name: /^Avancé/ })).toHaveAttribute('aria-expanded', 'true'));
    await waitFor(() => expect(panel().querySelector('.s-found')).toHaveTextContent(/^Ton identifiant/));
  });

  it('opens a page of a project in the project it names', async () => {
    render(SettingsModal, { page: 'general' });
    await userEvent.type(search(), 'worktree par agent');
    const inSite = hits().find((h) => h.textContent?.includes('site ›'))!;
    await userEvent.click(inSite);
    expect(settingsForm.page).toBe('git');
    expect(settingsForm.projectId).toBe('p2');
  });

  it('goes through the hits with the arrows, says when none answers, and empties itself on Escape', async () => {
    render(SettingsModal);
    await userEvent.type(search(), 'proxy');
    await userEvent.keyboard('{ArrowDown}');
    expect(hits()[1]).toHaveAttribute('aria-selected', 'true');
    expect(search()).toHaveAttribute('aria-activedescendant', hits()[1].id);
    await userEvent.type(search(), 'zzqx');
    expect(within(nav()).getByRole('status')).toHaveTextContent('Aucun réglage ne répond à « proxyzzqx »');
    await userEvent.keyboard('{Escape}');
    expect(search()).toHaveValue('');
    // The window stays open: Escape emptied the field.
    expect(app.modal).not.toBeNull();
    expect(entry('Général')).toBeInTheDocument();
  });

  it('takes the focus on Ctrl+F', async () => {
    render(SettingsModal);
    expect(search()).not.toHaveFocus();
    await userEvent.keyboard('{Control>}f{/Control}');
    expect(search()).toHaveFocus();
  });

  it('finds in English as in French, whatever the language of the interface', async () => {
    setLang('en');
    try {
      render(SettingsModal);
      const field = within(screen.getByRole('navigation', { name: 'Settings' })).getByRole('searchbox', { name: 'Search settings' });
      await userEvent.type(field, 'bêta');
      expect(hits()[0]).toHaveTextContent('Receive beta versions');
      expect(hits()[0]).toHaveTextContent('General');
    } finally {
      setLang('fr');
    }
  });
});

describe('the amber dots of the settings bar', () => {
  beforeEach(() => {
    resetApp({ projects: [project({ engine: 'codex' }), project({ id: 'p2', name: 'site' })] });
    app.modal = { kind: 'settings' };
  });

  it('says the pages that have a problem, their reason in the tooltip', async () => {
    app.enginesFound = ['claude', 'codex'];
    app.engines = [...app.engines, { ...app.engines[0], id: 'codex', name: 'Codex' }];
    fakeBackend({
      forges: () => ({
        providers: [],
        accounts: [{ label: '@ada · github.com', host: 'github.com', status: { health: 'expired', missing: [], error: null } }],
        unknown: [],
        issues: [],
      }),
    });
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    await waitFor(() => expect(entry('Services connectés')).toHaveAttribute('data-tip', 'Jeton expiré ou révoqué · @ada · github.com'));
    expect(entry('Comptes et harness')).toHaveAttribute('data-tip', 'Codex n’a aucun compte : aucun agent ni ticket n’y démarre');
    expect(entry('Comptes et harness').querySelector('.dotw')).not.toBeNull();
    // demo-api runs on Codex, which has no account: the project and its page « Projet » say it; site is served.
    expect(entry('demo-api')).toHaveAttribute('data-tip', expect.stringContaining('Aucun compte Codex ne sert ce projet'));
    expect(entry('Projet')).toHaveAttribute('data-tip', expect.stringContaining('Aucun compte Codex'));
    expect(entry('site').querySelector('.dotw')).toBeNull();
    expect(entry('Général').querySelector('.dotw')).toBeNull();
    expect(entry('Général')).not.toHaveAttribute('data-tip');
  });

  it('says a plugin stopped after its crashes on Plugins', async () => {
    plugins.loaded = false;
    fakeBackend({
      plugins_list: () => [
        {
          id: 'dev.x',
          name: 'Trello',
          state: { kind: 'ready' },
          choices: { enabled: true, projects: {} },
          declarative: false,
        },
      ],
      plugin_run: () => ({ status: { phase: 'stopped', stop: { kind: 'crashed', times: 3 }, restarts: 3 }, journal: [] }),
    });
    render(SettingsModal);
    await waitFor(() => expect(entry('Plugins')).toHaveAttribute('data-tip', 'Trello : Arrêté après 3 plantages'));
  });
});
