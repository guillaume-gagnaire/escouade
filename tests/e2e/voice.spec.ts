// ESC-96, the dictation: no microphone on CI, so the app listens to a WAV file in its place (`ESCOUADE_VOICE_WAV`, debug
// builds only), and gives a fixed text in place of Whisper (`ESCOUADE_VOICE_TRANSCRIPT`), unless a real model and a WAV
// of speech are given (`ESCOUADE_E2E_WHISPER_MODEL`, `ESCOUADE_E2E_VOICE_WAV`: run by hand). Its models come from a
// fake server of the test's (`ESCOUADE_VOICE_MODELS_URL`), on a port of `ESCOUADE_PORT_BASE` when it is set.

import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, type App, openSettingsPage } from './fixture';

/** The size of « base » in the catalog of the app (`voice::models::CATALOG`). */
const BASE_SIZE = 147_951_465;
const SAID = 'ajoute un test pour la page des réglages';

/** A WAV of a voice-like tone, 16-bit mono at 44.1 kHz: what the app hears in place of a microphone. */
function toneWav(file: string, secs = 30) {
  const rate = 44_100;
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
  for (let i = 0; i < n; i++) {
    // 220 Hz with a slow swell, as a voice rises and falls.
    const s = Math.sin((2 * Math.PI * 220 * i) / rate) * (0.3 + 0.2 * Math.sin((2 * Math.PI * i) / rate));
    buf.writeInt16LE(Math.round(s * 32767), 44 + i * 2);
  }
  fs.writeFileSync(file, buf);
}

const WAV = path.join(os.tmpdir(), `escouade-e2e-voice-${process.pid}.wav`);
toneWav(WAV);

const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
const PORT = base ? base + 7 : 20_000 + Math.floor(Math.random() * 9_000);
const MODELS_URL = `http://127.0.0.1:${PORT}/models`;

/** What the fake server of the models was asked, by path. */
const asked: string[] = [];
let server: http.Server;

test.beforeAll(async () => {
  // Serves a « model » of the right size whose bytes are not the model's, slowly (about 3 s), for the progress to show.
  server = http.createServer((req, res) => {
    asked.push(req.url ?? '');
    res.writeHead(200, { 'Content-Length': String(BASE_SIZE) });
    const chunk = Buffer.alloc(1024 * 1024, 7);
    let sent = 0;
    const next = () => {
      if (res.destroyed) return;
      if (sent >= BASE_SIZE) return res.end();
      const part = chunk.subarray(0, Math.min(chunk.length, BASE_SIZE - sent));
      sent += part.length;
      res.write(part);
      setTimeout(next, 20);
    };
    next();
  });
  await new Promise<void>((r) => server.listen(PORT, '127.0.0.1', r));
});

test.afterAll(async () => {
  await new Promise((r) => server.close(r));
});

/** Puts the model the app will use in its data folder: a file of its size (the fake transcriber reads nothing of it). */
function placeModel(root: string, from?: string) {
  const dir = path.join(root, 'data', 'voice', 'models');
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, 'ggml-base.bin');
  if (from) fs.copyFileSync(from, file);
  else {
    const fd = fs.openSync(file, 'w');
    fs.ftruncateSync(fd, BASE_SIZE);
    fs.closeSync(fd);
  }
}

const mic = (page: Page) => page.getByRole('button', { name: /^(Dicter|Finir la dictée)$/ }).first();
const panel = (page: Page) => page.locator('[data-voice-phase]');

/** What the fake CLI read on its stdin: the messages sent to the agent. */
function sent(app: App): string[] {
  const file = path.join(app.root, 'fake-claude.stdin.jsonl');
  if (!fs.existsSync(file)) return [];
  return fs
    .readFileSync(file, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.stringify(JSON.parse(l)));
}

test.describe('dictation from a WAV file', () => {
  test.use({
    appEnv: { ESCOUADE_VOICE_WAV: WAV, ESCOUADE_VOICE_TRANSCRIPT: SAID, ESCOUADE_VOICE_MODELS_URL: MODELS_URL },
    preStart: { run: (root: string) => placeModel(root) },
  });

  test('a click listens to the WAV with its level, a second puts the text at the cursor, never sent; the shortcut too', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    const box = page.getByRole('textbox').last();
    await box.fill('Merci, puis ');
    await mic(page).click();
    await expect(panel(page)).toHaveAttribute('data-voice-phase', 'listening');
    await expect(mic(page)).toHaveAttribute('aria-pressed', 'true');
    // The WAV is heard: the level rises, and the time goes on.
    await expect
      .poll(async () => Number(await page.getByRole('meter', { name: 'Niveau du micro' }).getAttribute('aria-valuenow')))
      .toBeGreaterThan(30);
    await expect(panel(page)).toContainText('0:01');
    await mic(page).click();
    await expect(box).toHaveValue(`Merci, puis ${SAID}`);
    await expect(panel(page)).toHaveCount(0);
    // The backend heard the file, at its rate.
    const log = fs.readFileSync(path.join(app.data, 'app.log'), 'utf8');
    expect(log).toMatch(/voice: listening at 44100 Hz/);
    expect(log).toMatch(/voice: \d+\.\d s heard/);
    // Nothing was sent by itself.
    await page.waitForTimeout(500);
    expect(sent(app).some((m) => m.includes('réglages'))).toBe(false);

    // Ctrl+Maj+Espace: once to start, once to end; the text goes at the cursor, after what was there.
    await box.click();
    await box.press('End');
    await page.keyboard.press('Control+Shift+Space');
    await expect(panel(page)).toHaveAttribute('data-voice-phase', 'listening');
    await page.waitForTimeout(600);
    await page.keyboard.press('Control+Shift+Space');
    await expect(box).toHaveValue(`Merci, puis ${SAID} ${SAID}`);
    // Escape calls one off: nothing is put in the field.
    await page.keyboard.press('Control+Shift+Space');
    await expect(panel(page)).toHaveAttribute('data-voice-phase', 'listening');
    await page.keyboard.press('Escape');
    await expect(panel(page)).toHaveCount(0);
    await expect(box).toHaveValue(`Merci, puis ${SAID} ${SAID}`);
    await expect.poll(() => fs.readFileSync(path.join(app.data, 'app.log'), 'utf8')).toMatch(/voice: listening called off/);
    // Sent by hand, it is the user's message.
    await box.press('Enter');
    await expect.poll(() => sent(app).some((m) => m.includes('réglages'))).toBe(true);
  });

  test('held down, it listens until it is let go', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    const box = page.getByRole('textbox').last();
    const b = (await mic(page).boundingBox())!;
    await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
    await page.mouse.down();
    await expect(panel(page)).toHaveAttribute('data-voice-phase', 'listening');
    await page.waitForTimeout(1200);
    await page.mouse.up();
    await expect(box).toHaveValue(SAID);
  });

  test('the listening stops when the window is closed', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await mic(page).click();
    await expect(panel(page)).toHaveAttribute('data-voice-phase', 'listening');
    // As the window's close button does: to the tray.
    await page.getByRole('button', { name: 'Fermer', exact: true }).first().click();
    await expect.poll(() => fs.readFileSync(path.join(app.data, 'app.log'), 'utf8')).toMatch(/voice: listening called off/);
  });
});

test.describe('dictation, the first time', () => {
  test.use({ appEnv: { ESCOUADE_VOICE_WAV: WAV, ESCOUADE_VOICE_TRANSCRIPT: SAID, ESCOUADE_VOICE_MODELS_URL: MODELS_URL } });

  test('offers the model with its size, shows its download, and keeps nothing whose checksum is wrong', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await mic(page).click();
    await expect(panel(page)).toContainText('Télécharger le modèle de transcription ?');
    await expect(panel(page)).toContainText(/« base » \(14\d Mo\)/);
    await panel(page).getByRole('button', { name: 'Télécharger' }).click();
    await expect(panel(page)).toHaveAttribute('data-voice-phase', 'downloading');
    await expect.poll(async () => Number(await page.getByRole('progressbar').getAttribute('aria-valuenow'))).toBeGreaterThan(0);
    // The bytes were not the model's: refused, and nothing kept.
    await expect(page.getByText(/somme de contrôle différente/)).toBeVisible({ timeout: 30_000 });
    expect(asked).toContain('/models/ggml-base.bin');
    const dir = path.join(app.data, 'voice', 'models');
    expect(fs.existsSync(dir) ? fs.readdirSync(dir) : []).toEqual([]);
    // Réglages › Voix: base is still to download.
    await page.keyboard.press('Control+,');
    await openSettingsPage(page, 'Voix');
    await expect(page.getByRole('radio', { name: /^base/ })).toBeChecked();
    await expect(page.getByRole('combobox', { name: 'Micro utilisé' })).toBeVisible();
  });
});

const realModel = process.env.ESCOUADE_E2E_WHISPER_MODEL;
const realWav = process.env.ESCOUADE_E2E_VOICE_WAV;

test.describe('dictation with the real Whisper', () => {
  test.skip(!realModel || !realWav, 'ESCOUADE_E2E_WHISPER_MODEL and ESCOUADE_E2E_VOICE_WAV not given');
  test.use({
    appEnv: { ESCOUADE_VOICE_WAV: realWav ?? '', ESCOUADE_VOICE_MODELS_URL: MODELS_URL },
    preStart: { run: (root: string) => placeModel(root, realModel) },
  });

  test('transcribes a WAV of speech on the machine into the composer', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await mic(page).click();
    await expect(panel(page)).toHaveAttribute('data-voice-phase', 'listening');
    await page.waitForTimeout(Number(process.env.ESCOUADE_E2E_VOICE_SECS ?? 7) * 1000);
    await mic(page).click();
    const box = page.getByRole('textbox').last();
    await expect.poll(() => box.inputValue(), { timeout: 60_000 }).toMatch(/réglages/i);
    console.log(`transcribed: ${await box.inputValue()}`);
  });
});
