// Captures the four directions of ESC-508 beside this file: `node design/inbox-2-3/render.mjs [prefix]` writes
// captures/<d>-<route>.png (dark, 1440 × 900), <d>-<route>-clair.png (light), and the same at 1024 × 728 with `-1024`
// before the extension; then x-eteint and x-eteindre (the MCP server off), common to the four. It also says any text
// that overflows its box, anything that leaves the window, and anything a fixed-height box cuts off.
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
const routes = ['pleine', 'choix', 'pdf', 'validation', 'tache', 'vide', 'compteur'];
const shots = [...['a', 'b', 'c', 'd'].flatMap((d) => routes.map((r) => [d, r])), ['x', 'eteint'], ['x', 'eteindre']];
const browser = await chromium.launch();
let count = 0;
let problems = 0;
for (const [width, height, suffix] of sizes) {
  const page = await browser.newPage({ viewport: { width, height } });
  for (const [d, route] of shots)
    for (const theme of ['dark', 'light']) {
      if (only && !`${d}-${route}`.startsWith(only)) continue;
      await page.goto('about:blank');
      await page.goto(`${url}?d=${d}${theme === 'light' ? '&theme=light' : ''}#${route}`);
      await page.evaluate(() => document.fonts.ready);
      await page.waitForTimeout(60);
      if (theme === 'dark') {
        const issues = await page.evaluate(() => {
          const sel = '.btn, .pill, .opt, h1, h2, h3, .stabs span, .seg span, .kind, .who, .tchip, .flt, .grp, .trayb, .cred, .hint, .tk';
          const over = [...document.querySelectorAll(sel)]
            .filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).textOverflow !== 'ellipsis')
            .map((e) => `déborde : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
          const W = innerWidth;
          const H = innerHeight;
          const outside = [
            ...document.querySelectorAll('.answer, .btn, .callout, .drawer, .sheet, .dialog, .settings, .plate, .pdfv, .open'),
          ]
            .filter((e) => {
              const r = e.getBoundingClientRect();
              return r.width && (r.right > W + 0.5 || r.bottom > H + 0.5 || r.left < -0.5 || r.top < -0.5);
            })
            .map((e) => `hors fenêtre : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
          // What a fixed-height box cuts off: content taller than the box that holds it. The lists (the sidebar, the queue,
          // the drawer's list, a column of the board) and the page of a PDF scroll in the app: they are left out.
          const cut = [...document.querySelectorAll('.fb, .pb, .shb, .ob, .cb, .empty, .sb, .bband, .answer')]
            .filter((e) => e.scrollHeight > e.clientHeight + 2)
            .map((e) => `coupé : ${e.className} (${e.scrollHeight} > ${e.clientHeight})`);
          return [...over, ...outside, ...cut];
        });
        if (issues.length) {
          problems += issues.length;
          console.log(`${d}-${route}${suffix}`, issues);
        }
      }
      await page.screenshot({ path: join(out, `${d}-${route}${theme === 'light' ? '-clair' : ''}${suffix}.png`) });
      count++;
    }
  await page.close();
}
await browser.close();
console.log(`ok · ${count} captures · ${problems} problème(s)`);
