import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import EditorTabs from './EditorTabs.svelte';

describe('EditorTabs', () => {
  const tabs = [
    { path: 'src/a.ts', name: 'a.ts', dirty: true, status: 'M' as const, active: true },
    { path: 'b.ts', name: 'b.ts', dirty: false, status: null, active: false },
  ];

  it('selects and closes tabs, the unsaved one marked', async () => {
    const onselect = vi.fn();
    const onclose = vi.fn();
    render(EditorTabs, { tabs, onselect, onclose });
    expect(screen.getByRole('tab', { name: /a\.ts/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('button', { name: 'Fermer a.ts' })).toHaveClass('dirty');
    await userEvent.click(screen.getByRole('tab', { name: /b\.ts/ }));
    expect(onselect).toHaveBeenCalledWith('b.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Fermer b.ts' }));
    expect(onclose).toHaveBeenCalledWith('b.ts');
    expect(onselect).toHaveBeenCalledTimes(1);
  });

  it('closes a background tab from the keyboard without selecting it', async () => {
    const onselect = vi.fn();
    const onclose = vi.fn();
    render(EditorTabs, { tabs, onselect, onclose });
    screen.getByRole('button', { name: 'Fermer b.ts' }).focus();
    await userEvent.keyboard('{Enter}');
    expect(onclose).toHaveBeenCalledWith('b.ts');
    expect(onselect).not.toHaveBeenCalled();
  });

  it('still selects a tab with Enter when the tab itself has the focus', async () => {
    const onselect = vi.fn();
    render(EditorTabs, { tabs, onselect, onclose: () => {} });
    screen.getByRole('tab', { name: /b\.ts/ }).focus();
    await userEvent.keyboard('{Enter}');
    expect(onselect).toHaveBeenCalledWith('b.ts');
  });

  it('shows no folder next to names that are alone', () => {
    const { container } = render(EditorTabs, { tabs, onselect: () => {}, onclose: () => {} });
    expect(container.querySelector('.hint')).toBeNull();
    expect(screen.getByRole('tab', { name: /a\.ts/ })).toHaveAttribute('title', 'src/a.ts');
  });

  describe('with two files of the same name', () => {
    const twins = [
      { path: 'src/components/editor/index.ts', name: 'index.ts', dirty: false, status: null, active: true },
      { path: 'src/components/board/index.ts', name: 'index.ts', dirty: true, status: null, active: false },
      { path: 'src/main.ts', name: 'main.ts', dirty: false, status: null, active: false },
    ];

    it('tells them apart by their folder, in grey, with the full path as the title', () => {
      render(EditorTabs, { tabs: twins, onselect: () => {}, onclose: () => {} });
      const editor = screen.getByRole('tab', { name: /index\.ts · editor/ });
      const board = screen.getByRole('tab', { name: /index\.ts · board/ });
      expect(editor).toHaveAttribute('title', 'src/components/editor/index.ts');
      expect(board).toHaveAttribute('title', 'src/components/board/index.ts');
      expect(within(editor).getByText('editor', { exact: false })).toHaveClass('hint');
      expect(screen.getByRole('tab', { name: /main\.ts/ })).not.toHaveTextContent('·');
    });

    it('names the right tab on each close button', async () => {
      const onclose = vi.fn();
      render(EditorTabs, { tabs: twins, onselect: () => {}, onclose });
      await userEvent.click(screen.getByRole('button', { name: 'Fermer index.ts · board' }));
      expect(onclose).toHaveBeenCalledWith('src/components/board/index.ts');
    });

    it('takes the folder back once the other file is closed', async () => {
      const { rerender } = render(EditorTabs, { tabs: twins, onselect: () => {}, onclose: () => {} });
      expect(screen.getAllByRole('tab', { name: /index\.ts · / })).toHaveLength(2);
      await rerender({ tabs: twins.slice(0, 1), onselect: () => {}, onclose: () => {} });
      expect(screen.getByRole('tab', { name: /index\.ts/ })).not.toHaveTextContent('·');
    });
  });
});
