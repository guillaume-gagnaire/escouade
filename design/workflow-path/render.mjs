// Captures the three directions of ESC-406 beside this file: `node design/workflow-path/render.mjs` writes
// captures/<d>-colonne.png and captures/<d>-grand.png (dark), and their `-light` twins. It says what sticks out.
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
const page = await browser.newPage({ viewport: { width: 1180, height: 800 }, deviceScaleFactor: 2 });
for (const d of ['a', 'b', 'c'])
  for (const route of ['colonne', 'grand'])
    for (const theme of ['dark', 'light']) {
      await page.goto('about:blank');
      await page.goto(`${url}?d=${d}${theme === 'light' ? '&theme=light' : ''}#${route}`);
      await page.waitForSelector('body[data-ready]');
      // Nothing of a drawing out of its frame.
      const over = await page.evaluate(() =>
        [...document.querySelectorAll('.col, .big')].flatMap((f) => {
          const box = f.getBoundingClientRect();
          return [...f.querySelectorAll('rect, text, .c, .card, li')]
            .filter((e) => e.getBoundingClientRect().right > box.right + 0.5 || e.getBoundingClientRect().left < box.left - 0.5)
            .map((e) => `${e.tagName} ${e.textContent.trim().slice(0, 40)}`);
        }),
      );
      if (over.length && theme === 'dark') console.log(d, route, over);
      await page.screenshot({ path: join(out, `${d}-${route}${theme === 'light' ? '-light' : ''}.png`), fullPage: true });
    }
await browser.close();
console.log('ok');
