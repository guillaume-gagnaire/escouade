// The foundations of the 2.0 design (ESC-71) in the real app: « Thème » in Réglages › Général › Apparence (Système
// by default, applied at once, the window's own theme with it, kept after a restart and shown before the first render),
// and the common controls as the preview draws them (design/app-redesign/index.html#settings-app), in both themes, at
// 1440 × 900 and at CI's 1028 × 779. The captures of the app and of the preview go to ESCOUADE_E2E_SHOTS when it is set,
// to be looked at side by side.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, settingsEntry, closeSettings } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const PREVIEW = path.join(ROOT, 'design', 'app-redesign', 'index.html');
/** The preview loads Inter from the site's packages: without them, its text is not the app's. */
const PREVIEW_FONT = path.join(ROOT, 'website', 'node_modules', '@fontsource-variable', 'inter', 'opsz.css');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900, name: '' },
  { width: 1028, height: 779, name: '-small' },
];
/** The project colour both are tinted with: the preview's first project's. */
const TINT = 'oklch(0.72 0.12 48)';

const settingsOf = (data: string) => JSON.parse(fs.readFileSync(path.join(data, 'settings.json'), 'utf8'));

async function openApplication(page: Page) {
  await page.keyboard.press('Control+,');
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Général');
  return dialog;
}

/** The theme the window's page shows, its color-scheme, and the theme Tauri gives the native window. */
const shown = (page: Page) =>
  page.evaluate(async () => ({
    theme: document.documentElement.dataset.theme,
    scheme: getComputedStyle(document.documentElement).colorScheme,
    window: await (
      window as unknown as { __TAURI_INTERNALS__: { invoke: (c: string, a: object) => Promise<string> } }
    ).__TAURI_INTERNALS__.invoke('plugin:window|theme', { label: 'main' }),
  }));

/** The look of a control, as the browser computes it: what the app and the preview must share. */
function look(el: Locator) {
  return el.evaluate((node) => {
    const s = getComputedStyle(node);
    const r = node.getBoundingClientRect();
    return {
      height: Math.round(r.height),
      background: s.backgroundColor,
      color: s.color,
      shadow: s.boxShadow,
      radius: s.borderRadius,
      padding: s.padding,
      font: `${s.fontWeight} ${s.fontSize} ${s.fontFamily.split(',')[0]}`,
      features: s.fontFeatureSettings,
    };
  });
}

/** Tints a page with `TINT` at once, nothing gliding (the tint, a segment chosen), for both to be read alike. */
const tint = (page: Page) =>
  page.evaluate((p) => {
    if (!document.getElementById('still')) {
      const still = document.createElement('style');
      still.id = 'still';
      still.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
      document.head.append(still);
    }
    document.documentElement.style.setProperty('--p', p);
  }, TINT);

test('« Thème » is the system’s by default, applies at once with the window’s own theme, and stays after a restart', async ({ app }) => {
  let { page } = app;
  await addProject(page, app.repo);
  const system = await page.evaluate(() => (matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'));
  expect((await shown(page)).theme).toBe(system);

  let dialog = await openApplication(page);
  const theme = dialog.getByRole('group', { name: 'Thème' });
  await expect(theme.getByRole('button', { name: 'Système' })).toHaveAttribute('aria-pressed', 'true');
  await theme.getByRole('button', { name: 'Clair' }).click();
  await expect.poll(() => shown(page)).toEqual({ theme: 'light', scheme: 'light', window: 'light' });
  // The frame is the light one: a pale surface, a dark text.
  const colours = () =>
    page.evaluate(() => [
      getComputedStyle(document.body).backgroundColor,
      getComputedStyle(document.body).color,
      document.fonts.check('13px "Inter Variable"'),
    ]);
  expect(await colours()).toEqual([expect.stringMatching(/^oklch\(0\.9\d* /), 'rgb(21, 23, 28)', true]);
  // Saved by itself, nothing to « Enregistrer ».
  await expect.poll(() => settingsOf(app.data).theme).toBe('light');
  await closeSettings(dialog);

  // After a restart: light, the window too, and what the page reads before the first render says so.
  page = await app.restart();
  await expect.poll(() => page.evaluate(() => localStorage.getItem('escouade-theme')).catch(() => null)).toBe('light');
  await expect.poll(() => shown(page)).toEqual({ theme: 'light', scheme: 'light', window: 'light' });

  dialog = await openApplication(page);
  await dialog.getByRole('group', { name: 'Thème' }).getByRole('button', { name: 'Sombre' }).click();
  await expect.poll(() => shown(page)).toEqual({ theme: 'dark', scheme: 'dark', window: 'dark' });
  expect((await colours())[1]).toBe('rgb(247, 248, 248)');
  await dialog.getByRole('group', { name: 'Thème' }).getByRole('button', { name: 'Système' }).click();
  await expect.poll(() => settingsOf(app.data).theme).toBe('system');
  await expect.poll(async () => (await shown(page)).theme).toBe(system);
});

test('the common controls of Réglages › Général look as the preview draws them, in both themes, at both sizes', async ({ app }) => {
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await addProject(page, app.repo);
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const theme of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${theme}#settings-app`);
        await preview.evaluate(() => document.fonts.ready);

        const dialog = await openApplication(page);
        await dialog
          .getByRole('group', { name: 'Thème' })
          .getByRole('button', { name: theme === 'dark' ? 'Sombre' : 'Clair' })
          .click();
        await expect.poll(async () => (await shown(page)).theme).toBe(theme);
        await tint(page);
        await tint(preview);
        await page.evaluate(() => document.fonts.ready);

        const where = `${theme} ${size.width}×${size.height}`;
        // The segments: the group, the one chosen and another.
        const appSeg = dialog.getByRole('group', { name: 'Thème' });
        const prevSeg = preview.locator('#theme-pick');
        const pick = theme === 'dark' ? 'Sombre' : 'Clair';
        await preview.locator(`#theme-pick button`, { hasText: 'Système' }).evaluate((b) => b.setAttribute('aria-selected', 'false'));
        await preview.locator(`#theme-pick button`, { hasText: pick }).evaluate((b) => b.setAttribute('aria-selected', 'true'));
        expect(await look(appSeg), `segments, ${where}`).toEqual(await look(prevSeg));
        expect(await look(appSeg.getByRole('button', { name: pick })), `segment chosen, ${where}`).toEqual(
          await look(prevSeg.locator('button', { hasText: pick })),
        );
        expect(await look(appSeg.getByRole('button', { name: 'Système' })), `segment not chosen, ${where}`).toEqual(
          await look(prevSeg.locator('button', { hasText: 'Système' })),
        );
        // A field: the size of the terminals' text.
        const appField = dialog.getByRole('spinbutton', { name: 'Taille du texte des terminaux' });
        const prevField = preview.getByRole('textbox', { name: 'Taille du texte des terminaux' });
        expect(await look(appField), `field, ${where}`).toEqual(await look(prevField));
        // The surfaces and the texts: the tokens resolved by each engine, WebView2 and Chromium.
        const tokens = (p: Page) =>
          p.evaluate(() => {
            const probe = document.createElement('div');
            document.body.append(probe);
            const out = [
              '--frame',
              '--panel',
              '--raise',
              '--raise-2',
              '--well',
              '--text',
              '--soft',
              '--muted',
              '--dim',
              '--p-ink',
              '--wait',
              '--run',
              '--del',
              '--primary',
            ].map((t) => {
              probe.style.color = `var(${t})`;
              return [t, getComputedStyle(probe).color];
            });
            probe.remove();
            return out;
          });
        expect(await tokens(page), `tokens, ${where}`).toEqual(await tokens(preview));

        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const name = `settings-app${size.name}${theme === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${name}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${name}.preview.png`) });
          await appSeg.locator('xpath=..').screenshot({ path: path.join(SHOTS, `${name}.theme-row.app.png`) });
          await prevSeg.locator('xpath=../..').screenshot({ path: path.join(SHOTS, `${name}.theme-row.preview.png`) });
        }
        await closeSettings(dialog);
      }
  } finally {
    await browser.close();
  }
});

test('nothing stops gliding or pulsing but with prefers-reduced-motion, where everything does', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  /** How long the tint glides, and how long the tooltip's and the waiting dot's animations last. */
  const motion = () =>
    page.evaluate(() => {
      const dot = document.createElement('span');
      dot.className = 'pulse';
      document.body.append(dot);
      const out = [getComputedStyle(document.documentElement).transitionDuration, getComputedStyle(dot).animationDuration];
      dot.remove();
      return out;
    });
  expect(await motion()).toEqual(['0.7s', '1.6s']);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  // As good as none: a hundredth of a millisecond, for what waits for the end of one.
  expect(await motion()).toEqual(['1e-05s', '1e-05s']);
  await page.emulateMedia({ reducedMotion: 'no-preference' });
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of « Thème » and of the segments fit in their place at 1028 × 779, in both themes', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    await page.setViewportSize({ width: 1028, height: 779 });
    await page.keyboard.press('Control+,');
    const dialog = page.getByRole('dialog', { name: 'Settings' });
    await openSettingsPage(dialog, 'General');
    await expect(dialog.getByText('Theme, zoom, language, updates and what concerns you')).toBeVisible();
    const theme = dialog.getByRole('group', { name: 'Theme' });
    await expect(theme.getByRole('button')).toHaveText(['System', 'Light', 'Dark']);
    for (const pick of ['Light', 'Dark']) {
      await theme.getByRole('button', { name: pick }).click();
      // What is cut short, or overflows its group: none of it.
      const cut = await dialog.evaluate((d) =>
        [...d.querySelectorAll<HTMLElement>('[role="group"], [role="group"] button, [role="tab"], h2, h3')]
          .filter((el) => el.offsetParent && (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1))
          .map((el) => `${el.tagName.toLowerCase()} « ${el.textContent?.trim().slice(0, 40)} »`),
      );
      expect(cut, pick).toEqual([]);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `settings-app-small-en${pick === 'Light' ? '-light' : ''}.app.png`) });
    }
  });
});
