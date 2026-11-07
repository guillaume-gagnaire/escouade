// The theme's promise, checked on its manifest alone: every text and status color stands out on every surface it is
// drawn on, in dark and in light (WCAG 2.2 ratios). `node --test` in this folder (it reads its manifest there), no dependency.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const manifest = JSON.parse(readFileSync('escouade-plugin.json', 'utf8'));
const [theme] = manifest.contributes.themes;
const SURFACES = ['--frame', '--raise', '--raise-2', '--well'];

/** The relative luminance of a `#rrggbb` color. */
function luminance(hex) {
  const [r, g, b] = [1, 3, 5]
    .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
const ratio = (a, b) => {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

/** The lowest ratio of `variable` on the surfaces, in both modes. */
const lowest = (variable) =>
  Math.min(...['dark', 'light'].flatMap((mode) => SURFACES.map((s) => ratio(theme[mode][variable], theme[mode][s]))));

test('the text is AAA on every surface (7:1)', () => {
  for (const v of ['--text', '--soft', '--muted', '--dim']) assert.ok(lowest(v) >= 7, `${v}: ${lowest(v).toFixed(2)}`);
});

test('the accent and the status colors are AA on every surface (4.5:1)', () => {
  for (const v of ['--p-ink', '--ring', '--run', '--wait', '--done', '--add', '--del'])
    assert.ok(lowest(v) >= 4.5, `${v}: ${lowest(v).toFixed(2)}`);
});

test('the borders are seen (3:1, the ratio of what is not text)', () => {
  for (const v of ['--line', '--line-2', '--line-3']) assert.ok(lowest(v) >= 3, `${v}: ${lowest(v).toFixed(2)}`);
});

test('both modes set the same variables, and six project colors', () => {
  assert.deepEqual(Object.keys(theme.dark).sort(), Object.keys(theme.light).sort());
  assert.equal(theme.projectColors.length, 6);
  for (const c of theme.projectColors) assert.match(c, /^#[0-9a-f]{6}$/);
});
