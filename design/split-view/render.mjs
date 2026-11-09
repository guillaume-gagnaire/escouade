// Captures the previews of the split screen (ESC-392): `node design/split-view/render.mjs [route…]` writes
// <route>.png beside it from apercu.html#<route>, at the app's default size (1440 × 900), and reports a text cut or
// a box wider than its pane.

import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const dir = dirname(fileURLToPath(import.meta.url));
const url = pathToFileURL(join(dir, 'apercu.html')).href;
const ROUTES = ['a-cote', 'a-pile', 'a-glisser', 'b-cote', 'b-pile', 'b-glisser', 'c-cote', 'c-pile', 'c-glisser'];
const only = process.argv.slice(2);

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
let problems = 0;
for (const route of ROUTES.filter((r) => !only.length || only.includes(r))) {
  await page.goto('about:blank');
  await page.goto(`${url}#${route}`);
  await page.evaluate(() => document.fonts.ready);
  const issues = await page.evaluate(() => {
    const found = [];
    for (const el of document.querySelectorAll('.ph, .card .r1, .row, .composer .bar, .tabsbar')) {
      if (el.scrollWidth > el.clientWidth + 1) found.push(`wider than its box: ${el.textContent.trim().replace(/\s+/g, ' ').slice(0, 60)}`);
    }
    return found;
  });
  for (const i of issues) {
    problems++;
    console.log(`${route}: ${i}`);
  }
  await page.screenshot({ path: join(dir, `${route}.png`) });
}
await browser.close();
console.log(problems ? `${problems} problem(s)` : 'no overflow found');
