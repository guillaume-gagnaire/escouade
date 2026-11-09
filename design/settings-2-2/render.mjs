// Captures the three directions of index.html beside it: `node design/settings-2-2/render.mjs [route…]` writes
// captures/<route>.png (dark) and captures/<route>-light.png (light), at the app's default size (1440 × 900).
// It also reports what overflows a window: a row wider than its column, a text cut where no ellipsis is meant.

import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const dir = dirname(fileURLToPath(import.meta.url));
const url = pathToFileURL(join(dir, 'index.html')).href;
const ROUTES = [
  'a-general',
  'a-projet',
  'a-comptes',
  'a-plugins',
  'b-accueil',
  'b-general',
  'b-projet',
  'b-comptes',
  'b-plugins',
  'c-general',
  'c-projet',
  'c-comptes',
  'c-plugins',
];
const only = process.argv.slice(2);
const out = join(dir, 'captures');
mkdirSync(out, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
let problems = 0;
for (const route of ROUTES.filter((r) => !only.length || only.includes(r))) {
  for (const theme of ['dark', 'light']) {
    await page.goto('about:blank');
    await page.goto(`${url}${theme === 'light' ? '?theme=light' : ''}#${route}`);
    await page.evaluate(() => document.fonts.ready);
    const issues = await page.evaluate(() => {
      const found = [];
      const win = document.querySelector('.win');
      if (!win) return ['no window'];
      const wr = win.getBoundingClientRect();
      if (wr.right > innerWidth || wr.bottom > innerHeight) found.push('the window leaves the screen');
      for (const el of win.querySelectorAll('.sr, .item, .b-link, .acc-h, .a-i, .hg')) {
        if (el.scrollWidth > el.clientWidth + 1) found.push(`wider than its box: ${el.textContent.trim().slice(0, 60)}`);
      }
      return found;
    });
    for (const i of issues) {
      problems++;
      console.log(`${route} ${theme}: ${i}`);
    }
    await page.screenshot({ path: join(out, `${route}${theme === 'light' ? '-light' : ''}.png`) });
  }
}
await browser.close();
console.log(problems ? `${problems} problem(s)` : 'no overflow found');
