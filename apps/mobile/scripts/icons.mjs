// The phone app's icons and launch screens, and the two images Google Play asks beside the captures (MO-8,
// docs/MOBILE-PUBLISH.md § 5), drawn from Escouade's logo (website/public/logo.svg: the teal tile, the chevron and the
// spark), at the sizes the files already have. From the repository's root: `node apps/mobile/scripts/icons.mjs`.
//
// - iOS: the 1024 icon, full bleed and opaque (the system rounds it); the launch screen, 2732 square, the logo on the
//   app's dark panel.
// - Android: the adaptive icon's foreground (the glyph alone, inside the 66 dp of the safe zone; its background is
//   values/ic_launcher_background.xml, the tile's teal), the legacy square and round icons, the launch screens.
// - Google Play: the 512 icon, full bleed (Play rounds it), and the 1024 × 500 feature graphic, a PNG without alpha,
//   in French and in English, into apps/mobile/store/graphics/play/.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const ROOT = fileURLToPath(new URL('../../..', import.meta.url));
const MOBILE = path.join(ROOT, 'apps', 'mobile');
const RES = path.join(MOBILE, 'android', 'app', 'src', 'main', 'res');
const ASSETS = path.join(MOBILE, 'ios', 'App', 'App', 'Assets.xcassets');
const PLAY = path.join(MOBILE, 'store', 'graphics', 'play');
const FONT = path.join(ROOT, 'node_modules', '@fontsource-variable', 'inter', 'files', 'inter-latin-wght-normal.woff2');

/** The app's dark panel (src/app.css, `--panel`), under the launch screens and the feature graphic. */
const PANEL = '#0e0f11';

// The logo: its tile's colour and its glyph, in the 100 units of logo.svg.
const logo = fs.readFileSync(path.join(ROOT, 'website', 'public', 'logo.svg'), 'utf8');
const TEAL = /<rect[^>]*fill="(#[0-9a-fA-F]{6})"/.exec(logo)[1];
const GLYPH = logo
  .replace(/<!--[\s\S]*?-->/g, '')
  .replace(/^[\s\S]*?<rect[^>]*\/>/, '')
  .replace(/<\/svg>\s*$/, '')
  .trim();
const TILE = `<rect x="8" y="8" width="84" height="84" rx="24" fill="${TEAL}" />`;

/** The glyph at `scale` of its size on the tile, centred on (cx, cy) of the drawing. */
const glyph = (scale, cx = 50, cy = 50) => `<g transform="translate(${cx} ${cy}) scale(${scale}) translate(-50 -50)">${GLYPH}</g>`;
const svg = (box, body) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${box} ${box}">${body}</svg>`;

const DRAWINGS = {
  /** Full bleed: the tile's teal over the whole square, the glyph as large as on the tile (84 units become 100). */
  bleed: svg(100, `<rect width="100" height="100" fill="${TEAL}" />${glyph(100 / 84)}`),
  /** The adaptive foreground: 108 dp, of which the launcher shows 72; the glyph as on a 72 dp tile. */
  foreground: svg(108, glyph(72 / 84, 54, 54)),
  /** The legacy icon: the logo itself. */
  square: svg(100, TILE + GLYPH),
  /** The legacy round icon: a teal disc, the glyph as on the tile. */
  round: svg(100, `<circle cx="50" cy="50" r="46" fill="${TEAL}" />${GLYPH}`),
};

/** The width and height of a PNG, from its header. */
const size = (file) => {
  const b = fs.readFileSync(file);
  return [b.readUInt32BE(16), b.readUInt32BE(20)];
};

const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: 1 });

/** `html` drawn at width × height into `file`; `transparent` keeps the alpha, an opaque page is written as RGB. */
async function draw(file, width, height, html, transparent = false) {
  await page.setViewportSize({ width, height });
  await page.setContent(
    `<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;width:${width}px;height:${height}px;overflow:hidden;` +
      `background:${transparent ? 'transparent' : '#fff'}}svg{display:block}</style></head><body>${html}</body></html>`,
  );
  await page.evaluate(() => document.fonts.ready);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  await page.screenshot({ path: file, omitBackground: transparent });
}

/** A drawing that fills a square of `n` pixels. */
const square = (drawing, n) => drawing.replace('<svg ', `<svg width="${n}" height="${n}" `);

/** A launch screen: the logo, a quarter of the shorter side, on the dark panel. */
const launch = (width, height) => {
  const n = Math.round(Math.min(width, height) * (width === height ? 0.12 : 0.25));
  return `<div style="width:${width}px;height:${height}px;background:${PANEL};display:grid;place-items:center">${square(DRAWINGS.square, n)}</div>`;
};

try {
  // Android: the icons of each density, the launch screens of each orientation.
  for (const dir of fs.readdirSync(RES).filter((d) => d.startsWith('mipmap-') && !d.includes('anydpi'))) {
    for (const [name, drawing, transparent] of [
      ['ic_launcher.png', DRAWINGS.square, true],
      ['ic_launcher_round.png', DRAWINGS.round, true],
      ['ic_launcher_foreground.png', DRAWINGS.foreground, true],
    ]) {
      const file = path.join(RES, dir, name);
      const [n] = size(file);
      await draw(file, n, n, square(drawing, n), transparent);
    }
  }
  for (const dir of fs.readdirSync(RES).filter((d) => d.startsWith('drawable'))) {
    const file = path.join(RES, dir, 'splash.png');
    if (!fs.existsSync(file)) continue;
    const [w, h] = size(file);
    await draw(file, w, h, launch(w, h));
  }

  // iOS: the icon, the launch screen.
  await draw(path.join(ASSETS, 'AppIcon.appiconset', 'AppIcon-512@2x.png'), 1024, 1024, square(DRAWINGS.bleed, 1024));
  const splash = path.join(ASSETS, 'Splash.imageset');
  for (const name of fs.readdirSync(splash).filter((f) => f.endsWith('.png'))) {
    const [w, h] = size(path.join(splash, name));
    await draw(path.join(splash, name), w, h, launch(w, h));
  }

  // Google Play: the icon, the feature graphic.
  await draw(path.join(PLAY, 'icon-512.png'), 512, 512, square(DRAWINGS.bleed, 512));
  const font = fs.readFileSync(FONT).toString('base64');
  const feature = ({ line, sub }) => `<style>
@font-face { font-family: Inter; src: url(data:font/woff2;base64,${font}) format('woff2'); font-weight: 100 900; }
.f { position: relative; width: 1024px; height: 500px; overflow: hidden; font-family: Inter, sans-serif; background:
  radial-gradient(520px 420px at 230px 250px, color-mix(in oklch, ${TEAL} 22%, transparent), transparent 70%), ${PANEL}; }
.f .logo { position: absolute; left: 70px; top: 50%; transform: translateY(-50%); filter: drop-shadow(0 24px 48px rgba(0, 0, 0, .45)); }
.f .text { position: absolute; left: 430px; right: 56px; top: 50%; transform: translateY(-50%);
  display: flex; flex-direction: column; gap: 16px; }
.f h1 { margin: 0; font-size: 78px; font-weight: 720; letter-spacing: -0.035em; color: #f4f5f6; line-height: 1; }
.f .line { font-size: 38px; font-weight: 620; letter-spacing: -0.015em; line-height: 1.12; color: ${TEAL}; }
.f .sub { font-size: 21px; font-weight: 440; color: #9aa1aa; line-height: 1.4; max-width: 520px; }
</style><div class="f"><div class="logo">${square(DRAWINGS.square, 330)}</div>
<div class="text"><h1>Escouade</h1><div class="line">${line}</div><div class="sub">${sub}</div></div></div>`;
  for (const [locale, texts] of Object.entries({
    'fr-FR': { line: 'Débloque tes agents de code', sub: 'Depuis ton téléphone, directement vers ton poste ou ton serveur.' },
    'en-US': { line: 'Unblock your coding agents', sub: 'From your phone, straight to your computer or your server.' },
  })) {
    await draw(path.join(PLAY, `feature-graphic-${locale}.png`), 1024, 500, feature(texts));
  }
} finally {
  await browser.close();
}
console.log('written: the Android and iOS icons and launch screens, and the images of Google Play');
