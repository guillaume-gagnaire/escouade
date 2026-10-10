import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { Account } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import Composer from './Composer.svelte';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
const TEAM: Account = { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false };

/**
 * An agent that has not started, in a window with these accounts (`current`: the one new agents go to now, the first
 * unless said).
 */
function setup(over: Parameters<typeof agent>[0] = {}, accounts: Account[] = [PRINCIPAL, PRO], current = accounts[0].id) {
  const a = agent({ id: `c${Math.random()}`, status: 'idle', ...over });
  resetApp({ agents: [a] });
  app.settings.accounts = accounts;
  app.usage.current = current;
  const backend = fakeBackend({ get_conversation: () => [], set_agent_account: () => null });
  render(Composer, { agent: app.agents[a.id] });
  return { a, backend };
}

const chip = (name: string | RegExp) => screen.getByRole('button', { name });

describe('the account chip of the Composer', () => {
  beforeEach(() => resetApp());

  it('is there only with several accounts', () => {
    setup({}, [PRINCIPAL]);
    expect(screen.queryByRole('button', { name: /^Compte/ })).not.toBeInTheDocument();
  });

  it('shows the account of the agent beside the model', () => {
    setup({ account: 'pro' });
    const chips = [...chip('Modèle : Opus').closest('.bar')!.querySelectorAll('button[aria-haspopup]')];
    expect(chips.map((b) => b.getAttribute('aria-label')).slice(0, 2)).toEqual(['Modèle : Opus', 'Compte : Pro']);
  });

  it('offers « Automatique » and each active account, the agent’s checked', async () => {
    setup({ account: 'principal' }, [PRINCIPAL, PRO, TEAM]);
    await userEvent.click(chip('Compte : Principal'));
    const items = screen.getAllByRole('menuitemradio');
    // The one switched off is not offered.
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveAccessibleName(/^Automatique \(Principal\)/);
    expect(items[1]).toHaveAccessibleName('Principal');
    expect(items[2]).toHaveAccessibleName('Pro');
    expect(items.map((i) => i.getAttribute('aria-checked'))).toEqual(['false', 'true', 'false']);
  });

  it('puts an agent on the account picked', async () => {
    const { a, backend } = setup({ account: 'principal' });
    await userEvent.click(chip('Compte : Principal'));
    await userEvent.click(screen.getByRole('menuitemradio', { name: 'Pro' }));
    expect(backend.called('set_agent_account').at(-1)?.args).toEqual({ id: a.id, account: 'pro' });
    expect(chip('Compte : Pro')).toBeInTheDocument();
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('gives the agent its account back, and says why, when the backend refuses the change', async () => {
    const { a } = setup({ account: 'principal' });
    const backend = fakeBackend({
      get_conversation: () => [],
      set_agent_account: () => Promise.reject('Le compte « Pro » est désactivé.'),
    });
    await userEvent.click(chip('Compte : Principal'));
    await userEvent.click(screen.getByRole('menuitemradio', { name: 'Pro' }));
    expect(backend.called('set_agent_account')).toHaveLength(1);
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'Le compte « Pro » est désactivé.' }));
    expect(app.agents[a.id].account).toBe('principal');
    expect(chip('Compte : Principal')).toBeInTheDocument();
  });

  it('leaves « Automatique » to the backend, which says which account that is', async () => {
    const { a, backend } = setup({ account: 'pro' });
    await userEvent.click(chip('Compte : Pro'));
    await userEvent.click(screen.getByRole('menuitemradio', { name: /^Automatique/ }));
    expect(backend.called('set_agent_account').at(-1)?.args).toEqual({ id: a.id, account: '' });
    // Until it answers, the agent stays as it is.
    expect(chip('Compte : Pro')).toBeInTheDocument();
  });

  it('names under « Automatique » the account new agents go to now', async () => {
    setup({ account: 'principal' }, [PRINCIPAL, PRO], 'pro');
    await userEvent.click(chip('Compte : Principal'));
    expect(screen.getByRole('menuitemradio', { name: /^Automatique \(Pro\)/ })).toBeInTheDocument();
  });

  it('names under « Automatique » the account the project prefers, when it has one', async () => {
    const a = agent({ id: 'c-pref', account: 'pro', status: 'idle' });
    resetApp({ agents: [a], projects: [project({ account: 'principal' })] });
    app.settings.accounts = [PRINCIPAL, PRO];
    app.usage.current = 'pro';
    fakeBackend({ get_conversation: () => [] });
    render(Composer, { agent: app.agents[a.id] });
    await userEvent.click(chip('Compte : Pro'));
    expect(screen.getByRole('menuitemradio', { name: /^Automatique \(Principal\)/ })).toBeInTheDocument();
  });

  it('keeps an agent’s own account in the menu even when it was switched off since', async () => {
    setup({ account: 'team' }, [PRINCIPAL, PRO, TEAM]);
    await userEvent.click(chip('Compte : Équipe (inactif)'));
    expect(screen.getAllByRole('menuitemradio')).toHaveLength(4);
    expect(screen.getByRole('menuitemradio', { checked: true })).toHaveAccessibleName('Équipe (inactif)');
  });

  it.each([
    ['a session', { sessionId: 's1' }],
    ['a first message', { prompts: 1 }],
    ['an original', { forkOf: 's0' }],
    ['a turn under way', { status: 'running' as const }],
  ])('can no longer be changed once the agent has %s, and says why', async (_, started) => {
    const { backend } = setup({ account: 'pro', ...started });
    const c = chip('Compte : Pro');
    expect(c).toHaveAttribute('aria-disabled', 'true');
    expect(c).toHaveAttribute(
      'title',
      'Le compte d’un agent ne change plus après son premier message (sa conversation est rangée dans ce compte).',
    );
    await userEvent.click(c);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(backend.called('set_agent_account')).toHaveLength(0);
  });

  it('can be changed while the agent has not started', () => {
    setup({ account: 'pro', prompts: 0, sessionId: null });
    expect(chip('Compte : Pro')).not.toHaveAttribute('aria-disabled');
  });

  it('speaks English', async () => {
    setLang('en');
    setup({ account: 'principal' });
    await userEvent.click(chip('Account: Main'));
    expect(screen.getByRole('menuitemradio', { name: /^Automatic \(Main\)/ })).toBeInTheDocument();
  });

  it('says in English that the account no longer changes', () => {
    setLang('en');
    setup({ account: 'pro', sessionId: 's1' });
    expect(chip('Account: Pro')).toHaveAttribute(
      'title',
      'An agent’s account can’t change after its first message (its conversation is filed in that account).',
    );
  });
});
