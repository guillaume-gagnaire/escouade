import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

const panel = () => screen.getByRole('tabpanel');
/** The choices of a setting, as its chips read. */
const choices = (name: string) =>
  within(within(panel()).getByRole('group', { name }))
    .getAllByRole('button')
    .map((b) => b.textContent?.trim());
const choice = (setting: string, name: string) =>
  within(within(panel()).getByRole('group', { name: setting })).getByRole('button', { name });

const UI = 'Langue de l’interface';
const CLAUDE = 'Langue des textes rédigés par Claude';

describe('the « Application » tab', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'settings', tab: 'app' };
  });

  it('comes first and chooses the languages, each named in itself', () => {
    fakeBackend();
    render(SettingsModal, { tab: 'app' });
    expect(screen.getAllByRole('tab')[0]).toHaveTextContent('Application');
    expect(screen.getByRole('tab', { name: 'Application' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('Langue de l’interface et des textes rédigés par Claude')).toBeInTheDocument();
    expect(within(panel()).getByRole('heading', { name: 'Langue' })).toBeInTheDocument();
    expect(choices(UI)).toEqual(['Système (Français)', 'English', 'Français']);
    expect(choices(CLAUDE)).toEqual(['Comme l’interface (par défaut)', 'English', 'Français']);
    expect(panel()).toHaveTextContent(
      'Messages de commit et descriptions de pull request proposés, commentaires publiés dans Jira, Trello et GitHub, consignes données aux agents (par défaut, le message de commit proposé reprend la langue des derniers commits du dépôt).',
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
    render(SettingsModal, { tab: 'app' });
    expect(choices(UI)[0]).toBe('Système (Anglais)');
    setLang('en');
    flushSync();
    expect(choices('Interface language')[0]).toBe('System (English)');
    app.lang = { ui: 'en', system: 'fr', claude: 'en' };
    flushSync();
    expect(choices('Interface language')[0]).toBe('System (French)');
  });

  it('saves the languages chosen with the other settings', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { tab: 'app' });
    expect(screen.getByRole('tab', { name: 'Application' })).not.toHaveClass('changed');
    await userEvent.click(choice(UI, 'English'));
    await userEvent.click(choice(CLAUDE, 'Français'));
    expect(choice(UI, 'English')).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('tab', { name: 'Application' })).toHaveClass('changed');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ language: 'en', claudeLanguage: 'fr' });
    expect(app.settings).toMatchObject({ language: 'en', claudeLanguage: 'fr' });
  });

  it('reads in English', () => {
    fakeBackend();
    setLang('en');
    render(SettingsModal, { tab: 'app' });
    expect(screen.getByRole('tab', { name: 'Application' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('Language of the interface and of texts written by Claude')).toBeInTheDocument();
    expect(within(panel()).getByRole('heading', { name: 'Language' })).toBeInTheDocument();
    expect(choices('Interface language')).toEqual(['System (French)', 'English', 'Français']);
    expect(choices('Language of texts written by Claude')).toEqual(['Same as the interface (default)', 'English', 'Français']);
    expect(panel()).toHaveTextContent(
      'Proposed commit messages and pull request descriptions, comments posted to Jira, Trello, and GitHub, instructions given to agents (by default, the proposed commit message follows the language of the repository’s latest commits).',
    );
  });
});
