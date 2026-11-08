import { screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { errorText, missingAccountOf } from './refusal';
import { app } from './state.svelte';
import { fakeBackend, resetApp } from '../test/ipc';

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
