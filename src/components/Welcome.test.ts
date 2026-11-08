import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import { OS } from '../lib/platform';
import { BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, resetApp } from '../test/ipc';
import Welcome from './Welcome.svelte';

const machine = (over: object = {}) => ({ found: true, added: false, connected: false, signsIn: true, email: null, dir: '', ...over });

describe('Welcome', () => {
  beforeEach(() => resetApp());

  it('invites to add a project once a harness and an account are there, which opens the window for it', async () => {
    fakeBackend();
    render(Welcome);
    expect(screen.getByRole('heading', { name: 'Un premier projet' })).toBeInTheDocument();
    expect(screen.getByText(/^Ajoute un projet pour y lancer des agents, suivre/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter un projet…' }));
    expect(app.modal).toEqual({ kind: 'newProject' });
  });

  it('says « Aucun harness trouvé » when none is found, with the command that installs each one, to copy', async () => {
    fakeBackend();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.enginesFound = [];
    render(Welcome);
    const none = screen.getByRole('region', { name: 'Aucun harness trouvé' });
    expect(none).toHaveTextContent(
      'Escouade fait travailler des agents de code. Installe l’un d’eux, puis indique son chemin dans Réglages › Comptes et harness s’il n’est pas dans le PATH.',
    );
    const items = within(none).getAllByRole('listitem');
    const claude = OS === 'windows' ? 'irm https://claude.ai/install.ps1 | iex' : 'curl -fsSL https://claude.ai/install.sh | bash';
    expect(items.map((i) => [i.querySelector('.name')?.textContent, i.querySelector('code')?.textContent])).toEqual([
      ['Claude Code', claude],
      ['Harness nu', 'npm install -g @openai/codex'],
    ]);
    expect(within(items[0]).getByText(/claude\.ai\/install/)).toHaveClass('mono');
    // Its button copies the command, an icon and no character.
    const copy = within(items[0]).getByRole('button', { name: 'Copier la commande de Claude Code' });
    expect(copy.querySelector('[data-icon="import"]')).not.toBeNull();
    // No project to add before a harness: « + » of the rail stays for it.
    expect(screen.queryByRole('button', { name: /Ajouter un projet/ })).not.toBeInTheDocument();
  });

  it('looks for the harnesses again, and opens Réglages › Comptes et harness', async () => {
    const backend = fakeBackend({ claude_found: () => ({ claudeFound: true, claudePathFound: true, enginesFound: ['claude'] }) });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.enginesFound = [];
    render(Welcome);
    expect(screen.getByText('Cherché dans le PATH')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Chercher à nouveau' }));
    expect(backend.called('claude_found')).toHaveLength(1);
    expect(app.enginesFound).toEqual(['claude']);
    expect(screen.queryByRole('region', { name: 'Aucun harness trouvé' })).not.toBeInTheDocument();
    app.enginesFound = [];
    await waitFor(() => expect(screen.getByRole('button', { name: 'Réglages › Comptes et harness…' })).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: 'Réglages › Comptes et harness…' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'accounts' });
  });

  it('says WSL was searched too when a distribution is set', () => {
    fakeBackend();
    app.enginesFound = [];
    app.settings.wslDistro = 'Ubuntu';
    render(Welcome);
    expect(screen.getByText('Cherché dans le PATH et dans WSL (Ubuntu)')).toBeInTheDocument();
  });

  it('leads a first installation, harnesses found and no account, to « Ajouter un compte… », with the machine’s own account (ESC-128, choice 19)', async () => {
    fakeBackend({
      account_machine: ({ engine }: { engine: string }) =>
        engine === 'codex' ? machine({ connected: true, email: 'moi@exemple.dev' }) : machine(),
    });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.enginesFound = ['claude', 'codex'];
    app.settings.accounts = [];
    render(Welcome);
    const none = screen.getByRole('region', { name: 'Aucun compte' });
    expect(none).toHaveTextContent('Ajoute un compte : celui déjà connecté sur cette machine, ou un nouveau.');
    const [claude, codex] = within(none).getAllByRole('listitem');
    await waitFor(() => expect(codex).toHaveTextContent('Connecté sur cette machine · moi@exemple.dev'));
    expect(claude).toHaveTextContent('Installé, pas connecté');
    // The main button is the harness signed in on the machine.
    expect(within(codex).getByRole('button', { name: 'Ajouter un compte…' })).toHaveClass('primary');
    expect(within(claude).getByRole('button', { name: 'Ajouter un compte…' })).not.toHaveClass('primary');
    await userEvent.click(within(codex).getByRole('button', { name: 'Ajouter un compte…' }));
    expect(app.modal).toEqual({ kind: 'account', engine: 'codex' });
  });

  it('says nothing of the accounts once one is added', () => {
    fakeBackend();
    app.enginesFound = ['claude'];
    render(Welcome);
    expect(screen.queryByRole('region', { name: 'Aucun compte' })).not.toBeInTheDocument();
  });

  it('says nothing about it once one is found', () => {
    fakeBackend();
    app.enginesFound = ['claude'];
    render(Welcome);
    expect(screen.queryByText('Aucun harness trouvé')).not.toBeInTheDocument();
  });
});

describe('Welcome in English', () => {
  beforeEach(() => {
    resetApp();
    setLang('en');
  });

  it('writes the warning and its buttons in English', () => {
    fakeBackend();
    app.enginesFound = [];
    render(Welcome);
    expect(screen.getByRole('region', { name: 'No harness found' })).toHaveTextContent(
      'Escouade runs coding agents. Install one of them, then set its path in Settings › Accounts and harnesses if it isn’t on the PATH.',
    );
    expect(screen.getByRole('button', { name: 'Search again' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Settings › Accounts and harnesses…' })).toBeInTheDocument();
  });

  it('writes the invitation in English', () => {
    fakeBackend();
    render(Welcome);
    expect(screen.getByText(/^Add a project to run agents, follow/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add a project…' })).toBeInTheDocument();
  });
});
