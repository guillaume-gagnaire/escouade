import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import { fakeBackend, resetApp } from '../test/ipc';
import Welcome from './Welcome.svelte';

describe('Welcome', () => {
  beforeEach(() => resetApp());

  it('invites to add a project, which opens the window for it', async () => {
    fakeBackend();
    render(Welcome);
    expect(screen.getByText(/^Ajoute un projet pour y lancer des agents Claude Code/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: '+ Ajouter un projet' }));
    expect(app.modal).toEqual({ kind: 'newProject' });
  });

  it('says where Claude Code can be installed from when it is not found, the command in code style', () => {
    fakeBackend();
    app.claudeFound = false;
    render(Welcome);
    const warn = screen.getByText(/Claude Code est introuvable sur ce poste/);
    expect(warn).toHaveTextContent(
      'Claude Code est introuvable sur ce poste. Installe-le (irm https://claude.ai/install.ps1 | iex) ou indique son chemin dans les réglages.',
    );
    expect(screen.getByText('irm https://claude.ai/install.ps1 | iex')).toHaveClass('mono');
  });

  it('says nothing about it when it is found', () => {
    fakeBackend();
    app.claudeFound = true;
    render(Welcome);
    expect(screen.queryByText(/introuvable/)).not.toBeInTheDocument();
  });
});

describe('Welcome in English', () => {
  beforeEach(() => {
    resetApp();
    setLang('en');
  });

  it('writes the invitation, the warning and the button in English', () => {
    fakeBackend();
    app.claudeFound = false;
    render(Welcome);
    expect(screen.getByText(/^Add a project to run Claude Code agents in it/)).toBeInTheDocument();
    expect(screen.getByText(/Claude Code couldn’t be found on this machine/)).toHaveTextContent(
      'Claude Code couldn’t be found on this machine. Install it (irm https://claude.ai/install.ps1 | iex) or set its path in the settings.',
    );
    expect(screen.getByRole('button', { name: '+ Add a project' })).toBeInTheDocument();
  });
});
