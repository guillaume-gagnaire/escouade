import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import LongText from './LongText.svelte';

// ESC-290: a tool's output of megabytes is drawn a slice at a time.
describe('LongText', () => {
  beforeEach(() => setLang('fr'));

  it('draws a short text whole, without a button', () => {
    const { container } = render(LongText, { text: 'court', class: 'out' });
    expect(container.querySelector('pre.out')?.textContent).toBe('court');
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('draws 20 000 characters of a 5 MB line, then 20 000 more at each click', async () => {
    const { container } = render(LongText, { text: 'x'.repeat(5_000_000) });
    const pre = container.querySelector('pre')!;
    expect(pre.firstChild?.textContent).toHaveLength(20_000);
    await userEvent.click(screen.getByRole('button', { name: 'Afficher la suite (4980000 caractères)' }));
    expect(pre.firstChild?.textContent).toHaveLength(40_000);
    expect(screen.getByRole('button', { name: 'Afficher la suite (4960000 caractères)' })).toBeInTheDocument();
  });
});
