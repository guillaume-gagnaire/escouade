// The website's screenshots of the first launch (ESC-507, the page « Premiers pas » of the docs): the real app on a new
// installation, Claude Code (the fake `claude`) signed in on the machine, the demonstration project and its first agent.
// Four pictures, as site-shots.spec.ts frames them (the window at 1500 × 844 on the site's backdrop, JPEG quality 85): the
// harness and the first agent in the dark theme, the starting settings and the map « À découvrir » in the light one.
// `npm run site-shots -- --grep "first launch"` takes them alone.

import fs from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import path from 'node:path';
import { chromium, type Page } from '@playwright/test';
import { en } from '../../src/lib/i18n/en';
import { fr } from '../../src/lib/i18n/fr';
import { expect, test } from '../e2e/fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const OUT = process.env.ESCOUADE_SHOTS_OUT ?? path.join(ROOT, 'website', 'public', 'images');
const TURNS = path.join(ROOT, 'tests', 'fixtures', 'onboarding-turns.json');
/** The first agent's name, as Haiku would give it: not the fake claude's own. */
const NAMES = path.join(ROOT, 'tests', 'fixtures', 'onboarding-names.json');
const WINDOW = { x: 210, y: 70, w: 1500, h: 844 };
const SHOT = { w: 1920, h: 960 };
const SIGNED_IN = JSON.stringify({ claudeAiOauth: { accessToken: 'tok', refreshToken: 'ref', expiresAt: 4_102_444_800_000 } });

/** The picture of the window on the site's backdrop (site-shots.spec.ts' `compose`). */
async function compose(png: Buffer, file: string) {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: SHOT.w, height: SHOT.h }, deviceScaleFactor: 1 });
  await page.setContent(`<body style="margin:0;width:${SHOT.w}px;height:${SHOT.h}px;overflow:hidden;
      background:radial-gradient(ellipse at 50% 45%, #15191d 0%, #0b0d0f 60%, #08090a 100%)">
    <img src="data:image/png;base64,${png.toString('base64')}" style="position:absolute;left:${WINDOW.x}px;top:${WINDOW.y}px;
      width:${WINDOW.w}px;height:${WINDOW.h}px;border-radius:12px;
      box-shadow:0 0 0 1px rgba(255,255,255,.1),0 40px 90px -20px rgba(0,0,0,.85)"></body>`);
  await page.evaluate(() => document.images[0].decode());
  fs.mkdirSync(path.dirname(file), { recursive: true });
  await page.screenshot({ path: file, type: 'jpeg', quality: 85 });
  await browser.close();
}

for (const theme of ['dark', 'light'] as const)
  test.describe(`the website’s screenshots of the first launch, ${theme} theme`, () => {
    test.use({
      // No update to offer: a latest.json of an old version, as site-shots.spec.ts serves it.
      appEnv: async ({}, use) => {
        const updates = http.createServer((_, res) =>
          res.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify({ version: '0.0.1', notes: '', platforms: {} })),
        );
        await new Promise<void>((resolve) => updates.listen(0, '127.0.0.1', resolve));
        const { port } = updates.address() as net.AddressInfo;
        await use({ ESCOUADE_UPDATE_ENDPOINT: `http://127.0.0.1:${port}/latest.json`, FAKE_CLAUDE_DEMO: TURNS, FAKE_CLAUDE_NAMES: NAMES });
        await new Promise((resolve) => updates.close(resolve));
      },
      preStart: {
        run: (root: string) => {
          const dir = path.join(root, 'claude');
          fs.mkdirSync(dir, { recursive: true });
          fs.writeFileSync(path.join(dir, '.credentials.json'), SIGNED_IN);
          fs.writeFileSync(path.join(dir, '.claude.json'), JSON.stringify({ oauthAccount: { emailAddress: 'ada@example.com' } }));
          return { noAccounts: true, agentMode: '', allowBypass: false, telemetry: null, theme, sound: false };
        },
      },
    });

    test('the stations', async ({ app, language }) => {
      test.setTimeout(300_000);
      const L = (language === 'fr' ? fr : en).onboarding;
      const page: Page = app.page;
      const out = (name: string) => path.join(OUT, language === 'fr' ? '' : language, `${name}.jpg`);
      await page.setViewportSize({ width: WINDOW.w, height: WINDOW.h });
      const shot = async (name: string) => {
        await page.mouse.move(WINDOW.w - 2, WINDOW.h / 2);
        await page.waitForTimeout(700);
        await compose(await page.screenshot({ animations: 'disabled', caret: 'hide' }), out(name));
      };
      const heading = (name: string) => page.getByRole('heading', { level: 1, name });
      const journey = page.getByRole('region', { name: L.title });

      await expect(heading(L.harness.question)).toBeFocused();
      await expect(journey.getByRole('radio', { name: /Claude Code/ })).toContainText('ada@example.com');
      if (theme === 'dark') await shot('first-launch-harness');
      await page.keyboard.press('Enter');
      await expect(heading(L.defaults.question)).toBeFocused();
      if (theme === 'light') await shot('first-launch-defaults');
      await page.keyboard.press('Enter');
      await expect(heading(L.hookups.question)).toBeFocused();
      await page.keyboard.press('Enter');
      await expect(heading(L.project.question)).toBeVisible();
      await journey.getByRole('radio', { name: new RegExp(L.project.demo) }).click();
      await journey.getByRole('button', { name: new RegExp(L.project.demoGo) }).click();
      await expect(heading(L.agent.question)).toBeVisible();
      const frame = journey.getByTestId('onboarding-frame');
      await frame.getByRole('button', { name: language === 'fr' ? 'Envoyer' : 'Send' }).click();
      await expect(heading(L.agent.permission.title)).toBeVisible({ timeout: 60_000 });
      await frame
        .getByRole('button', { name: new RegExp(`^${L.agent.permission.allow}`) })
        .first()
        .click();
      await expect(heading(L.agent.done.title)).toBeVisible({ timeout: 60_000 });
      if (theme === 'dark') {
        await shot('first-launch-agent');
        return;
      }
      await journey.getByRole('button', { name: new RegExp(`^${L.continue}`) }).click();
      // The features (F-2, ESC-559): passed.
      await journey.getByRole('button', { name: (language === 'fr' ? fr : en).features.tour.skip }).click();
      await journey.getByRole('button', { name: language === 'fr' ? 'Non merci' : 'No thanks' }).click();
      await journey.getByRole('button', { name: L.end.discover }).click();
      await expect(page.getByRole('heading', { level: 1, name: language === 'fr' ? 'À découvrir' : 'Discover' })).toBeVisible();
      await shot('first-launch-discover');
    });
  });
