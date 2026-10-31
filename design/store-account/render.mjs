// Captures the three directions of ESC-318 beside them: `node design/store-account/render.mjs [name…]` (Playwright from
// store/node_modules). For each direction a, b, c: the list with its « ⋯ » (`-list`, 1440×900), the modal of a
// published plugin (`-modal`, 1440×900) and, on a phone (390×844), the modal with the deletion being confirmed (`-mobile`).

import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(dir, '../../store/package.json'))('playwright');
const url = pathToFileURL(join(dir, 'index.html')).href;
const shots = ['a', 'b', 'c'].flatMap((d) => [
  { name: `${d}-list`, query: `d=${d}&modal=0`, width: 1440, height: 900 },
  { name: `${d}-modal`, query: `d=${d}`, width: 1440, height: 900 },
  { name: `${d}-mobile`, query: `d=${d}&step=delete`, width: 390, height: 844 },
]);
const only = process.argv.slice(2);
const errors = [];
const browser = await chromium.launch();
for (const s of shots.filter((x) => !only.length || only.includes(x.name))) {
  const page = await browser.newPage({ viewport: { width: s.width, height: s.height }, deviceScaleFactor: s.width < 500 ? 2 : 1 });
  page.on('pageerror', (e) => errors.push(`${s.name}: ${e.message}`));
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${url}?${s.query}`);
  await page.evaluate(() => document.fonts.ready);
  // On a phone, the deletion being confirmed is shown: the modal's body scrolled to its end.
  if (s.query.includes('step=delete')) await page.evaluate(() => document.querySelector('.dlg-body')?.scrollTo(0, 1e5));
  const out = join(dir, `${s.name}.png`);
  await page.screenshot({ path: out });
  console.log(out);
  await page.close();
}
await browser.close();
if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
