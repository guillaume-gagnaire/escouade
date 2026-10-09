import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { NavTarget } from '../../lib/editor/goto';
import TargetPicker from './TargetPicker.svelte';

describe('TargetPicker', () => {
  const targets: NavTarget[] = [
    { path: 'src/ui/app.ts', line: 9, col: 10, text: '  export function render() {}' },
    { path: 'src/ui/near.ts', line: 5, col: 14, text: 'export const render = () => 1;' },
    { path: 'lib/far.ts', line: 3, col: 17, text: 'export function render() {}' },
  ];

  function open(over: Partial<{ targets: NavTarget[]; onpick: (t: NavTarget) => void; onclose: () => void }> = {}) {
    const onpick = vi.fn();
    const onclose = vi.fn();
    render(TargetPicker, { targets, label: 'render', at: { left: 40, top: 100, bottom: 118 }, onpick, onclose, ...over });
    return { onpick, onclose, list: screen.getByRole('listbox', { name: 'Définitions de « render »' }) };
  }

  it('lists each place as path:line with a preview of its line, the first one chosen, the focus on the list', () => {
    const { list } = open();
    const options = screen.getAllByRole('option');
    expect(options.map((o) => o.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'src/ui/app.ts:9 export function render() {}',
      'src/ui/near.ts:5 export const render = () => 1;',
      'lib/far.ts:3 export function render() {}',
    ]);
    expect(options[0]).toHaveAttribute('aria-selected', 'true');
    expect(list).toHaveFocus();
    expect(list).toHaveAttribute('aria-activedescendant', options[0].id);
  });

  it('shows at most 50 places', () => {
    const many = Array.from({ length: 70 }, (_, i) => ({ path: `src/f${i}.ts`, line: 1, col: 1, text: 'x' }));
    open({ targets: many });
    expect(screen.getAllByRole('option')).toHaveLength(50);
  });

  it('moves with ↑ and ↓, round the list, and takes the one chosen with Enter', async () => {
    const { onpick } = open();
    await userEvent.keyboard('{ArrowDown}{ArrowDown}');
    expect(screen.getAllByRole('option')[2]).toHaveAttribute('aria-selected', 'true');
    await userEvent.keyboard('{ArrowDown}');
    expect(screen.getAllByRole('option')[0]).toHaveAttribute('aria-selected', 'true');
    await userEvent.keyboard('{ArrowUp}');
    expect(screen.getAllByRole('option')[2]).toHaveAttribute('aria-selected', 'true');
    await userEvent.keyboard('{ArrowUp}{Enter}');
    expect(onpick).toHaveBeenCalledExactlyOnceWith(targets[1]);
  });

  it('closes with Escape, the focus back where it was, and when clicked away', async () => {
    const before = document.createElement('button');
    document.body.append(before);
    before.focus();
    const { onclose, onpick } = open();
    await userEvent.keyboard('{Escape}');
    expect(onclose).toHaveBeenCalledOnce();
    expect(before).toHaveFocus();
    await userEvent.click(before);
    expect(onclose).toHaveBeenCalledTimes(2);
    expect(onpick).not.toHaveBeenCalled();
    before.remove();
  });

  it('takes the place clicked', async () => {
    const { onpick, onclose } = open();
    await userEvent.click(screen.getByRole('option', { name: /lib\/far\.ts:3/ }));
    expect(onpick).toHaveBeenCalledExactlyOnceWith(targets[2]);
    expect(onclose).not.toHaveBeenCalled();
  });
});
