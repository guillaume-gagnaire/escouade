import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { treeRows } from '../../lib/editor/tree';
import FileTree from './FileTree.svelte';

const noop = () => {};
const base = { active: null, ontoggle: noop, onopen: noop };

describe('FileTree', () => {
  const rows = treeRows(['src/app.ts', 'README.md'], { src: true }, { 'src/app.ts': 'M' });

  it('opens folders and files', async () => {
    const ontoggle = vi.fn();
    const onopen = vi.fn();
    render(FileTree, { rows, active: 'README.md', ontoggle, onopen });
    await userEvent.click(screen.getByRole('treeitem', { name: /src/, expanded: true }));
    expect(ontoggle).toHaveBeenCalledWith('src');
    await userEvent.click(screen.getByRole('treeitem', { name: /app\.ts/ }));
    expect(onopen).toHaveBeenCalledWith('src/app.ts');
    expect(screen.getByRole('treeitem', { name: /README\.md/ })).toHaveAttribute('aria-selected', 'true');
  });

  it('shows the git status of a file and a dot on its folder', () => {
    render(FileTree, { ...base, rows });
    expect(screen.getByRole('treeitem', { name: /app\.ts/ })).toHaveTextContent('M');
    expect(screen.getByRole('treeitem', { name: /src/ }).querySelector('.dot')).toHaveStyle({ background: 'var(--wait)' });
    expect(screen.getByRole('treeitem', { name: /app\.ts/ }).querySelector('.dot')).toBeNull();
  });

  it('marks each file with its type, each row with its level, and a folder of a single folder with its whole chain', () => {
    render(FileTree, { ...base, rows: treeRows(['src/lib/editor/a.ts', 'src/lib/editor/b.md'], { 'src/lib/editor': true }, {}) });
    expect(screen.getByRole('treeitem', { name: /src\/lib\/editor/ })).toHaveAttribute('aria-level', '1');
    const a = screen.getByRole('treeitem', { name: /a\.ts/ });
    expect(a).toHaveTextContent('TS');
    expect(a).toHaveAttribute('aria-level', '2');
    expect(a.querySelectorAll('.guide')).toHaveLength(1);
    expect(screen.getByRole('treeitem', { name: /b\.md/ })).toHaveTextContent('M↓');
  });
});

describe('FileTree ignored files', () => {
  const rows = treeRows(['.env', 'README.md'], {}, {}, null, ['.env']);

  it('greys out a file git ignores and says so in its title, the others keeping their path', () => {
    render(FileTree, { ...base, rows });
    const env = screen.getByRole('treeitem', { name: /\.env/ });
    expect(env).toHaveAttribute('title', 'Ignoré par git');
    expect(env).toHaveClass('ignored');
    const readme = screen.getByRole('treeitem', { name: /README\.md/ });
    expect(readme).toHaveAttribute('title', 'README.md');
    expect(readme).not.toHaveClass('ignored');
  });

  it('opens it like any other', async () => {
    const onopen = vi.fn();
    render(FileTree, { ...base, rows, onopen });
    await userEvent.click(screen.getByRole('treeitem', { name: /\.env/ }));
    expect(onopen).toHaveBeenCalledWith('.env');
  });
});

describe('FileTree from the keyboard', () => {
  const rows = treeRows(['src/a/x.ts', 'src/app.ts', 'README.md'], { src: true }, {});
  const item = (name: RegExp) => screen.getByRole('treeitem', { name });

  it('is one Tab stop, on the file shown', () => {
    render(FileTree, { ...base, rows, active: 'src/app.ts' });
    expect(screen.getAllByRole('treeitem').filter((r) => r.tabIndex === 0)).toEqual([item(/app\.ts/)]);
  });

  it('hands the focus of a click on the free space to its Tab stop, for the arrows to go on from there', async () => {
    const { container } = render(FileTree, { ...base, rows, active: 'src/app.ts' });
    await userEvent.click(container.querySelector('.tree')!);
    expect(item(/app\.ts/)).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}');
    expect(item(/README/)).toHaveFocus();
  });

  it('moves up and down the rows, to the first and the last', async () => {
    render(FileTree, { ...base, rows });
    item(/src/).focus();
    await userEvent.keyboard('{ArrowDown}');
    expect(item(/^a$/)).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}');
    expect(item(/README/)).toHaveFocus();
    await userEvent.keyboard('{ArrowUp}');
    expect(item(/app\.ts/)).toHaveFocus();
    expect(item(/app\.ts/).tabIndex).toBe(0);
    await userEvent.keyboard('{Home}');
    expect(item(/src/)).toHaveFocus();
    await userEvent.keyboard('{End}');
    expect(item(/README/)).toHaveFocus();
  });

  it('opens a closed folder with →, goes into an open one, closes it with ← and goes up from a file', async () => {
    const ontoggle = vi.fn();
    render(FileTree, { ...base, rows, ontoggle });
    item(/^a$/).focus();
    await userEvent.keyboard('{ArrowRight}');
    expect(ontoggle).toHaveBeenLastCalledWith('src/a');
    item(/src/).focus();
    await userEvent.keyboard('{ArrowRight}');
    expect(item(/^a$/)).toHaveFocus();
    item(/app\.ts/).focus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(item(/src/)).toHaveFocus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(ontoggle).toHaveBeenLastCalledWith('src');
    expect(ontoggle).toHaveBeenCalledTimes(2);
  });
});

describe('FileTree menu', () => {
  it('is asked for on a row, or on the free space below them', async () => {
    const onmenu = vi.fn();
    const { container } = render(FileTree, { ...base, rows: treeRows(['src/a.ts'], { src: true }, {}), onmenu });
    await fireEvent.contextMenu(screen.getByRole('treeitem', { name: /a\.ts/ }));
    expect(onmenu).toHaveBeenLastCalledWith(expect.any(MouseEvent), expect.objectContaining({ kind: 'file', path: 'src/a.ts' }));
    await fireEvent.contextMenu(container.querySelector('.tree')!);
    expect(onmenu).toHaveBeenLastCalledWith(expect.any(MouseEvent), null);
    expect(onmenu).toHaveBeenCalledTimes(2);
  });
});

describe('FileTree naming a new file', () => {
  const rows = treeRows(['src/app.ts'], { src: true }, {}, 'src');
  const check = (name: string) => (name === 'app.ts' ? '« app.ts » existe déjà à cet endroit.' : null);
  const field = () => screen.getByRole('textbox', { name: 'Nom du nouveau fichier' });

  it('takes the focus in its folder and creates the file named on Enter', async () => {
    const oncreate = vi.fn(async () => null);
    render(FileTree, { ...base, rows, check, oncreate, oncancel: noop });
    expect(field()).toHaveFocus();
    await userEvent.keyboard('util.ts{Enter}');
    expect(oncreate).toHaveBeenCalledWith('util.ts');
    expect(oncreate).toHaveBeenCalledTimes(1);
  });

  it('marks the name typed with the type of the file', async () => {
    render(FileTree, { ...base, rows, check, oncreate: vi.fn(), oncancel: noop });
    await userEvent.keyboard('data.json');
    expect(field().closest('.row')).toHaveTextContent('{}');
  });

  it('tells why a name cannot be taken and keeps the field until it can', async () => {
    const oncreate = vi.fn(async () => null);
    render(FileTree, { ...base, rows, check, oncreate, oncancel: noop });
    await userEvent.keyboard('app.ts');
    expect(screen.getByRole('alert')).toHaveTextContent('« app.ts » existe déjà à cet endroit.');
    expect(field()).toHaveAttribute('aria-invalid', 'true');
    await userEvent.keyboard('{Enter}');
    expect(oncreate).not.toHaveBeenCalled();
    await userEvent.keyboard('{Backspace}{Backspace}x');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('shows what refused the creation, until the name changes, and tries it again on Enter', async () => {
    const oncreate = vi.fn().mockResolvedValueOnce('Fichier verrouillé').mockResolvedValueOnce(null);
    render(FileTree, { ...base, rows, check, oncreate, oncancel: noop });
    await userEvent.keyboard('b.ts{Enter}');
    expect(await screen.findByRole('alert')).toHaveTextContent('Fichier verrouillé');
    expect(field()).toBeInTheDocument();
    await userEvent.keyboard('{Enter}');
    expect(oncreate).toHaveBeenCalledTimes(2);
    await userEvent.keyboard('x');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  /** The field loses the focus to the page; what it does then is decided once the blur is over. */
  const leave = async () => {
    field().blur();
    await tick();
  };

  it('is given up with Escape, or when it loses the focus empty or wrong', async () => {
    const oncancel = vi.fn();
    const oncreate = vi.fn(async () => null);
    const { unmount } = render(FileTree, { ...base, rows, check, oncreate, oncancel });
    await userEvent.keyboard('b.ts{Escape}');
    expect(oncancel).toHaveBeenCalledTimes(1);
    unmount();
    render(FileTree, { ...base, rows, check, oncreate, oncancel });
    await leave();
    expect(oncancel).toHaveBeenCalledTimes(2);
    field().focus();
    await userEvent.keyboard('app.ts');
    await leave();
    expect(oncancel).toHaveBeenCalledTimes(3);
    expect(oncreate).not.toHaveBeenCalled();
  });

  it('decides nothing more from the focus it loses once given up or created', async () => {
    const oncreate = vi.fn(async () => null);
    const oncancel = vi.fn();
    const { unmount } = render(FileTree, { ...base, rows, check, oncreate, oncancel });
    await userEvent.keyboard('b.ts{Escape}');
    await leave();
    expect(oncreate).not.toHaveBeenCalled();
    expect(oncancel).toHaveBeenCalledTimes(1);
    field().focus();
    await userEvent.clear(field());
    await leave();
    expect(oncancel).toHaveBeenCalledTimes(1);
    unmount();
    render(FileTree, { ...base, rows, check, oncreate, oncancel });
    await userEvent.keyboard('c.ts{Enter}');
    await leave();
    expect(oncreate).toHaveBeenCalledTimes(1);
    expect(oncancel).toHaveBeenCalledTimes(1);
  });

  it('creates the file named when it loses the focus, as VS Code does', async () => {
    const oncreate = vi.fn(async () => null);
    const oncancel = vi.fn();
    render(FileTree, { ...base, rows, check, oncreate, oncancel });
    await userEvent.keyboard('b.ts');
    await leave();
    expect(oncreate).toHaveBeenCalledWith('b.ts');
    expect(oncancel).not.toHaveBeenCalled();
  });

  it('takes the focus back to show what refused a file created as it lost it, and is given up when it loses it again', async () => {
    const oncreate = vi.fn(async () => 'Accès refusé');
    const oncancel = vi.fn();
    render(FileTree, { ...base, rows, check, oncreate, oncancel });
    await userEvent.keyboard('b.ts');
    await leave();
    expect(await screen.findByRole('alert')).toHaveTextContent('Accès refusé');
    expect(field()).toHaveFocus();
    await leave();
    expect(oncancel).toHaveBeenCalledTimes(1);
    expect(oncreate).toHaveBeenCalledTimes(1);
  });

  it('creates nothing when it is taken away from under the focus', async () => {
    const oncreate = vi.fn(async () => null);
    const oncancel = vi.fn();
    render(FileTree, { ...base, rows, check, oncreate, oncancel });
    await userEvent.keyboard('b.ts');
    // As Chromium does: a focused element taken out of the page gets a blur first.
    const input = field();
    input.blur();
    input.remove();
    await tick();
    expect(oncreate).not.toHaveBeenCalled();
    expect(oncancel).not.toHaveBeenCalled();
  });

  it('waits when the window loses the focus, as the field keeps it to get it back with the window', async () => {
    const oncreate = vi.fn(async () => null);
    const oncancel = vi.fn();
    render(FileTree, { ...base, rows, check, oncreate, oncancel });
    await userEvent.keyboard('comp');
    // The window is left: the field gets a blur but stays the page's focused element.
    field().dispatchEvent(new FocusEvent('blur'));
    await tick();
    expect(field()).toHaveFocus();
    expect(oncreate).not.toHaveBeenCalled();
    expect(oncancel).not.toHaveBeenCalled();
  });
});

describe('FileTree and the file shown', () => {
  afterEach(() => vi.restoreAllMocks());

  it('brings its row into view', async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    const rows = treeRows(['a.ts', 'b.ts'], {}, {});
    const { rerender } = render(FileTree, { ...base, rows, active: 'a.ts' });
    await rerender({ ...base, rows, active: 'b.ts' });
    expect(scroll).toHaveBeenLastCalledWith({ block: 'nearest' });
    expect(scroll.mock.contexts.at(-1)).toBe(screen.getByRole('treeitem', { name: /b\.ts/ }));
  });
});
