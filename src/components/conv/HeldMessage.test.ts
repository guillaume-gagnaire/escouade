import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Held } from '../../lib/outbox.svelte';
import { resetApp } from '../../test/ipc';
import HeldMessage from './HeldMessage.svelte';

const held = (over: Partial<Held> = {}): Held => ({
  id: 'held-1',
  agentId: 'a1',
  body: 'Ajoute un test',
  files: [],
  mode: 'delay',
  at: Date.now() + 10_000,
  ...over,
});

describe('HeldMessage', () => {
  beforeEach(() => {
    resetApp();
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());

  it('counts down the seconds before a held message goes', async () => {
    render(HeldMessage, { held: held(), engine: 'claude' });
    expect(screen.getByText('Ajoute un test')).toBeInTheDocument();
    expect(screen.getByText('Envoi dans 10 s')).toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(1_000);
    expect(screen.getByText('Envoi dans 9 s')).toBeInTheDocument();
    for (const name of ['Envoyer maintenant', 'Modifier', 'Annuler']) expect(screen.getByRole('button', { name })).toBeInTheDocument();
  });

  it('offers to send now only a message held for a delay', () => {
    render(HeldMessage, { held: held({ mode: 'turnEnd', at: null }), engine: 'cursor' });
    expect(screen.getByText('Attend la fin du tour')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Envoyer maintenant' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Modifier' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Annuler' })).toBeInTheDocument();
  });

  it('shows the files it carries, as a message sent does', () => {
    const pdf = { name: 'rapport.pdf', mediaType: 'application/pdf', data: '', kind: 'pdf' as const, size: 8 };
    render(HeldMessage, { held: held({ files: [pdf] }), engine: 'claude' });
    expect(screen.getByText('rapport.pdf')).toBeInTheDocument();
  });
});
