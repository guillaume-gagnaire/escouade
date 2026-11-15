// The Overseer's screen (ESC-468), in the real app with the fake claude: its global shortcut set in Réglages › Overseer
// and pressed (a sandbox records it instead of the system's keys, `overseer_hotkey_press` presses it as the system
// would), its button; a ticket made and started by what is typed, its card in the orbit, its news in the discussion
// with ready answers; an instruction denied by its own words refused, the user asked to say it plainly. Its voice by
// the fake voice of the end-to-end tests: a WAV in place of the microphone, a fixed transcript, a voice that writes what
// it says. With ESCOUADE_E2E_SHOTS, its captures in both themes, at the size of the run (ESCOUADE_E2E_VIEWPORT).

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const QUESTION = path.join(ROOT, 'src-tauri', 'src', 'voice', 'testdata', 'question-fr.wav');
const SAID = 'Ajoute un test pour la page des réglages, puis lance la suite.';
const VOICES = JSON.parse(fs.readFileSync(path.join(ROOT, 'crates', 'escouade-voice-local', 'src', 'speech', 'voices.json'), 'utf8')) as {
  id: string;
  onnxSize: number;
  jsonSize: number;
}[];
const BASE_SIZE = 147_951_465;
const SPOKEN_LOG = path.join(os.tmpdir(), `escouade-e2e-overseer-spoken-${process.pid}.txt`);

/** A file of `size` bytes that holds nothing (the fake transcriber and voice read nothing of it). */
function sparse(file: string, size: number) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const fd = fs.openSync(file, 'w');
  fs.ftruncateSync(fd, size);
  fs.closeSync(fd);
}

const screenOf = (page: Page) => page.locator('[data-overseer-screen]');

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** The screen captured in both themes, as « Thème » sets them, to be compared with the mock-ups of design/overseer. */
async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const size = await page.evaluate(() => ({ width: innerWidth, height: innerHeight }));
  for (const theme of ['dark', 'light'] as const) {
    await page.evaluate((t) => (document.documentElement.dataset.theme = t), theme);
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(SHOTS, `overseer-${name}-${theme}-${size.width}x${size.height}.png`) });
  }
  await page.evaluate(() => delete document.documentElement.dataset.theme);
}

async function ask(page: Page, text: string) {
  const field = screenOf(page).getByRole('textbox', { name: 'Demande à l’Overseer…' });
  await field.fill(text);
  await field.press('Enter');
  await expect(field).toHaveValue('');
}

/** Réglages › Overseer: turned on (with Escouade's tools); the page left open. */
async function turnOn(page: Page) {
  const settings = await openSettingsPage(page, 'Overseer');
  const on = settings.locator('#settings-panel').getByRole('switch', { name: 'Activer l’Overseer' });
  await on.click();
  await expect(on).toHaveAttribute('aria-checked', 'true');
  return settings;
}

/** Its global shortcut pressed, as the system would hand it over. */
const pressShortcut = (page: Page) =>
  page.evaluate(() =>
    (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
      'overseer_hotkey_press',
    ),
  );

test.describe('the Overseer’s screen', () => {
  test.use({ appEnv: { ESCOUADE_OVERSEER_HOLD_MS: '200', ESCOUADE_OVERSEER_GROUP_MS: '300' } });

  test('opened by its shortcut and its button: the orbit, the news with ready answers, a denied instruction refused', async ({ app }) => {
    test.setTimeout(240_000);
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: /^Kanban/ }).click();
    await page.getByRole('switch', { name: 'Pilote auto' }).click();

    // Its shortcut: the default one held once it is on, then changed by the keys pressed.
    const settings = await turnOn(page);
    const panel = settings.locator('#settings-panel');
    const shown = panel.locator('[data-overseer-shortcut]');
    await expect(shown).toHaveText(/Alt\+Maj\+O|⌥⌘O/);
    await expect(panel.locator('[data-shortcut-state]')).toHaveText('Actif');
    await panel.getByRole('button', { name: 'Changer' }).click();
    await expect(shown).toHaveText('Appuie sur le raccourci…');
    await page.keyboard.press('Control+Alt+KeyJ');
    await expect(shown).toHaveText(/Ctrl\+Alt\+J|⌃⌥J/);
    await expect(panel.locator('[data-shortcut-state]')).toHaveText('Actif');
    await closeSettings(settings);

    // The shortcut opens it from anywhere; Escape closes it; its button opens it again.
    const screen = screenOf(page);
    await expect(screen).toHaveCount(0);
    await pressShortcut(page);
    await expect(screen).toBeVisible();
    await expect(screen).toContainText('1 projet');
    await expect(screen.locator('[data-orbit]')).toContainText('Rien en cours');
    await page.keyboard.press('Escape');
    await expect(screen).toHaveCount(0);
    await page.locator('[data-overseer-button]').click();
    await expect(screen).toBeVisible();

    // Typed: a ticket made, shown under its answer with « Annuler ».
    await ask(page, 'Crée un ticket « Ajouter le fichier » sur demo-api');
    const typed = screen.locator('[data-user-message]').first();
    await expect(typed).toContainText('Crée un ticket « Ajouter le fichier » sur demo-api');
    await expect(typed).toContainText('tapé ·');
    await expect(screen.locator('[data-overseer-reply]').last()).toContainText('Ticket DEM-1 créé', { timeout: 60_000 });
    await expect(screen.locator('[data-action="create_ticket"]').getByRole('button', { name: 'Annuler' })).toBeVisible();

    // Started: its card in the orbit, under way, then done and waiting for the user, its news a card of the discussion.
    await ask(page, 'Lance DEM-1');
    await expect(screen.locator('[data-overseer-reply]').last()).toContainText('C’est parti : DEM-1 est lancé.', { timeout: 60_000 });
    const card = screen.locator('[data-orbit-task]').filter({ hasText: 'DEM-1' });
    await expect(card).toHaveAttribute('data-state', /working|waiting/, { timeout: 60_000 });
    const news = screen.locator('[data-news="review"][data-news-ticket="DEM-1"]');
    await expect(news).toContainText('DEM-1 · Ajouter le fichier', { timeout: 120_000 });
    await expect(card).toHaveAttribute('data-state', 'waiting');
    await expect(card).toContainText(/À valider|Terminé · à l’instant/);
    // A ready answer: sent as the user's message.
    await news.getByRole('button', { name: 'Valide-le' }).click();
    await expect(screen.locator('[data-user-message]').last()).toContainText('Valide DEM-1.');
    await expect(screen.locator('[data-overseer-reply]').last()).toContainText('Je regarde DEM-1 avant de le valider.', {
      timeout: 60_000,
    });

    // Denied by its own words: no instruction, the refusal shown, the user asked plainly.
    await ask(page, 'Ne merge surtout pas DEM-1, je relis d’abord');
    await expect(screen.locator('[data-overseer-reply]').last()).toContainText('Tu me confirmes que je peux merger DEM-1 ?', {
      timeout: 60_000,
    });
    const refused = screen.locator('[data-action="add_directive"][data-outcome="refused"]');
    await expect(refused).toContainText('près d’une négation');
    await expect(screen.locator('[data-directives-chip]')).toHaveText('Consignes · 0');
    // The card finished has melted toward the orb and settled back.
    if (SHOTS) await page.waitForTimeout(3000);
    await shots(page, 'discussion');

    // Clicking a card opens its ticket's agent, and the screen goes.
    await card.click();
    await expect(screen).toHaveCount(0);
  });
});

test.describe('the Overseer’s voice, by the fake voice', () => {
  test.use({
    appEnv: {
      ESCOUADE_OVERSEER_HOLD_MS: '200',
      ESCOUADE_OVERSEER_GROUP_MS: '300',
      ESCOUADE_VOICE_WAV: QUESTION,
      ESCOUADE_VOICE_TRANSCRIPT: SAID,
      ESCOUADE_SPEECH_FAKE: SPOKEN_LOG,
    },
    preStart: {
      run: (root: string) => {
        sparse(path.join(root, 'data', 'voice', 'models', 'ggml-base.bin'), BASE_SIZE);
        const v = VOICES.find((x) => x.id === 'fr_FR-siwis-medium')!;
        sparse(path.join(root, 'data', 'voice', 'voices', `${v.id}.onnx`), v.onnxSize);
        sparse(path.join(root, 'data', 'voice', 'voices', `${v.id}.onnx.json`), v.jsonSize);
        fs.rmSync(SPOKEN_LOG, { force: true });
      },
    },
  });

  test('the microphone held: the sentence said goes when let go, « dit », and its answer is read aloud', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false });
    const settings = await turnOn(page);
    await closeSettings(settings);
    await page.locator('[data-overseer-button]').click();
    const screen = screenOf(page);
    await expect(screen).toBeVisible();
    await expect(screen.getByText('Maintenir', { exact: false })).toBeVisible();
    // Held: it listens, the field a wave, the orb lit.
    const mic = screen.locator('[data-overseer-mic]');
    const box = (await mic.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await expect(screen.locator('[data-orbit]')).toHaveAttribute('data-voice', 'talking', { timeout: 15_000 });
    await expect(screen.locator('[data-wave]')).toBeVisible();
    await expect(screen.locator('[data-orb-label]')).toHaveText('Je t’écoute');
    await page.waitForTimeout(1200);
    await shots(page, 'talking');
    await page.mouse.up();
    // Sent as said, its words in the discussion.
    const said = screen.locator('[data-user-message]').filter({ hasText: SAID });
    await expect(said).toContainText('dit ·', { timeout: 30_000 });
    await expect(said).not.toContainText('escouade-voice');
    // Its answer, and its passage read by the voice.
    await expect(screen.locator('[data-overseer-reply]').last()).toContainText('Compris.', { timeout: 60_000 });
    await expect
      .poll(() => (fs.existsSync(SPOKEN_LOG) ? fs.readFileSync(SPOKEN_LOG, 'utf8') : ''), { timeout: 30_000 })
      .toContain('C’est noté, je m’en occupe.');
    // Read: it listens no more until the next press; Escape closes the screen and the microphone.
    await expect(screen.locator('[data-orbit]')).toHaveAttribute('data-voice', 'idle', { timeout: 30_000 });
    await page.keyboard.press('Escape');
    await expect(screen).toHaveCount(0);
    await expect.poll(() => fs.readFileSync(path.join(app.data, 'app.log'), 'utf8')).toMatch(/voice: conversation stopped/);
  });
});
