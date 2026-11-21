// Renders the ESC-549 mockups (the phone app, three directions). `node design/mobile-2-3/render.mjs [prefix]` writes
// captures/<d>-<route>-<size>.png (dark) and <d>-<route>-clair-<size>.png (light), at 360 × 780 (an Android phone, `-360`)
// and 430 × 932 (an iPhone, `-430`), at the phone's pixel ratio; then x-<route>, common to the three. It also says the
// page's errors, an empty page, any text that overflows its box, anything that leaves the screen, and anything a
// fixed-height box cuts off. With `--sheets`, it also writes contact sheets (captures/planches/, not versioned) that put
// the four variants of a screen side by side, to look at them all. Without a prefix, it also captures the comparison
// page (index.html) in dark and light: captures/comparaison.png and comparaison-clair.png.
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
// The two phones of the ticket: a small Android (360 × 780) and a large iPhone (430 × 932). The captures are taken at
// a device pixel ratio of 2, so that the text reads as on a phone once the image is shrunk.
const sizes = [
  [360, 780, '-360'],
  [430, 932, '-430'],
];
export const routes = ['appairage', 'accueil', 'autorisation', 'conversation', 'boite', 'kanban', 'notification'];
export const common = ['verrou', 'horsligne', 'revoque'];
// The creation screens, drawn in B only: the direction the user chose on 2026-11-20, with creation from the phone.
export const creation = ['creer-projet', 'creer-agent', 'ticket-dicte'];
const shots = [
  ...['a', 'b', 'c'].flatMap((d) => routes.map((r) => [d, r])),
  ...creation.map((r) => ['b', r]),
  ...common.map((r) => ['x', r]),
];
const browser = await chromium.launch();
let count = 0;
let problems = 0;
for (const [width, height, suffix] of sizes) {
  const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 2 });
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
            '.btn, .pill, .chip, h1, h2, .lab, .seg span, .tabs span, .peek span, .cfoot span, .nav .t, .where, .found, .opts span, .chips span, .kh, .sh';
          const over = [...document.querySelectorAll(sel)]
            .filter(
              (e) =>
                e.scrollWidth > e.clientWidth + 1 &&
                getComputedStyle(e).textOverflow !== 'ellipsis' &&
                !e.closest('.chips, .tiles, .ktiles, .carousel'),
            )
            .map((e) => `déborde : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
          const W = innerWidth;
          const H = innerHeight;
          // What leaves the screen. The rows that scroll sideways in the app (chips, tiles, a carousel, the columns of
          // the Kanban) are left out: their last items are meant to go past the edge.
          const outside = [
            ...document.querySelectorAll(
              '.btn, .card, .sheet, .bio, .nt, .composer, .slide, .hold, .tabs, .peek, .cfoot, .field, .code8, .dict, .clone, .area, .opt, .spaces',
            ),
          ]
            .filter((e) => !e.closest('.chips, .tiles, .ktiles, .carousel, [data-side]'))
            .filter((e) => {
              const r = e.getBoundingClientRect();
              return r.width && (r.right > W + 0.5 || r.bottom > H + 0.5 || r.left < -0.5 || r.top < -0.5);
            })
            .map((e) => `hors de l'écran : ${e.className} « ${e.textContent.trim().slice(0, 40)} »`);
          // What a fixed-height box cuts off. The lists that scroll in the app (the feed, a conversation, the body of a
          // space, a sheet's body) are left out; a card, a sheet, a tile, a notification and the prompt are not.
          const cut = [
            ...document.querySelectorAll('.card, .sheet, .tile, .ktile, .bio, .nt:not(.stackbehind), .perm, .lost, .row .ex, .center'),
          ]
            .filter((e) => e.scrollHeight > e.clientHeight + 2)
            .map((e) => `coupé : ${e.className} (${e.scrollHeight} > ${e.clientHeight})`);
          // A row whose own children overlap: the controls at the bottom of a card pushed under its edge.
          const clipped = [
            ...document.querySelectorAll('.card .slide, .card .row2, .sheet .hold, .sheet .btn.pri, .composer, .tabs, .cfoot'),
          ]
            .filter((e) => {
              const box = e.closest('.card, .sheet, .phone').getBoundingClientRect();
              const r = e.getBoundingClientRect();
              return r.bottom > box.bottom + 0.5;
            })
            .map((e) => `poussé hors de sa boîte : ${e.className}`);
          const empty = document.getElementById('root').children.length ? [] : ['page vide'];
          return [...empty, ...over, ...outside, ...cut, ...clipped];
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
  // A sheet per screen: the four variants (dark and light, at 360 and 430), side by side at their CSS size.
  const sheetDir = join(out, 'planches');
  mkdirSync(sheetDir, { recursive: true });
  const page = await browser.newPage({ viewport: { width: 1640, height: 960 } });
  for (const [d, route] of shots) {
    if (only && !`${d}-${route}`.startsWith(only)) continue;
    const src = (v) => pathToFileURL(join(out, `${d}-${route}${v}.png`)).href;
    const cell = (v, w) =>
      `<figure style="margin:0"><img src="${src(v)}" style="width:${w}px;display:block"><figcaption>${d}-${route}${v}</figcaption></figure>`;
    const html = join(sheetDir, 'planche.html');
    writeFileSync(
      html,
      `<body style="margin:0;background:#777;font:12px sans-serif;display:flex;gap:10px;padding:10px;align-items:flex-start">
      ${cell('-360', 360)}${cell('-clair-360', 360)}${cell('-430', 430)}${cell('-clair-430', 430)}</body>`,
    );
    await page.goto(pathToFileURL(html).href);
    await page.waitForLoadState('load');
    await page.screenshot({ path: join(sheetDir, `${d}-${route}.png`), fullPage: true });
  }
  await page.close();
}
if (!only) {
  // The comparison page (index.html), its first rows, dark and light: captures/comparaison.png, comparaison-clair.png.
  const page = await browser.newPage({ viewport: { width: 1440, height: 1500 } });
  for (const light of [false, true]) {
    await page.goto(pathToFileURL(join(dir, 'index.html')).href);
    if (light) await page.click('[data-t="-clair"]');
    await page.evaluate(() => Promise.all([...document.images].map((i) => ((i.loading = 'eager'), i.decode().catch(() => {})))));
    await page.waitForTimeout(150);
    await page.screenshot({ path: join(out, `comparaison${light ? '-clair' : ''}.png`) });
    count++;
  }
  await page.close();
}
await browser.close();
console.log(`ok · ${count} captures · ${problems} problème(s)`);
