// Captures the three directions of ESC-390 beside this file: `node design/kanban-envs-2-2/render.mjs`
// writes captures/<d>-<route>.png (dark) and captures/<d>-<route>-light.png (light), a whole window and the sheet of states.
import { mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(dir, '../../package.json'))('playwright');
const url = pathToFileURL(join(dir, 'index.html')).href;
const out = join(dir, 'captures');
mkdirSync(out, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
for (const d of ['a', 'b', 'c'])
  for (const route of ['fenetre', 'etats'])
    for (const theme of ['dark', 'light']) {
      await page.goto('about:blank');
      await page.goto(`${url}?d=${d}${theme === 'light' ? '&theme=light' : ''}#${route}`);
      await page.evaluate(() => document.fonts.ready);
      const over = await page.evaluate(() =>
        [...document.querySelectorAll('.seg span, .head, .h1, .h2, .tk, .lh, .chip, .cfg, .btn')]
          .filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).textOverflow !== 'ellipsis')
          .map((e) => e.className + ': ' + e.textContent.trim().slice(0, 50)),
      );
      if (over.length && theme === 'dark') console.log(d, route, over);
      await page.screenshot({ path: join(out, `${d}-${route}${theme === 'light' ? '-light' : ''}.png`), fullPage: true });
    }
await browser.close();
console.log('ok');
