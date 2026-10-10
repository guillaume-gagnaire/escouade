import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import Dropdown from './Dropdown.svelte';

const options = [
  { value: 'opus', label: 'Opus' },
  { value: 'sonnet', label: 'Sonnet' },
];
const props = (over: Record<string, unknown> = {}) => ({
  caption: 'Modèle',
  value: 'opus',
  options,
  open: false,
  onToggle: () => {},
  onPick: () => {},
  ...over,
});

describe('Dropdown', () => {
  it('names its trigger after its caption and its value, with the colon of French', () => {
    render(Dropdown, props());
    const trigger = screen.getByRole('button', { name: 'Modèle : Opus' });
    expect(trigger).toHaveAttribute('title', 'Modèle : Opus');
  });

  it('keeps the title it is given', () => {
    render(Dropdown, props({ title: 'Changer de modèle' }));
    expect(screen.getByRole('button', { name: 'Modèle : Opus' })).toHaveAttribute('title', 'Changer de modèle');
  });

  it('names its trigger with the colon of English, and offers its options', async () => {
    setLang('en');
    const onPick = vi.fn();
    render(Dropdown, props({ caption: 'Model', open: true, onPick }));
    expect(screen.getByRole('button', { name: 'Model: Opus' })).toHaveAttribute('title', 'Model: Opus');
    expect(screen.getByRole('menu', { name: 'Model' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('menuitemradio', { name: 'Sonnet' }));
    expect(onPick).toHaveBeenCalledWith('sonnet');
  });
});
