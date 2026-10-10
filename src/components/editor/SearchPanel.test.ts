import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { FileSearch } from '../../lib/editor/search.svelte';
import { setLang } from '../../lib/i18n';
import type { SearchMatch, SearchResult } from '../../lib/types';
import SearchPanel from './SearchPanel.svelte';

const line = (path: string, n: number, text: string, ranges: [number, number][], offset = 0): SearchMatch => ({
  path,
  line: n,
  col: offset + (ranges[0]?.[0] ?? 0) + 1,
  text,
  offset,
  ranges,
});

const FOUND: SearchResult = {
  matches: [
    line('src/lib/app.ts', 3, '  const total = sum(a);', [[8, 13]]),
    line('src/lib/app.ts', 9, 'return total * total;', [
      [7, 12],
      [15, 20],
    ]),
    line('README.md', 12, 'The total 😀 total.', [
      [4, 9],
      [12, 17],
    ]),
  ],
  truncated: false,
  timedOut: false,
};

/** The panel showing `result` for `text`, its search asking `run`. */
function panel(result: SearchResult | null = FOUND, run = vi.fn(async () => FOUND)) {
  const search = new FileSearch(run, 0);
  search.text = 'total';
  search.result = result;
  const onopen = vi.fn();
  render(SearchPanel, { search, onopen });
  return { search, onopen, run, field: screen.getByRole('textbox', { name: 'Rechercher' }) };
}

const rows = () => screen.getAllByRole('treeitem');
const shown = (el: HTMLElement) => el.textContent?.replace(/\s+/g, ' ').trim();

describe('SearchPanel', () => {
  it('groups the lines found by file, with its folder and how many, and counts them all', () => {
    panel();
    const files = rows().filter((r) => r.getAttribute('aria-level') === '1');
    expect(
      files.map((f) => [
        within(f).getByText(/app\.ts|README/).textContent,
        f.querySelector('.dir')?.textContent ?? '',
        f.querySelector('.count')?.textContent,
      ]),
    ).toEqual([
      ['app.ts', 'src/lib', '2'],
      ['README.md', '', '1'],
    ]);
    expect(files.map((f) => f.getAttribute('aria-expanded'))).toEqual(['true', 'true']);
    expect(rows().map((r) => r.getAttribute('aria-level'))).toEqual(['1', '2', '2', '1', '2']);
    expect(screen.getByRole('status')).toHaveTextContent('3 résultats dans 2 fichiers');
  });

  it('counts one result, and says when there is none', async () => {
    const { search } = panel({ ...FOUND, matches: FOUND.matches.slice(0, 1) });
    expect(screen.getByRole('status')).toHaveTextContent('1 résultat dans 1 fichier');
    search.result = { matches: [], truncated: false, timedOut: false };
    await vi.waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Aucun résultat.'));
    expect(screen.queryAllByRole('treeitem')).toEqual([]);
  });

  it('shows the 2 000 lines of a search cut there, and folds one of its files without drawing the others again', async () => {
    const many = Array.from({ length: 2000 }, (_, i) => line(`src/f${Math.floor(i / 5)}.ts`, i + 1, `  const total${i} = 1;`, [[8, 13]]));
    panel({ matches: many, truncated: true, timedOut: false });
    // Looked up by their label: computing the names of 2 400 rows would take seconds here.
    const row = (label: string) => document.querySelector(`[role="treeitem"][aria-label="${label}"]`);
    expect(rows()).toHaveLength(2400);
    expect(screen.getByRole('status')).toHaveTextContent('2 000 résultats dans 400 fichiers Résultats limités aux 2 000 premiers.');
    const kept = row('Ligne 2000 : const total1999 = 1;');
    await userEvent.click(row('f0.ts, src, 5 résultats') as HTMLElement);
    expect(rows()).toHaveLength(2395);
    expect(row('Ligne 2000 : const total1999 = 1;')).toBe(kept);
  }, 20_000);

  it('shows each line without its indentation, every match marked', () => {
    panel();
    const lines = rows().filter((r) => r.getAttribute('aria-level') === '2');
    expect(lines.map(shown)).toEqual(['const total = sum(a);', 'return total * total;', 'The total 😀 total.']);
    expect(lines.map((l) => [...l.querySelectorAll('mark')].map((m) => m.textContent))).toEqual([
      ['total'],
      ['total', 'total'],
      // After the emoji, one character of the line but two of a JavaScript string.
      ['total', 'total'],
    ]);
  });

  it('says when the results stop at the first 2 000, or when the search ran out of time', async () => {
    const { search } = panel({ ...FOUND, truncated: true });
    expect(screen.getByRole('status')).toHaveTextContent('3 résultats dans 2 fichiers Résultats limités aux 2 000 premiers.');
    search.result = { ...FOUND, truncated: true, timedOut: true };
    await vi.waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Recherche arrêtée après 10 s : résultats partiels.'));
    expect(screen.getByRole('status')).not.toHaveTextContent('Résultats limités');
  });

  it('opens the line clicked, or the one Enter is pressed on', async () => {
    const { onopen } = panel();
    await userEvent.click(screen.getByRole('treeitem', { name: 'Ligne 9 : return total * total;' }));
    expect(onopen).toHaveBeenLastCalledWith(FOUND.matches[1]);
    screen.getByRole('treeitem', { name: 'Ligne 12 : The total 😀 total.' }).focus();
    await userEvent.keyboard('{Enter}');
    expect(onopen).toHaveBeenLastCalledWith(FOUND.matches[2]);
    expect(onopen).toHaveBeenCalledTimes(2);
  });

  it('goes from the field to the results with ↓, between them with ↑ ↓, and folds a file with ← and →', async () => {
    const { field, onopen } = panel();
    field.focus();
    await userEvent.keyboard('{ArrowDown}');
    const app = screen.getByRole('treeitem', { name: /app\.ts/ });
    expect(app).toHaveFocus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(app).toHaveAttribute('aria-expanded', 'false');
    expect(rows()).toHaveLength(3);
    await userEvent.keyboard('{ArrowDown}');
    expect(screen.getByRole('treeitem', { name: /README/ })).toHaveFocus();
    await userEvent.keyboard('{ArrowUp}{ArrowRight}');
    expect(app).toHaveAttribute('aria-expanded', 'true');
    // → on an open file goes to its first line, ← from a line back to its file.
    await userEvent.keyboard('{ArrowRight}');
    expect(screen.getByRole('treeitem', { name: 'Ligne 3 : const total = sum(a);' })).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}{ArrowLeft}');
    expect(app).toHaveFocus();
    expect(app).toHaveAttribute('aria-expanded', 'true');
    await userEvent.keyboard('{End}');
    expect(screen.getByRole('treeitem', { name: 'Ligne 12 : The total 😀 total.' })).toHaveFocus();
    await userEvent.keyboard('{Home}{ArrowUp}');
    expect(field).toHaveFocus();
    // A file row's Enter folds it, it opens nothing.
    app.focus();
    await userEvent.keyboard('{Enter}');
    expect(app).toHaveAttribute('aria-expanded', 'false');
    expect(onopen).not.toHaveBeenCalled();
  });

  it('opens every file again for a new answer', async () => {
    const { search } = panel();
    await userEvent.click(screen.getByRole('treeitem', { name: /app\.ts/ }));
    expect(rows()).toHaveLength(3);
    search.result = { ...FOUND };
    await vi.waitFor(() => expect(rows()).toHaveLength(5));
  });

  it('searches what is typed with the options pressed, and right away with Enter', async () => {
    const run = vi.fn(async () => FOUND);
    const { search, field } = panel(null, run);
    await userEvent.clear(field);
    await userEvent.type(field, 'sum');
    for (const name of ['Respecter la casse', 'Mot entier', 'Expression régulière']) {
      const toggle = screen.getByRole('button', { name });
      expect(toggle).toHaveAttribute('aria-pressed', 'false');
      await userEvent.click(toggle);
      expect(toggle).toHaveAttribute('aria-pressed', 'true');
    }
    expect(search).toMatchObject({ text: 'sum', caseSensitive: true, wholeWord: true, regex: true });
    await vi.waitFor(() =>
      expect(run).toHaveBeenLastCalledWith('project', {
        pattern: 'sum',
        regex: true,
        caseSensitive: true,
        wholeWord: true,
        maxResults: 2000,
      }),
    );
    const calls = run.mock.calls.length;
    field.focus();
    await userEvent.keyboard('{Enter}');
    expect(run).toHaveBeenCalledTimes(calls + 1);
  });

  it('tells under the field why nothing was searched', async () => {
    const { search, field } = panel(null);
    search.error = 'Expression régulière invalide.';
    expect(await screen.findByRole('alert')).toHaveTextContent('Expression régulière invalide.');
    expect(field).toHaveAttribute('aria-invalid', 'true');
    expect(field).toHaveAccessibleDescription('Expression régulière invalide.');
    expect(screen.getByRole('status')).toBeEmptyDOMElement();
  });

  it('says it is searching until the first answer comes, then what it found, in a status there from the start', async () => {
    const { search } = panel(null);
    search.text = '';
    // Always there, empty: a screen reader tells what comes in it only if it was there before.
    const status = await vi.waitFor(() => screen.getByRole('status'));
    expect(status).toBeEmptyDOMElement();
    search.text = 'total';
    search.pending = true;
    await vi.waitFor(() => expect(status).toHaveTextContent('Recherche…'));
    search.result = FOUND;
    search.pending = false;
    await vi.waitFor(() => expect(status).toHaveTextContent('3 résultats dans 2 fichiers'));
    expect(screen.getByRole('status')).toBe(status);
  });
});

describe('SearchPanel in English', () => {
  it('titles the panel and the options, counts the results and names the rows in English', async () => {
    setLang('en');
    const search = new FileSearch(vi.fn(async () => FOUND), 0);
    search.text = 'total';
    search.result = FOUND;
    render(SearchPanel, { search, onopen: vi.fn() });
    expect(screen.getByRole('textbox', { name: 'Search' })).toHaveAttribute('placeholder', 'Search');
    expect(screen.getByText('Search', { selector: '.label' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Match case' })).toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByRole('button', { name: 'Whole word' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Regular expression' })).toBeInTheDocument();
    expect(screen.getByRole('tree', { name: 'Results' })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('3 results in 2 files');
    expect(rows().map((r) => r.getAttribute('aria-label'))).toEqual([
      'app.ts, src/lib, 2 results',
      'Line 3: const total = sum(a);',
      'Line 9: return total * total;',
      'README.md, 1 result',
      'Line 12: The total 😀 total.',
    ]);
    search.result = { ...FOUND, truncated: true };
    await vi.waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Results limited to the first 2,000.'));
    search.result = { matches: [], truncated: false, timedOut: false };
    await vi.waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('No results.'));
  });
});
