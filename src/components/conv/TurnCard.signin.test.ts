import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { Account, TurnItem } from '../../lib/types';
import { agent, resetApp } from '../../test/ipc';
import TurnCard from './TurnCard.svelte';

// ESC-162: a turn stopped by a sign-in Claude Code could not refresh.
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
const LOST = 'Failed to authenticate: OAuth session expired and could not be refreshed';

const turn = (over: Partial<TurnItem> = {}): TurnItem => ({
  kind: 'turn',
  id: 'r1',
  ts: 1,
  durationMs: 100,
  cost: 0,
  tokens: 0,
  isError: true,
  interrupted: false,
  error: LOST,
  signIn: true,
  ...over,
});

describe('TurnCard of a turn stopped by a sign-in not refreshed', () => {
  beforeEach(() => {
    resetApp();
    app.settings.accounts = [PRO];
  });

  it('waiting for the account, names it and opens its sign-in', async () => {
    const a = agent({ status: 'error', account: 'pro', signIn: { account: 'pro', stamp: null } });
    render(TurnCard, { item: turn(), agent: a, last: true });
    expect(screen.getByText('Connexion à refaire')).toBeInTheDocument();
    expect(
      screen.getByText(
        'Le compte Pro doit se reconnecter : Claude Code n’a pas pu rafraîchir sa session. L’agent reprendra seul après la connexion.',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(LOST)).toBeInTheDocument();
    // No red card of an error beside it.
    expect(screen.queryByText("Le tour s'est terminé en erreur")).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Se reconnecter' }));
    expect(app.modal).toEqual({ kind: 'account', accountId: 'pro' });
  });

  it('its process started again instead, is a line with no error', () => {
    const a = agent({ status: 'running', account: 'pro', signIn: null });
    render(TurnCard, { item: turn(), agent: a, last: false, latest: true });
    expect(screen.queryByText('Connexion à refaire')).not.toBeInTheDocument();
    expect(screen.queryByText("Le tour s'est terminé en erreur")).not.toBeInTheDocument();
    expect(screen.queryByText(LOST)).not.toBeInTheDocument();
  });

  it('an earlier turn, the agent waiting since a later one, shows no card', () => {
    const a = agent({ status: 'error', account: 'pro', signIn: { account: 'pro', stamp: 'a' } });
    render(TurnCard, { item: turn(), agent: a, last: false, latest: false });
    expect(screen.queryByText('Connexion à refaire')).not.toBeInTheDocument();
  });
});
