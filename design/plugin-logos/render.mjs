// Captures the preview of the first-hand plugins' logos beside it (ESC-292): `node design/plugin-logos/render.mjs`
// writes it at 1440×900 (whole page) and on a phone (390×844, `-mobile`), dark then light (`-light`).

import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const dir = dirname(fileURLToPath(import.meta.url));
const url = pathToFileURL(join(dir, 'index.html')).href;
const browser = await chromium.launch();
for (const [suffix, viewport] of [
  ['', { width: 1440, height: 900 }],
  ['-mobile', { width: 390, height: 844 }],
])
  for (const theme of ['dark', 'light']) {
    const page = await browser.newPage({ viewport, deviceScaleFactor: 2 });
    await page.emulateMedia({ reducedMotion: 'reduce', colorScheme: theme });
    await page.goto(`${url}?theme=${theme}`);
    await page.evaluate(() => document.fonts.ready);
    const out = join(dir, `plugin-logos${suffix}${theme === 'light' ? '-light' : ''}.png`);
    await page.screenshot({ path: out, fullPage: true });
    console.log(out);
    await page.close();
  }
await browser.close();
