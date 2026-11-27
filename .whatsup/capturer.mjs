// Captures the atelier's states for a note of What's up ?, without the osho-whats-up plugin (docs/WHATS-UP.md).
//
//   npx vite --config .whatsup/atelier/vite.config.ts --port <port>      (in another terminal)
//   node .whatsup/capturer.mjs --url http://127.0.0.1:<port> --id 2026-11-27-2 --lang fr|en <state> [<state>…]
//
// Each state of .whatsup/atelier/etats.ts, in light and in dark, at 1440 × 900: public/whats-up/<id>-<state>.webp and
// <id>-<state>-sombre.webp (public/whats-up/en/ for English).
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  if (i < 0) return fallback;
  const [v] = args.splice(i, 2).slice(1);
  return v;
};
const url = opt('url', null);
const id = opt('id', null);
const lang = opt('lang', 'fr');
const states = args;
if (!url || !id || !states.length || !['fr', 'en'].includes(lang)) {
  console.error('node .whatsup/capturer.mjs --url <atelier> --id <note> --lang fr|en <state>…');
  process.exit(2);
}
const dir = join('public/whats-up', lang === 'fr' ? '' : lang);
mkdirSync(dir, { recursive: true });

const browser = await chromium.launch();
try {
  for (const theme of ['clair', 'sombre']) {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      locale: lang === 'en' ? 'en-US' : 'fr-FR',
      colorScheme: theme === 'sombre' ? 'dark' : 'light',
    });
    for (const state of states) {
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.goto(`${url}/.whatsup/atelier/atelier.html?etat=${state}&theme=${theme}&lang=${lang}`);
      await page.waitForFunction(() => window.__whatsupPret === true, null, { timeout: 30_000 });
      await page.mouse.move(-10, -10);
      const png = await page.screenshot({ type: 'png' });
      // WebP by the browser's own encoder: no image library to install.
      const webp = await page.evaluate(async (b64) => {
        const img = new Image();
        img.src = `data:image/png;base64,${b64}`;
        await img.decode();
        const c = document.createElement('canvas');
        c.width = img.width;
        c.height = img.height;
        c.getContext('2d').drawImage(img, 0, 0);
        return c.toDataURL('image/webp', 0.9).split(',')[1];
      }, png.toString('base64'));
      const file = join(dir, `${id}-${state}${theme === 'sombre' ? '-sombre' : ''}.webp`);
      writeFileSync(file, Buffer.from(webp, 'base64'));
      console.log(file, errors.length ? `(erreurs : ${errors.join(' | ')})` : '');
      await page.close();
    }
    await context.close();
  }
} finally {
  await browser.close();
}
