// Captures the three directions of ESC-503 beside this file: `node design/onboarding-2-3/render.mjs [prefix]`
// writes captures/<d>-<route>.png (dark, 1440 × 900), <d>-<route>-clair.png (light), and the same at 1024 × 728 with
// `-1024` before the extension. It also says any text that overflows its box, and anything that leaves the window.
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
const sizes = [
  [1440, 900, ''],
  [1024, 728, '-1024'],
];
const browser = await chromium.launch();
let shots = 0;
for (const [width, height, suffix] of sizes) {
  const page = await browser.newPage({ viewport: { width, height } });
  for (const d of ['a', 'b', 'c'])
    for (const route of ['harness', 'modes', 'projet', 'agent', 'decouverte'])
      for (const theme of ['dark', 'light']) {
        if (only && !`${d}-${route}`.startsWith(only)) continue;
        await page.goto('about:blank');
        await page.goto(`${url}?d=${d}${theme === 'light' ? '&theme=light' : ''}#${route}`);
        await page.evaluate(() => document.fonts.ready);
        await page.waitForTimeout(60);
        if (theme === 'dark') {
          const issues = await page.evaluate(() => {
            const sel = '.btn, .pill, h1, h2, h3, .lbl, .nm, .t, .ln, .state, .starter, .chip, .stabs span, .tip, .cmd';
            const over = [...document.querySelectorAll(sel)]
              .filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).textOverflow !== 'ellipsis')
              .map((e) => `déborde : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
            const W = innerWidth, H = innerHeight;
            const out = [...document.querySelectorAll('.tip, .choice, .gstep, .stop, .composer, .perm, .cfoot .btn, .sheet')]
              .filter((e) => {
                const r = e.getBoundingClientRect();
                return r.width && (r.right > W + 0.5 || r.bottom > H + 0.5 || r.left < -0.5 || r.top < -0.5);
              })
              .map((e) => `hors fenêtre : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
            // What a fixed-height box cuts off: content taller than the box that holds it.
            const cut = [...document.querySelectorAll('.guide .gb, .welcome, .cq, .conv, .lanes, .side')]
              .filter((e) => e.scrollHeight > e.clientHeight + 2)
              .map((e) => `coupé : ${e.className} (${e.scrollHeight} > ${e.clientHeight})`);
            return [...over, ...out, ...cut];
          });
          if (issues.length) console.log(`${d}-${route}${suffix}`, issues);
        }
        await page.screenshot({ path: join(out, `${d}-${route}${theme === 'light' ? '-clair' : ''}${suffix}.png`) });
        shots++;
      }
  await page.close();
}
await browser.close();
console.log(`ok · ${shots} captures`);
