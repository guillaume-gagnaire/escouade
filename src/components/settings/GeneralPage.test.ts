import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SAVE_DELAY_MS, zoomState } from '../../lib/display.svelte';
import { setLang } from '../../lib/i18n';
import { applyPluginTheme, chosenTheme, earlyPluginTheme, PLUGIN_THEME_KEY, projectPalette } from '../../lib/plugin-theme.svelte';
import type { PluginView } from '../../lib/plugins';
import { plugins } from '../../lib/plugins-view.svelte';
import { PROJECT_COLORS } from '../../lib/theme';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import { BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import { afterGesture } from '../../test/settings';

const panel = () => document.getElementById('settings-panel')!;
/** The choices of a setting, as its chips read. */
const choices = (name: string) =>
  within(within(panel()).getByRole('group', { name }))
    .getAllByRole('button')
    .map((b) => b.textContent?.trim());
const choice = (setting: string, name: string) =>
  within(within(panel()).getByRole('group', { name: setting })).getByRole('button', { name });

const UI = 'Langue de l’interface';
const CLAUDE = 'Langue des textes rédigés par les agents';

describe('the page « Général »', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'settings', page: 'general' };
  });

  it('comes first and chooses the languages, each named in itself', () => {
    fakeBackend();
    render(SettingsModal, { page: 'general' });
    expect(within(screen.getByRole('navigation')).getAllByRole('button')[0]).toHaveTextContent('Général');
    expect(within(screen.getByRole('navigation')).getByRole('button', { name: 'Général' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByText('Thème, zoom, langue, mises à jour et ce qui te concerne')).toBeInTheDocument();
    expect(within(panel()).getByRole('heading', { name: 'Langue' })).toBeInTheDocument();
    expect(choices(UI)).toEqual(['Système (Français)', 'English', 'Français']);
    expect(choices(CLAUDE)).toEqual(['Comme l’interface (par défaut)', 'English', 'Français']);
    expect(panel()).toHaveTextContent(
      'Messages de commit et descriptions de pull request proposés, commentaires publiés dans Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards, consignes données aux agents (par défaut, le message de commit proposé reprend la langue des derniers commits du dépôt).',
    );
    // By default: the system's, and the interface's for Claude.
    expect(choice(UI, 'Système (Français)')).toHaveAttribute('aria-pressed', 'true');
    expect(choice(CLAUDE, 'Comme l’interface (par défaut)')).toHaveAttribute('aria-pressed', 'true');
    // A language's name is read in that language.
    expect(choice(UI, 'English')).toHaveAttribute('lang', 'en');
    expect(choice(CLAUDE, 'Français')).toHaveAttribute('lang', 'fr');
  });

  it('names the system’s language in the language of the interface', () => {
    fakeBackend();
    app.lang = { ui: 'fr', system: 'en', claude: 'fr' };
    render(SettingsModal, { page: 'general' });
    expect(choices(UI)[0]).toBe('Système (Anglais)');
    setLang('en');
    flushSync();
    expect(choices('Interface language')[0]).toBe('System (English)');
    app.lang = { ui: 'en', system: 'fr', claude: 'en' };
    flushSync();
    expect(choices('Interface language')[0]).toBe('System (French)');
  });

  it('saves each language as it is chosen, once', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'general' });
    await userEvent.click(choice(UI, 'English'));
    await afterGesture();
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ language: 'en' });
    await userEvent.click(choice(CLAUDE, 'Français'));
    expect(choice(UI, 'English')).toHaveAttribute('aria-pressed', 'true');
    await afterGesture();
    expect(backend.called('save_settings')).toHaveLength(2);
    expect(backend.called('save_settings')[1].args.settings).toMatchObject({ language: 'en', claudeLanguage: 'fr' });
    expect(app.settings).toMatchObject({ language: 'en', claudeLanguage: 'fr' });
  });

  it('reads in English', () => {
    fakeBackend();
    setLang('en');
    render(SettingsModal, { page: 'general' });
    expect(within(screen.getByRole('navigation')).getByRole('button', { name: 'General' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByText('Theme, zoom, language, updates and what concerns you')).toBeInTheDocument();
    expect(within(panel()).getByRole('heading', { name: 'Appearance' })).toBeInTheDocument();
    expect(within(panel()).getByRole('heading', { name: 'Updates' })).toBeInTheDocument();
    expect(within(panel()).getByRole('heading', { name: 'You' })).toBeInTheDocument();
    expect(choices('Theme')).toEqual(['System', 'Light', 'Dark']);
    expect(choices('Interface zoom')).toEqual(['90%', '100%', '110%', '125%', '150%']);
    expect(panel()).toHaveTextContent('Also from the keyboard: Ctrl+= to zoom in, Ctrl+- to zoom out, Ctrl+0 to go back to 100%.');
    expect(within(panel()).getByRole('spinbutton', { name: 'Terminal text size' })).toBeInTheDocument();
    // The editor's, folded into « Advanced », named there.
    expect(within(panel()).queryByRole('spinbutton', { name: 'Editor text size' })).toBeNull();
    expect(within(panel()).getByRole('button', { name: /^Advanced/ })).toHaveTextContent('Editor text size');
    expect(within(panel()).getByRole('heading', { name: 'Language' })).toBeInTheDocument();
    expect(choices('Interface language')).toEqual(['System (French)', 'English', 'Français']);
    expect(choices('Language of texts written by the agents')).toEqual(['Same as the interface (default)', 'English', 'Français']);
    expect(panel()).toHaveTextContent(
      'Proposed commit messages and pull request descriptions, comments posted to Jira, Trello, GitHub, Linear, ClickUp, and GitLab, instructions given to agents (by default, the proposed commit message follows the language of the repository’s latest commits).',
    );
  });

  describe('« Apparence »', () => {
    const ZOOM = 'Zoom de l’interface';
    const field = (name: string) => within(panel()).getByRole('spinbutton', { name }) as HTMLInputElement;
    beforeEach(() => Object.assign(zoomState, { fit: 200, shown: 100 }));
    afterEach(() => vi.useRealTimers());

    it('comes first, its zoom in steps, its keys said, the font size of the terminals, the editor’s in « Avancé »', async () => {
      fakeBackend();
      render(SettingsModal, { page: 'general' });
      expect(
        within(panel())
          .getAllByRole('heading')
          .map((h) => h.textContent),
      ).toEqual(['Apparence', 'Langue', 'Mises à jour', 'Toi']);
      expect(within(panel()).queryByRole('spinbutton', { name: 'Taille du texte de l’éditeur' })).toBeNull();
      await userEvent.click(within(panel()).getByRole('button', { name: /^Avancé/ }));
      expect(choices(ZOOM)).toEqual(['90 %', '100 %', '110 %', '125 %', '150 %']);
      expect(choice(ZOOM, '100 %')).toHaveAttribute('aria-pressed', 'true');
      expect(panel()).toHaveTextContent('Aussi au clavier : Ctrl+= pour agrandir, Ctrl+- pour réduire, Ctrl+0 pour revenir à 100 %.');
      expect(field('Taille du texte des terminaux').value).toBe('12.5');
      expect(field('Taille du texte de l’éditeur').value).toBe('12.5');
      expect(panel()).toHaveTextContent('en pixels, 12.5 par défaut');
    });

    it('zooms at once and saves it by itself, without « Enregistrer », the draft left as it is', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      const backend = fakeBackend({ save_settings: () => [] });
      render(SettingsModal, { page: 'general' });
      await userEvent.click(choice(ZOOM, '150 %'));
      expect(choice(ZOOM, '150 %')).toHaveAttribute('aria-pressed', 'true');
      expect(app.settings.uiZoom).toBe(150);
      expect(backend.called('plugin:webview|set_webview_zoom').map((c) => c.args.value)).toEqual([1.5]);
      // Nothing to save in the tab: it is saved already.
      vi.advanceTimersByTime(SAVE_DELAY_MS);
      expect(backend.called('save_settings')).toHaveLength(1);
      expect(backend.called('save_settings').at(-1)!.args.settings.uiZoom).toBe(150);
      // A language saved afterwards keeps the zoom.
      await userEvent.click(choice(UI, 'English'));
      await afterGesture();
      expect(backend.called('save_settings')[1].args.settings).toMatchObject({ language: 'en', uiZoom: 150 });
    });

    it('chooses the theme in segments, Système by default, applied at once and saved by itself, the draft left as it is', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      const backend = fakeBackend({ save_settings: () => [] });
      render(SettingsModal, { page: 'general' });
      expect(choices('Thème')).toEqual(['Système', 'Clair', 'Sombre']);
      expect(choice('Thème', 'Système')).toHaveAttribute('aria-pressed', 'true');
      expect(panel()).toHaveTextContent('Système suit le thème de Windows, macOS ou Linux, et ses changements.');
      await userEvent.click(choice('Thème', 'Clair'));
      expect(choice('Thème', 'Clair')).toHaveAttribute('aria-pressed', 'true');
      expect(app.settings.theme).toBe('light');
      expect(document.documentElement.dataset.theme).toBe('light');
      // The window's own theme, its title bar with it.
      expect(backend.called('plugin:window|set_theme').map((c) => c.args.value)).toEqual(['light']);
      vi.advanceTimersByTime(SAVE_DELAY_MS);
      expect(backend.called('save_settings').at(-1)!.args.settings.theme).toBe('light');
      // Back to the system's: the window's theme is the system's again.
      await userEvent.click(choice('Thème', 'Système'));
      expect(backend.called('plugin:window|set_theme').map((c) => c.args.value)).toEqual(['light', null]);
      // A language saved afterwards keeps the theme.
      await userEvent.click(choice(UI, 'English'));
      await afterGesture();
      expect(backend.called('save_settings').at(-1)?.args.settings).toMatchObject({ language: 'en', theme: 'system' });
    });

    it('does not offer the steps the window cannot hold, and says when the zoom chosen is held to a smaller one', () => {
      fakeBackend();
      Object.assign(zoomState, { fit: 125, shown: 125 });
      render(SettingsModal, { page: 'general' });
      expect(choice(ZOOM, '125 %')).toBeEnabled();
      expect(choice(ZOOM, '150 %')).toBeDisabled();
      // A step out of the segments (the keys reach it) is shown only while it is the one chosen.
      expect(choices(ZOOM)).not.toContain('200 %');
      expect(within(panel()).queryByText(/La fenêtre est trop petite/)).toBeNull();
      app.settings.uiZoom = 200;
      flushSync();
      expect(choices(ZOOM)).toEqual(['90 %', '100 %', '110 %', '125 %', '150 %', '200 %']);
      expect(choice(ZOOM, '200 %')).toBeDisabled();
      expect(choice(ZOOM, '200 %')).toHaveAttribute('aria-pressed', 'true');
      expect(
        within(panel())
          .getByText(/La fenêtre est trop petite/)
          .textContent?.trim(),
      ).toBe('La fenêtre est trop petite pour 200 % : l’interface est à 125 % tant qu’elle ne grandit pas.');
    });

    it('keeps a font size typed in range, to the half pixel', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      const backend = fakeBackend({ save_settings: () => [] });
      render(SettingsModal, { page: 'general' });
      const term = field('Taille du texte des terminaux');
      await userEvent.clear(term);
      await userEvent.type(term, '40');
      await fireEvent.change(term);
      expect(app.settings.terminalFontSize).toBe(24);
      expect(term.value).toBe('24');
      await userEvent.click(within(panel()).getByRole('button', { name: /^Avancé/ }));
      const editor = field('Taille du texte de l’éditeur');
      await userEvent.clear(editor);
      await userEvent.type(editor, '15.2');
      await fireEvent.change(editor);
      expect(app.settings.editorFontSize).toBe(15);
      vi.advanceTimersByTime(SAVE_DELAY_MS);
      expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ terminalFontSize: 24, editorFontSize: 15 });
    });
  });

  describe('« Ton nom » (ESC-243)', () => {
    const NAME = 'Ton nom';
    const field = () => within(panel()).getByRole('textbox', { name: NAME });

    it('is filled with git’s name when none was given, says so, and saves nothing until changed', async () => {
      app.settings = { ...app.settings, userName: '', nameAsked: true };
      const backend = fakeBackend({ git_user_name: () => 'Bob Durand', save_settings: () => [] });
      render(SettingsModal, { page: 'general' });
      await vi.waitFor(() => expect(field()).toHaveValue('Bob Durand'));
      expect(backend.called('git_user_name')).toEqual([{ cmd: 'git_user_name', args: { projectId: null } }]);
      expect(panel()).toHaveTextContent('Repris de git');
      // Git's name stands in by itself: nothing to save.
      expect(settingsForm.changed('general')).toBe(false);
      await userEvent.type(field(), ' Jr');
      expect(panel()).not.toHaveTextContent('Repris de git');
      await afterGesture();
      expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ userName: 'Bob Durand Jr' });
    });

    it('stays empty once emptied on purpose, saved and opened again', async () => {
      app.settings = { ...app.settings, userName: '', nameAsked: true };
      const backend = fakeBackend({ git_user_name: () => 'Bob Durand', save_settings: () => [] });
      const view = render(SettingsModal, { page: 'general' });
      await vi.waitFor(() => expect(field()).toHaveValue('Bob Durand'));
      await userEvent.clear(field());
      expect(panel()).not.toHaveTextContent('Repris de git');
      await afterGesture();
      await vi.waitFor(() => expect(backend.called('save_settings')).toHaveLength(1));
      expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ userName: '', userNameCleared: true });
      expect(field()).toHaveValue('');
      view.unmount();
      render(SettingsModal, { page: 'general' });
      await vi.waitFor(() => expect(backend.called('git_user_name')).toHaveLength(2));
      await new Promise((r) => setTimeout(r, 20));
      expect(field()).toHaveValue('');
      expect(panel()).not.toHaveTextContent('Repris de git');
    });

    it('keeps the name given, git’s aside', async () => {
      app.settings = { ...app.settings, userName: 'Robert' };
      fakeBackend({ git_user_name: () => 'Bob Durand' });
      render(SettingsModal, { page: 'general' });
      await new Promise((r) => setTimeout(r, 20));
      expect(field()).toHaveValue('Robert');
      expect(panel()).not.toHaveTextContent('Repris de git');
    });
  });
});

describe('« Couleurs », the theme of a plugin (PLG-7)', () => {
  const themed = (enabled = true) =>
    ({
      id: 'dev.exemple.hc',
      name: 'Contraste',
      choices: { enabled, projects: {}, revoked: [], settings: {} },
      state: { kind: 'ready' },
      manifest: {
        contributes: {
          themes: [
            {
              id: 'high-contrast',
              name: { fr: 'Contraste élevé', en: 'High contrast' },
              dark: { '--frame': '#000000', '--panel': 'url(x)' },
              projectColors: ['#ff0000', '#00ff00'],
            },
          ],
        },
      },
    }) as unknown as PluginView;

  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'settings', page: 'general' };
    plugins.list = [];
    plugins.loaded = true;
  });
  afterEach(() => {
    plugins.list = [];
    applyPluginTheme(null);
  });

  it('offers the plugins’ way to themes when none is installed', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'general' });
    expect(panel()).toHaveTextContent('Aucun thème installé. Un plugin peut en apporter un : ses couleurs et sa palette de projets.');
    await userEvent.click(within(panel()).getByRole('button', { name: 'Voir les thèmes' }));
    expect(app.modal).toBeNull();
    expect(app.ui.view).toBe('plugins');
    expect(plugins.page).toEqual({ kind: 'category', category: 'themes' });
  });

  it('chooses a plugin’s theme, saved a moment later, and says what it left out', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const backend = fakeBackend();
    plugins.list = [themed()];
    render(SettingsModal, { page: 'general' });
    const select = within(panel()).getByRole('combobox', { name: 'Couleurs' }) as HTMLSelectElement;
    expect([...select.options].map((o) => o.textContent)).toEqual(['Escouade', 'Contraste élevé · Contraste']);
    expect(select.value).toBe('');
    await fireEvent.change(select, { target: { value: 'dev.exemple.hc/high-contrast' } });
    expect(app.settings.pluginTheme).toBe('dev.exemple.hc/high-contrast');
    expect(panel()).toHaveTextContent('Une valeur de ce thème est écartée');
    vi.advanceTimersByTime(SAVE_DELAY_MS + 10);
    expect(backend.called('save_settings').at(-1)?.args.settings).toMatchObject({ pluginTheme: 'dev.exemple.hc/high-contrast' });
    vi.useRealTimers();
  });

  it('says the theme chosen is gone when its plugin is off', () => {
    fakeBackend();
    plugins.list = [themed(false)];
    app.settings.pluginTheme = 'dev.exemple.hc/high-contrast';
    render(SettingsModal, { page: 'general' });
    expect(within(panel()).queryByRole('combobox', { name: 'Couleurs' })).toBeNull();
    expect(panel()).toHaveTextContent(
      'Le thème choisi n’est plus là : son plugin (dev.exemple.hc) est désactivé ou désinstallé. Escouade a repris ses couleurs.',
    );
  });

  it('is put on the window and its palette offered, then taken away', () => {
    const theme = chosenTheme('dev.exemple.hc/high-contrast');
    expect(theme).toBeNull();
    plugins.list = [themed()];
    applyPluginTheme(chosenTheme('dev.exemple.hc/high-contrast'));
    const sheet = document.getElementById('escouade-plugin-theme');
    expect(sheet?.textContent).toBe(":root:root:not([data-theme='light']) {\n  --frame: #000000;\n}\n");
    expect(document.documentElement.dataset.pluginTheme).toBe('dev.exemple.hc/high-contrast');
    expect(projectPalette()).toEqual(['#ff0000', '#00ff00']);
    expect(projectPalette('#123456')).toEqual(['#ff0000', '#00ff00', '#123456']);
    expect(JSON.parse(localStorage.getItem(PLUGIN_THEME_KEY)!).key).toBe('dev.exemple.hc/high-contrast');
    applyPluginTheme(null);
    expect(document.getElementById('escouade-plugin-theme')).toBeNull();
    expect(document.documentElement.dataset.pluginTheme).toBeUndefined();
    expect(projectPalette()).toEqual(PROJECT_COLORS);
    expect(localStorage.getItem(PLUGIN_THEME_KEY)).toBeNull();
  });

  it('is shown before the first render from what the page kept, filtered again', () => {
    localStorage.setItem(
      PLUGIN_THEME_KEY,
      JSON.stringify({ key: 'k', dark: { '--frame': '#111111', '--panel': 'url(x)' }, light: {}, projectColors: ['#abcdef', 'var(--x)'] }),
    );
    earlyPluginTheme();
    expect(document.getElementById('escouade-plugin-theme')?.textContent).toBe(
      ":root:root:not([data-theme='light']) {\n  --frame: #111111;\n}\n",
    );
    expect(projectPalette()).toEqual(['#abcdef']);
  });
});
