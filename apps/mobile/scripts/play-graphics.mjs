// The two images Google Play asks beside the captures (MO-8, docs/MOBILE-PUBLISH.md § 5), from the app's own icon:
// the icon at 512 × 512 and the feature graphic at 1024 × 500 (a PNG without alpha), in French and in English, into
// apps/mobile/store/graphics/play/. From the repository's root: `node apps/mobile/scripts/play-graphics.mjs`.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const ROOT = fileURLToPath(new URL('../../..', import.meta.url));
const OUT = path.join(ROOT, 'apps', 'mobile', 'store', 'graphics', 'play');
const ICON = path.join(ROOT, 'apps', 'mobile', 'ios', 'App', 'App', 'Assets.xcassets', 'AppIcon.appiconset', 'AppIcon-512@2x.png');
const FONT = path.join(ROOT, 'node_modules', '@fontsource-variable', 'inter', 'files', 'inter-latin-wght-normal.woff2');

const TEXTS = {
  'fr-FR': { line: 'Débloque tes agents de code', sub: 'Depuis ton téléphone, directement vers ton poste ou ton serveur.' },
  'en-US': { line: 'Unblock your coding agents', sub: 'From your phone, straight to your computer or your server.' },
};

const icon = fs.readFileSync(ICON).toString('base64');
const font = fs.readFileSync(FONT).toString('base64');

/** The feature graphic: the icon on the same diagonal grid, the name and the line of the listing. */
const feature = ({ line, sub }) => `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face { font-family: Inter; src: url(data:font/woff2;base64,${font}) format('woff2'); font-weight: 100 900; }
html, body { margin: 0; width: 1024px; height: 500px; overflow: hidden; background: #ffffff; }
body { font-family: Inter, sans-serif; position: relative; }
.grid { position: absolute; inset: 0; opacity: .9; background-image:
  repeating-linear-gradient(45deg, transparent 0 98px, #e3e7f0 98px 100px),
  repeating-linear-gradient(-45deg, transparent 0 98px, #e3e7f0 98px 100px); }
.card { position: absolute; left: 64px; top: 50%; transform: translateY(-50%); width: 300px; height: 300px;
  border-radius: 66px; overflow: hidden; background: #fff; box-shadow: 0 18px 48px rgba(40, 110, 200, .22); }
.card img { width: 100%; height: 100%; display: block; }
.text { position: absolute; left: 420px; right: 56px; top: 50%; transform: translateY(-50%);
  display: flex; flex-direction: column; gap: 14px; }
h1 { margin: 0; font-size: 76px; font-weight: 750; letter-spacing: -0.03em; color: #17202b; line-height: 1; }
.line { font-size: 38px; font-weight: 650; letter-spacing: -0.01em; line-height: 1.12; color: transparent;
  background: linear-gradient(120deg, #7fd0ff, #4ea8ff); -webkit-background-clip: text; background-clip: text; }
.sub { font-size: 21px; font-weight: 450; color: #5a6573; line-height: 1.35; max-width: 520px; }
</style></head><body><div class="grid"></div>
<div class="card"><img src="data:image/png;base64,${icon}"></div>
<div class="text"><h1>Escouade</h1><div class="line">${line}</div><div class="sub">${sub}</div></div>
</body></html>`;

fs.mkdirSync(OUT, { recursive: true });
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1024, height: 500 }, deviceScaleFactor: 1 });
  for (const [locale, texts] of Object.entries(TEXTS)) {
    await page.setContent(feature(texts));
    await page.evaluate(() => document.fonts.ready);
    // An opaque page: Chromium writes it as an RGB PNG, the 24 bits Google asks.
    await page.screenshot({ path: path.join(OUT, `feature-graphic-${locale}.png`) });
  }
  const square = await browser.newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: 1 });
  await square.setContent(
    `<html><body style="margin:0"><img src="data:image/png;base64,${icon}" style="width:512px;height:512px;display:block"></body></html>`,
  );
  await square.screenshot({ path: path.join(OUT, 'icon-512.png') });
} finally {
  await browser.close();
}
console.log(`written to ${path.relative(ROOT, OUT)}`);
