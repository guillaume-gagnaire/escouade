import { screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { errorText, missingAccountOf, missingAccountProjectOf } from './refusal';
import { app } from './state.svelte';
// « Partager à nouveau » loads the accounts' module at its first use (`import()` in `refused`): loaded with this file, what
// is tested is the share, not how long a loaded machine takes to transform that module (more than the 1 s of
// `vi.waitFor` in a full run, ESC-545).
import './accounts';
import { fakeBackend, project, resetApp } from '../test/ipc';

// ESC-128: a refusal for want of an account of a harness comes marked (`commands::err`): the window says it with
// « Ajouter un compte… » on that harness.
const NO_CODEX = 'no-account:codex|Aucun compte Codex : ajoute-en un dans Réglages › Comptes et harness.';

describe('the refusals of the backend', () => {
  beforeEach(() => resetApp());

  it('reads the harness a refusal wants an account of, and says it without its mark', () => {
    expect(missingAccountOf(NO_CODEX)).toBe('codex');
    expect(errorText(NO_CODEX)).toBe('Aucun compte Codex : ajoute-en un dans Réglages › Comptes et harness.');
    // Any other refusal, as it is.
    expect(missingAccountOf('ticket introuvable')).toBeNull();
    expect(errorText('ticket introuvable')).toBe('ticket introuvable');
    // The mark is at the start only: a text that quotes it is not one.
    expect(missingAccountOf('dit « no-account:codex|… »')).toBeNull();
  });

  it('refuses a new agent without an account of its harness with « Ajouter un compte… » on it', async () => {
    fakeBackend({
      create_agent: () => {
        throw NO_CODEX;
      },
    });
    app.ui.activeProject = 'p1';
    await app.newAgent('p1', 'codex');
    expect(app.toasts).toHaveLength(1);
    const [toast] = app.toasts;
    expect(toast.text).toBe('Aucun compte Codex : ajoute-en un dans Réglages › Comptes et harness.');
    expect(toast.kind).toBe('error');
    expect(toast.action?.label).toBe('Ajouter un compte…');
    toast.action?.onClick();
    expect(app.modal).toEqual({ kind: 'account', engine: 'codex' });
    expect(screen.queryByText(/no-account/)).not.toBeInTheDocument();
  });

  // ESC-556: a project no account serves, its account reserved to another: the toast offers « Partager à nouveau ».
  it('reads the project a refusal is for, and offers to share its account again', async () => {
    const away = 'no-account:claude@p1|Aucun compte Claude Code actif ne sert le projet « demo » : …';
    expect(missingAccountOf(away)).toBe('claude');
    expect(missingAccountProjectOf(away)).toBe('p1');
    expect(missingAccountProjectOf(NO_CODEX)).toBeNull();
    expect(errorText(away)).toBe('Aucun compte Claude Code actif ne sert le projet « demo » : …');
    resetApp({ projects: [project({ id: 'p1', name: 'demo' }), project({ id: 'p2', name: 'client' })] });
    app.settings.accounts = [{ id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true, projects: ['p2'] }];
    const b = fakeBackend({ account_update: ({ account }) => [account] });
    app.refused(away);
    const [toast] = app.toasts;
    expect(toast.action?.label).toBe('Partager à nouveau');
    toast.action?.onClick();
    await vi.waitFor(() => expect(b.called('account_update')[0]?.args.account).toMatchObject({ id: 'principal', projects: [] }));
  });

  it('says another refusal in a toast without a button, as before', async () => {
    fakeBackend({
      ticket_start: () => {
        throw 'Ce ticket est déjà parti.';
      },
    });
    await app.run(Promise.reject('Ce ticket est déjà parti.'));
    expect(app.toasts.map((t) => [t.text, t.action])).toEqual([['Ce ticket est déjà parti.', undefined]]);
  });
});
