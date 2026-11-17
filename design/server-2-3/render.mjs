// Renders the ESC-516 mockups (the server mode, three directions). `node design/server-2-3/render.mjs [prefix]` writes
// captures/<d>-<route>.png (dark, 1440 × 900), <d>-<route>-clair.png (light), and the same at 1024 × 728 with `-1024`
// before the extension; then x-<route>, common to the three. It also says any text that overflows its box, anything
// that leaves the window, and anything a fixed-height box cuts off. With `--sheets`, it also writes contact sheets
// (captures/planches/, not versioned) that put the four variants of a screen side by side, to look at them all.
import { mkdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dir = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(dir, '../../package.json'))('playwright');
const url = pathToFileURL(join(dir, 'maquette.html')).href;
const out = join(dir, 'captures');
mkdirSync(out, { recursive: true });
const args = process.argv.slice(2);
const sheets = args.includes('--sheets');
const only = args.find((a) => !a.startsWith('--'));
const sizes = [
  [1440, 900, ''],
  [1024, 728, '-1024'],
];
const routes = ['un', 'deux', 'trois', 'serveur', 'coupure', 'connexion', 'erreur', 'projet', 'appareils'];
const common = ['hote', 'etats', 'version', 'jeton', 'fichier', 'compte', 'notifs'];
const shots = [...['a', 'b', 'c'].flatMap((d) => routes.map((r) => [d, r])), ...common.map((r) => ['x', r])];
const browser = await chromium.launch();
let count = 0;
let problems = 0;
for (const [width, height, suffix] of sizes) {
  const page = await browser.newPage({ viewport: { width, height } });
  page.on('pageerror', (e) => {
    problems++;
    console.log(`erreur de la page (${width}) :`, e.message);
  });
  for (const [d, route] of shots)
    for (const theme of ['dark', 'light']) {
      if (only && !`${d}-${route}`.startsWith(only)) continue;
      await page.goto('about:blank');
      await page.goto(`${url}?d=${d}${theme === 'light' ? '&theme=light' : ''}#${route}`);
      await page.evaluate(() => document.fonts.ready);
      await page.waitForTimeout(60);
      if (theme === 'dark') {
        const issues = await page.evaluate(() => {
          const sel =
            '.btn, .pill, h1, h2, h3, .stabs span, .seg span, .mach, .lab, .hint, .kind, .tip, .mi, .it, .step b, .hsteps span, .lane .nm, .cmd, .frow .meta';
          const over = [...document.querySelectorAll(sel)]
            .filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).textOverflow !== 'ellipsis')
            .map((e) => `déborde : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
          const W = innerWidth;
          const H = innerHeight;
          const outside = [
            ...document.querySelectorAll(
              '.btn, .menu, .tip, .switcher, .dialog, .settings, .wiz, .addcard, .toast, .intoast, .lostbar, .acts',
            ),
          ]
            .filter((e) => {
              const r = e.getBoundingClientRect();
              return r.width && (r.right > W + 0.5 || r.bottom > H + 0.5 || r.left < -0.5 || r.top < -0.5);
            })
            .map((e) => `hors fenêtre : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
          // What a fixed-height box cuts off. The lists that scroll in the app (the sidebar, the folder browser's rows,
          // a settings page) are left out; the dialogs, the switcher and the rail are not.
          const cut = [
            ...document.querySelectorAll('.cbody, .dialog, .wiz .wb, .switcher, .rail, .spaces, .addcard, .jcard, .bpage .bb, .ov'),
          ]
            .filter((e) => e.scrollHeight > e.clientHeight + 2)
            .map((e) => `coupé : ${e.className} (${e.scrollHeight} > ${e.clientHeight})`);
          // A rail or a column whose last item overlaps the low icons.
          const rails = [...document.querySelectorAll('.rail, .spaces')].flatMap((r) => {
            const low = r.querySelector('.low');
            if (!low) return [];
            const lowTop = low.getBoundingClientRect().top;
            const kids = [...r.children].filter((k) => k !== low);
            const last = kids[kids.length - 1];
            return last && last.getBoundingClientRect().bottom > lowTop + 1 ? [`tassé : ${r.className}`] : [];
          });
          const empty = document.getElementById('root').children.length ? [] : ['page vide'];
          return [...empty, ...over, ...outside, ...cut, ...rails];
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
if (sheets) {
  // A sheet per screen: the four variants (dark and light, at 1440 and 1024), each scaled to half.
  const sheetDir = join(out, 'planches');
  mkdirSync(sheetDir, { recursive: true });
  const page = await browser.newPage({ viewport: { width: 1480, height: 1000 } });
  for (const [d, route] of shots) {
    if (only && !`${d}-${route}`.startsWith(only)) continue;
    const src = (v) => pathToFileURL(join(out, `${d}-${route}${v}.png`)).href;
    const cell = (v, w) => `<figure><img src="${src(v)}" style="width:${w}px"><figcaption>${d}-${route}${v}</figcaption></figure>`;
    const html = join(sheetDir, 'planche.html');
    writeFileSync(
      html,
      `<body style="margin:0;background:#777;font:12px sans-serif;display:grid;grid-template-columns:720px 720px;gap:8px;padding:8px">
      ${cell('', 720)}${cell('-clair', 720)}${cell('-1024', 512)}${cell('-clair-1024', 512)}</body>`,
    );
    await page.goto(pathToFileURL(html).href);
    await page.waitForLoadState('load');
    await page.screenshot({ path: join(sheetDir, `${d}-${route}.png`), fullPage: true });
  }
  await page.close();
}
await browser.close();
console.log(`ok · ${count} captures · ${problems} problème(s)`);
