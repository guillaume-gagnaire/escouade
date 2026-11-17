import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, unfoldSections } from './fixture';

// ESC-506 (ON-3): the discovery of the features, Aide › À découvrir. Three discoveries done for real (a ticket the
// autopilot launches with the fake `claude`, then validated; a terminal; a dictation with the e2e's fake microphone, a
// WAV file), each ticked by what the app did, never by a click on its card; one dismissed that does not come back after
// a restart; « Tout masquer ». Run at both sizes, the CI's runner's (ESCOUADE_E2E_VIEWPORT=1024x728) and the default one.

/** The size of « base » in the catalog of the app (`voice::models::CATALOG`). */
const BASE_SIZE = 147_951_465;
const SAID = 'découvrir la dictée';

/** A WAV of a tone, 16-bit mono at 16 kHz: what the app hears in place of a microphone. */
function toneWav(file: string, secs = 20) {
  const rate = 16_000;
  const n = rate * secs;
  const buf = Buffer.alloc(44 + n * 2);
  buf.write('RIFF', 0);
  buf.writeUInt32LE(36 + n * 2, 4);
  buf.write('WAVEfmt ', 8);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(1, 22);
  buf.writeUInt32LE(rate, 24);
  buf.writeUInt32LE(rate * 2, 28);
  buf.writeUInt16LE(2, 32);
  buf.writeUInt16LE(16, 34);
  buf.write('data', 36);
  buf.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) buf.writeInt16LE(Math.round(Math.sin((2 * Math.PI * 220 * i) / rate) * 0.4 * 32767), 44 + i * 2);
  fs.writeFileSync(file, buf);
}

const WAV = path.join(os.tmpdir(), `escouade-e2e-discover-${process.pid}.wav`);
toneWav(WAV);

/** The model the dictation uses, already there: a file of its size (the fake transcriber reads nothing of it). */
function placeModel(root: string) {
  const dir = path.join(root, 'data', 'voice', 'models');
  fs.mkdirSync(dir, { recursive: true });
  const fd = fs.openSync(path.join(dir, 'ggml-base.bin'), 'w');
  fs.ftruncateSync(fd, BASE_SIZE);
  fs.closeSync(fd);
}

/** Aide › À découvrir. */
async function openMap(page: Page) {
  await page.getByTestId('rail-help').click();
  await page.getByRole('menuitem', { name: /À découvrir/ }).click();
  await expect(page.getByRole('heading', { level: 1, name: 'À découvrir' })).toBeVisible();
}

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const stop = (page: Page, id: string) => page.locator(`[data-stop="${id}"]`);

/** A station chosen on the map, then « Essayer »: the real screen, its real control shown in a bubble. */
async function tryIt(page: Page, id: string) {
  await stop(page, id).click();
  await page.getByTestId('discover-detail').getByRole('button', { name: 'Essayer' }).click();
}

/** The bubble that shows the real control, closed. */
async function spotted(page: Page, id: string) {
  const tip = page.locator(`[data-testid="discover-tip"][data-tip="${id}"]`);
  await expect(tip).toBeVisible();
  await expect(tip).toHaveAttribute('data-mode', 'spot');
  await tip.getByRole('button', { name: 'Compris' }).click();
  await expect(tip).toHaveCount(0);
}

/** Nothing of the window goes past its right edge. */
async function fitsAcross(page: Page) {
  const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(over).toBeLessThanOrEqual(0);
}

test.describe('the discovery of the features', () => {
  test.use({
    appEnv: { ESCOUADE_VOICE_WAV: WAV, ESCOUADE_VOICE_TRANSCRIPT: SAID },
    preStart: { run: (root: string) => placeModel(root) },
  });

  test('three discoveries done for real tick themselves, one dismissed never comes back, « Tout masquer » keeps the map', async ({
    app,
  }) => {
    test.setTimeout(300_000);
    let page = app.page;
    await addProject(page, app.repo);
    // Something to discover: a dot on « Aide ».
    await expect(page.getByTestId('help-dot')).toBeVisible();
    await openMap(page);
    await fitsAcross(page);
    // The five lanes of the cycle; no team server here: the team is not on this device, and is not counted.
    for (const lane of ['Planifier', 'Coder', 'Relire', 'Livrer', 'Piloter'])
      await expect(page.getByRole('list', { name: lane, exact: true })).toBeVisible();
    await expect(stop(page, 'team')).toHaveAttribute('data-status', 'absent');
    await expect(page.getByText(/^0 sur 17 · /)).toBeVisible();
    // The dictation says its weight before anything is downloaded.
    await stop(page, 'dictation').click();
    await expect(page.getByTestId('discover-detail')).toContainText('148 Mo');

    // 1. A ticket: « Essayer » opens the board, on its button « Nouveau ticket ».
    await tryIt(page, 'ticket');
    await spotted(page, 'ticket');
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Une première tâche');
    await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
    // The first ticket made: the autopilot offered at the right moment, on its switch, once.
    const offered = page.locator('[data-testid="discover-tip"][data-tip="autopilot"]');
    await expect(offered).toBeVisible();
    await expect(offered).toHaveAttribute('data-mode', 'offer');
    await expect(offered).toContainText('il ne valide ni ne merge jamais sans toi');
    if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `discover-tip-${page.viewportSize()?.width ?? 0}.png`) });
    await offered.getByRole('button', { name: 'Compris' }).click();
    // The autopilot launched it with the fake claude; its report read, « Valider » lands it.
    const review = page.getByRole('region', { name: 'À tester' });
    await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 90_000 });
    await review.getByRole('button', { name: 'Valider et merger' }).click();
    await expect(page.getByRole('region', { name: 'Terminé' }).getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });

    // 2. A terminal: « Essayer » shows the « + » of the terminals (its section unfolded in a short window).
    await unfoldSections(page);
    await openMap(page);
    await expect(stop(page, 'ticket')).toHaveAttribute('data-status', 'done');
    await expect(stop(page, 'autopilot')).toHaveAttribute('data-status', 'done');
    await expect(stop(page, 'approve')).toHaveAttribute('data-status', 'done');
    await tryIt(page, 'terminal');
    await spotted(page, 'terminal');
    await page.getByRole('button', { name: 'Nouveau terminal' }).click();
    await page.getByRole('menuitem').first().click();
    await expect(page.locator('.xterm')).toBeVisible();

    // 3. The dictation, with the fake microphone: « Essayer » shows the microphone of the agent's message.
    await openMap(page);
    await expect(stop(page, 'terminal')).toHaveAttribute('data-status', 'done');
    await tryIt(page, 'dictation');
    await spotted(page, 'dictation');
    // Hovered now, the microphone offers nothing more: one bubble at the right moment per session.
    const mic = page.getByRole('button', { name: /^(Dicter|Finir la dictée)$/ }).first();
    await mic.hover();
    await page.waitForTimeout(400);
    await expect(page.getByTestId('discover-tip')).toHaveCount(0);
    await mic.click();
    await expect(page.locator('[data-voice-phase]')).toHaveAttribute('data-voice-phase', 'listening');
    await page.waitForTimeout(800);
    await mic.click();
    await expect(page.getByRole('textbox').last()).toHaveValue(new RegExp(SAID), { timeout: 30_000 });

    await openMap(page);
    await expect(stop(page, 'dictation')).toHaveAttribute('data-status', 'done');
    await expect(page.getByText(/^5 sur 17 · /)).toBeVisible();
    // One dismissed: « Le serveur MCP ».
    await stop(page, 'mcp').click();
    await page.getByTestId('discover-detail').getByRole('button', { name: 'Écarter' }).click();
    await expect(stop(page, 'mcp')).toHaveAttribute('data-status', 'dismissed');
    await fitsAcross(page);

    // A restart: what was done stays done, what was dismissed stays dismissed.
    page = await app.restart();
    await openMap(page);
    for (const id of ['ticket', 'autopilot', 'approve', 'terminal', 'dictation'])
      await expect(stop(page, id)).toHaveAttribute('data-status', 'done');
    await expect(stop(page, 'mcp')).toHaveAttribute('data-status', 'dismissed');
    const state = JSON.parse(fs.readFileSync(path.join(app.root, 'data', 'state.json'), 'utf8'));
    expect(state.onboarding.discover).toMatchObject({ ticket: 'done', terminal: 'done', dictation: 'done', mcp: 'dismissed' });
    expect(state.onboarding.tipped).toContain('autopilot');

    // « Tout masquer »: no more dot nor bubble, the map stays in Aide › À découvrir; and so after a restart.
    await expect(page.getByTestId('help-dot')).toBeVisible();
    await page.getByRole('button', { name: 'Tout masquer' }).click();
    await expect(page.getByText(/Masqué : plus de bulle ni de point sur Aide/)).toBeVisible();
    await expect(page.getByTestId('help-dot')).toHaveCount(0);
    page = await app.restart();
    await expect(page.getByRole('button', { name: 'Aide' })).toBeVisible();
    await expect(page.getByTestId('help-dot')).toHaveCount(0);
    await openMap(page);
    await expect(page.getByRole('button', { name: 'Réafficher' })).toBeVisible();
    await expect(stop(page, 'mcp')).toHaveAttribute('data-status', 'dismissed');
  });
});

for (const theme of ['dark', 'light'] as const) {
  test.describe(`the map in the ${theme} theme`, () => {
    test.use({
      appEnv: { ESCOUADE_VOICE_WAV: WAV, ESCOUADE_VOICE_TRANSCRIPT: SAID },
      preStart: {
        run: (root: string) => {
          placeModel(root);
          return { theme };
        },
      },
    });

    test('holds whole in the window, the detail of the station chosen in view', async ({ app }) => {
      const page = app.page;
      await addProject(page, app.repo, { firstAgent: false });
      await openMap(page);
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      await stop(page, 'dictation').click();
      await fitsAcross(page);
      const size = page.viewportSize();
      const detail = page.getByTestId('discover-detail');
      const box = await detail.getByRole('button', { name: 'Essayer' }).boundingBox();
      expect(box && box.y + box.height).toBeLessThanOrEqual(size?.height ?? 10_000);
      // Every station of the map in the window and seen: none cut, none under the detail.
      const hidden = await page.locator('[data-stop]').evaluateAll((els) =>
        els
          .filter((e) => {
            const r = e.getBoundingClientRect();
            const at = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
            return r.right > window.innerWidth + 1 || r.bottom > window.innerHeight + 1 || at?.closest('[data-stop]') !== e;
          })
          .map((e) => (e as HTMLElement).dataset.stop),
      );
      expect(hidden).toEqual([]);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `discover-${theme}-${size?.width ?? 0}.png`) });
    });
  });
}

/** The device's policy, as a build of development reads it from `ESCOUADE_TEST_POLICIES`: the dictation cut. */
const NO_DICTATION = path.join(os.tmpdir(), `escouade-e2e-discover-policies-${process.pid}.json`);
fs.writeFileSync(NO_DICTATION, JSON.stringify({ 'voice.dictation': false }));

test.describe('a device whose policy cuts the dictation', () => {
  test.use({
    appEnv: { ESCOUADE_VOICE_WAV: WAV, ESCOUADE_VOICE_TRANSCRIPT: SAID, ESCOUADE_TEST_POLICIES: NO_DICTATION },
    preStart: { run: (root: string) => placeModel(root) },
  });

  test('shows the voice as not on this device, with why, and never offers it', async ({ app }) => {
    const page = app.page;
    await addProject(page, app.repo);
    await openMap(page);
    for (const id of ['dictation', 'voice']) {
      await expect(stop(page, id)).toHaveAttribute('data-status', 'absent');
      await expect(stop(page, id)).toContainText('non permis sur ce poste');
    }
    await expect(page.getByText(/^0 sur 15 · /)).toBeVisible();
    await stop(page, 'dictation').click();
    const detail = page.getByTestId('discover-detail');
    await expect(detail).toContainText('voice.dictation');
    await expect(detail.getByRole('button', { name: 'Essayer' })).toHaveCount(0);
  });
});
