// ESC-665, « Sensibilité du micro » and its tester in Réglages › Voix: no microphone on CI, so the app hears a WAV of a
// spoken question in place of the microphone (`ESCOUADE_VOICE_WAV`, debug builds only), between silences. At both sizes
// of the window (a wide screen, the CI runner's), the tester shows the level, the line of the threshold that the slider
// moves at once, and « Voix détectée » lit by the question then out after its silence; the slider is kept on the machine.
// With `ESCOUADE_E2E_SHOTS`, a capture of the tester, voice detected, in both themes at both sizes.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Locator, Page } from '@playwright/test';
import { expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
/** « Ajoute un test pour la page des réglages, puis lance la suite. », said by Piper, between silences. */
const QUESTION = path.join(ROOT, 'src-tauri', 'src', 'voice', 'testdata', 'question-fr.wav');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900 },
  { width: 1028, height: 779 },
];

/** Moves a slider as a drag would: its value, then the events the page listens to. */
async function slide(slider: Locator, value: number) {
  await slider.evaluate((el, v) => {
    const input = el as HTMLInputElement;
    input.value = String(v);
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }, value);
}

const left = (line: Locator) => line.evaluate((el) => parseFloat((el as HTMLElement).style.left));

async function shoot(page: Page, dialog: Locator, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await dialog.locator('[data-mic-tester]').scrollIntoViewIfNeeded();
  for (const t of ['light', 'dark'] as const) {
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
      document.documentElement.style.colorScheme = t;
    }, t);
    await page.screenshot({ path: path.join(SHOTS, `mic-tester-${name}-${t}.png`) });
  }
}

test.describe('« Sensibilité du micro » and its tester, from a WAV file', () => {
  test.use({ appEnv: { ESCOUADE_VOICE_WAV: QUESTION } });

  test('the tester shows the level, the line moved by the slider, and « Voix détectée » lit then out, at both sizes', async ({ app }) => {
    const { page } = app;
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await page.keyboard.press('Control+,');
      const dialog = await openSettingsPage(page, 'Voix');
      const slider = dialog.getByRole('slider', { name: 'Sensibilité du micro' });
      await slider.scrollIntoViewIfNeeded();
      await expect(slider).toBeVisible();
      // It says what it acts on, and that the dictation does not depend on it.
      await expect(dialog.getByText(/La dictée n’en dépend pas/)).toBeVisible();
      const test_ = dialog.getByRole('button', { name: 'Tester' });
      // Whole in the window, the narrow one too.
      const box = (await test_.boundingBox())!;
      expect(box.x + box.width).toBeLessThanOrEqual(size.width);
      await test_.click();
      const tester = dialog.getByRole('group', { name: 'Testeur du micro' });
      await expect(tester).toBeVisible();
      await expect(tester).toContainText('Rien de ce que le micro entend n’est gardé ni envoyé');
      const meter = tester.getByRole('meter', { name: 'Niveau de ta voix' });
      const witness = tester.locator('[data-voice-heard]');
      // The question is heard: the level rises, the light comes on.
      await expect(witness).toHaveAttribute('data-voice-heard', 'true', { timeout: 15_000 });
      await expect(witness).toHaveText('Voix détectée');
      expect(Number(await meter.getAttribute('aria-valuenow'))).toBeGreaterThan(0);
      await shoot(page, dialog, `${size.width}x${size.height}`);
      await page.evaluate(() => {
        document.documentElement.dataset.theme = 'light';
        document.documentElement.style.colorScheme = 'light';
      });
      // The slider moves the line at once, down when more sensitive, up when less.
      const line = tester.locator('[data-threshold]');
      const at = await left(line);
      await slide(slider, 5);
      await expect.poll(() => left(line)).toBeLessThan(at);
      await expect(dialog.locator('[data-sensitivity-value]')).toHaveText('+5');
      await slide(slider, -5);
      await expect.poll(() => left(line)).toBeGreaterThan(at);
      await slide(slider, 0);
      await expect.poll(() => left(line)).toBeCloseTo(at, 1);
      // The silence after it: the light goes out.
      await expect(witness).toHaveAttribute('data-voice-heard', 'false', { timeout: 15_000 });
      await expect(witness).toHaveText('Pas de voix');
      // « Arrêter »: the tester goes.
      await dialog.getByRole('button', { name: 'Arrêter' }).click();
      await expect(tester).toHaveCount(0);
      await page.keyboard.press('Escape');
      await expect(dialog).toHaveCount(0);
    }
    // Kept on the machine, with the other settings; never sent by the tester.
    await page.keyboard.press('Control+,');
    const dialog = await openSettingsPage(page, 'Voix');
    const slider = dialog.getByRole('slider', { name: 'Sensibilité du micro' });
    await slide(slider, 2);
    await expect
      .poll(() => {
        const file = path.join(app.data, 'settings.json');
        return fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, 'utf8')).voice?.speech?.sensitivity : undefined;
      })
      .toBe(2);
    // Leaving the page stops the tester.
    await dialog.getByRole('button', { name: 'Tester' }).click();
    await expect(dialog.getByRole('group', { name: 'Testeur du micro' })).toBeVisible();
    await openSettingsPage(page, 'Général');
    await openSettingsPage(page, 'Voix');
    await expect(dialog.getByRole('group', { name: 'Testeur du micro' })).toHaveCount(0);
    await expect(dialog.getByRole('button', { name: 'Tester' })).toBeVisible();
  });
});
