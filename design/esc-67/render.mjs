// Captures the three directions of ESC-67's view « Environnements »: `node design/esc-67/render.mjs` (Playwright of the
// app's node_modules), one 1440×900 picture per direction (a.png, b.png, c.png) beside this file.

import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(dir, '../../package.json'))('playwright');
const url = pathToFileURL(join(dir, 'index.html')).href;
const browser = await chromium.launch();
for (const d of ['a', 'b', 'c']) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto(`${url}?d=${d}`);
  await page.evaluate(() => document.fonts.ready);
  const out = join(dir, `${d}.png`);
  await page.screenshot({ path: out });
  console.log(out);
  await page.close();
}
await browser.close();
