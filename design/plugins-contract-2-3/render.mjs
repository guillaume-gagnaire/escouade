// Captures the mockups of ESC-501 beside this file: `node design/plugins-contract-2-3/render.mjs` writes
// captures/<route>.png (dark, 1440 × 900) and captures/<route>-clair.png (light). It also says any text that overflows
// its box and anything that leaves the window.
import { mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(dir, '../../package.json'))('playwright');
const url = pathToFileURL(join(dir, 'maquette.html')).href;
const out = join(dir, 'captures');
mkdirSync(out, { recursive: true });
const routes = ['a-plugin', 'a-projet', 'b-plugin', 'b-profil', 'b-projet', 'onglet', 'install'];
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
let problems = 0;
for (const route of routes)
  for (const theme of ['dark', 'light']) {
    await page.goto('about:blank');
    await page.goto(`${url}${theme === 'light' ? '?theme=light' : ''}#${route}`);
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(60);
    const issues = await page.evaluate(() => {
      const over = [...document.querySelectorAll('.btn, .pill, .field, h1, h2, h3, h4, .chip, .tabs span, .cmd')]
        .filter((e) => e.scrollWidth > e.clientWidth + 1)
        .map((e) => `déborde : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
      const cut = [...document.querySelectorAll('.db, .sbody, .pbody > div:first-child')]
        .filter((e) => e.scrollHeight > e.clientHeight + 2)
        .map((e) => `coupé : ${e.className} (${e.scrollHeight} > ${e.clientHeight})`);
      const outside = [...document.querySelectorAll('.dialog, .btn')]
        .filter((e) => {
          const r = e.getBoundingClientRect();
          return r.width && (r.right > innerWidth + 0.5 || r.bottom > innerHeight + 0.5);
        })
        .map((e) => `hors fenêtre : ${e.className}`);
      return [...over, ...cut, ...outside];
    });
    if (issues.length) {
      problems += issues.length;
      console.log(`${route} ${theme}`, issues);
    }
    await page.screenshot({ path: join(out, `${route}${theme === 'light' ? '-clair' : ''}.png`) });
  }
await browser.close();
console.log(`ok · ${routes.length * 2} captures · ${problems} problème(s)`);
