// Captures the three directions of ESC-661 beside this file: `node design/agent-pages/render.mjs` writes
// captures/<d>-<route>.png (dark, 1440 × 900), <d>-<route>-clair.png (light), and the same at 1024 × 728 with `-1024`
// before the extension. It also says any text that overflows its box and anything that leaves the window.
import { mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(dir, '../../package.json'))('playwright');
const url = pathToFileURL(join(dir, 'maquette.html')).href;
const out = join(dir, 'captures');
mkdirSync(out, { recursive: true });
const sizes = [
  [1440, 900, ''],
  [1024, 728, '-1024'],
];
const browser = await chromium.launch();
let count = 0;
let problems = 0;
for (const [width, height, suffix] of sizes) {
  const page = await browser.newPage({ viewport: { width, height } });
  for (const d of ['a', 'b', 'c'])
    for (const route of ['revue', 'choisi'])
      for (const theme of ['dark', 'light']) {
        await page.goto(`${url}?d=${d}${theme === 'light' ? '&theme=light' : ''}#${route}`);
        await page.reload();
        await page.evaluate(() => document.fonts.ready);
        const found = await page.evaluate(() => {
          const bad = [];
          for (const el of document.querySelectorAll('.ell, .btn, .chip, .drv, .pill, .tag, b, kbd')) {
            const r = el.getBoundingClientRect();
            if (!r.width) continue;
            if (!el.classList.contains('ell') && el.scrollWidth > el.clientWidth + 1 && getComputedStyle(el).overflow !== 'visible')
              bad.push(`overflow: ${el.textContent.trim().slice(0, 40)}`);
            if (r.right > innerWidth + 0.5) bad.push(`leaves the window: ${el.textContent.trim().slice(0, 40)}`);
            if (el.closest('.webview, .msgs')) continue;
            if (!el.classList.contains('ell') && getComputedStyle(el).whiteSpace === 'nowrap') {
              const p = el.parentElement.getBoundingClientRect();
              if (r.right > p.right + 1 && getComputedStyle(el.parentElement).overflow === 'hidden') bad.push(`cut: ${el.textContent.trim().slice(0, 40)}`);
            }
          }
          return bad;
        });
        for (const f of found) console.log(`${d}-${route}-${theme}${suffix}: ${f}`);
        problems += found.length;
        const name = `${d}-${route}${theme === 'light' ? '-clair' : ''}${suffix}.png`;
        await page.screenshot({ path: join(out, name) });
        count++;
      }
  await page.close();
}
await browser.close();
console.log(`${count} captures, ${problems} problems`);
