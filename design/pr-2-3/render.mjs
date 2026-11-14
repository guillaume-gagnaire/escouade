// Captures the three directions of ESC-449 beside this file: `node design/pr-2-3/render.mjs`
// writes captures/<d>-<route>.png (dark) and captures/<d>-<route>-clair.png (light): the window (fenetre), the control
// of the PR (controle) and the sheet of the four states (etats). It also says any text that overflows its box.
import { mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(dir, '../../package.json'))('playwright');
const url = pathToFileURL(join(dir, 'maquette.html')).href;
const out = join(dir, 'captures');
mkdirSync(out, { recursive: true });
const only = process.argv[2];
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
for (const d of ['a', 'b', 'c'])
  for (const route of ['fenetre', 'controle', 'etats'])
    for (const theme of ['dark', 'light']) {
      if (only && !`${d}-${route}`.startsWith(only)) continue;
      await page.goto('about:blank');
      await page.goto(`${url}?d=${d}${theme === 'light' ? '&theme=light' : ''}#${route}`);
      await page.evaluate(() => document.fonts.ready);
      const over = await page.evaluate(() =>
        [...document.querySelectorAll('.btn, .pill, .ci, .lbl, h1, h2, .tt, .who, .stp .v, .rh, .fh, .tabs span, .ptabs span, .dt span, .f')]
          .filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).textOverflow !== 'ellipsis')
          .map((e) => e.className + ': ' + e.textContent.trim().slice(0, 50)),
      );
      if (over.length && theme === 'dark') console.log(d, route, 'déborde :', over);
      await page.screenshot({ path: join(out, `${d}-${route}${theme === 'light' ? '-clair' : ''}.png`), fullPage: true });
    }
await browser.close();
console.log('ok');
