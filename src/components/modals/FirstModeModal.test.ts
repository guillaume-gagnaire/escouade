import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { accountDefaults } from '../../lib/accounts';
import { modeToAsk } from '../../lib/first-mode';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import FirstModeModal from './FirstModeModal.svelte';

// SEC-09: the first start asks the mode of new agents, the most careful one that keeps Escouade usable recommended.
describe('FirstModeModal', () => {
  beforeEach(() => {
    resetApp();
    app.settings.agentMode = '';
    app.settings.telemetry = null;
  });
  afterEach(cleanup);

  it('is asked to a new installation alone: one that existed has its mode', () => {
    expect(modeToAsk({ agentMode: '' })).toBe(true);
    expect(modeToAsk({ agentMode: 'auto' })).toBe(false);
  });

  it('offers four modes, never Bypass, the recommended one chosen, and runs it until answered', () => {
    fakeBackend();
    render(FirstModeModal);
    const group = within(screen.getByTestId('first-mode'));
    expect(group.getAllByRole('radio').map((r) => r.querySelector('b')!.textContent)).toEqual(['Édits auto', 'Demander', 'Plan', 'Auto']);
    expect(group.getByRole('radio', { name: /Édits auto/ })).toHaveAttribute('aria-checked', 'true');
    expect(group.getByRole('radio', { name: /Édits auto/ })).toHaveTextContent('Recommandé');
    expect(screen.queryByText('Bypass')).toBeNull();
    // Not answered: the recommended mode for the new agents of an account that says none.
    expect(accountDefaults(undefined, 'claude').mode).toBe('acceptEdits');
  });

  it('keeps the mode chosen, then asks about the statistics', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(FirstModeModal);
    await userEvent.click(screen.getByRole('radio', { name: /Demander/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Choisir' }));
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ agentMode: 'default' });
    expect(app.settings.agentMode).toBe('default');
    expect(app.modal).toEqual({ kind: 'telemetry' });
    expect(accountDefaults(undefined, 'claude').mode).toBe('default');
  });
});
