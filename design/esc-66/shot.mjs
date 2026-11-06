// Le rendu de l'aperçu : node design/esc-66/shot.mjs
import { chromium } from '@playwright/test';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const browser = await chromium.launch();
for (const [name, theme] of [
  ['apercu', 'dark'],
  ['apercu-clair', 'light'],
]) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(pathToFileURL(path.join(here, 'apercu.html')).href);
  await page.evaluate((t) => (document.documentElement.dataset.theme = t), theme);
  await page.screenshot({ path: path.join(here, `${name}.png`) });
}
await browser.close();
