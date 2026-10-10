import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { ThinkingItem } from '../../lib/types';
import Thinking from './Thinking.svelte';

const item = (over: Partial<ThinkingItem> = {}): ThinkingItem => ({
  kind: 'thinking',
  id: 'k1',
  text: 'Je regarde le routeur',
  streaming: false,
  ...over,
});

describe('Thinking', () => {
  it('unfolds what Claude thought under a heading in the language of the interface', async () => {
    render(Thinking, { item: item() });
    const head = screen.getByRole('button', { name: /Réflexion/ });
    expect(head).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('Je regarde le routeur')).not.toBeInTheDocument();
    await userEvent.click(head);
    expect(screen.getByText('Je regarde le routeur')).toBeInTheDocument();
  });

  it('is headed “Thinking” in English', () => {
    setLang('en');
    render(Thinking, { item: item({ streaming: true, text: '' }) });
    expect(screen.getByRole('button', { name: /Thinking/ })).toBeInTheDocument();
    expect(screen.queryByText(/Réflexion/)).not.toBeInTheDocument();
  });

  it('shows nothing for a thought without text that is not coming', () => {
    const { container } = render(Thinking, { item: item({ text: '  ' }) });
    expect(container.querySelector('.think')).toBeNull();
  });
});
