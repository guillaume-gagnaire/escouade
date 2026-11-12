// ESC-97, the voice conversation: no microphone nor speakers on CI, so the app hears a WAV of a spoken question in
// place of the microphone (`ESCOUADE_VOICE_WAV`, debug builds only), gives fixed texts in place of Whisper
// (`ESCOUADE_VOICE_TRANSCRIPT`, « a|b » one after the other), and reads aloud with a fake voice that writes what it says
// to a file and plays it in silence (`ESCOUADE_SPEECH_FAKE`). The fake claude answers a sentence said in a conversation
// with a passage to be read between <spoken> and </spoken>.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, test, type App } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
/** « Ajoute un test pour la page des réglages, puis lance la suite. », said by Piper, between silences. */
const QUESTION = path.join(ROOT, 'src-tauri', 'src', 'voice', 'testdata', 'question-fr.wav');
const SAID = 'Ajoute un test pour la page des réglages, puis lance la suite.';
const VOICES = JSON.parse(fs.readFileSync(path.join(ROOT, 'crates', 'escouade-voice-local', 'src', 'speech', 'voices.json'), 'utf8')) as {
  id: string;
  onnxSize: number;
  jsonSize: number;
}[];
const BASE_SIZE = 147_951_465;

/** A file of `size` bytes that holds nothing (the fake transcriber and voice read nothing of it). */
function sparse(file: string, size: number) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const fd = fs.openSync(file, 'w');
  fs.ftruncateSync(fd, size);
  fs.closeSync(fd);
}

/** Whisper's model and the French voice, in the app's data folder: no download offered. */
function placeModels(root: string) {
  sparse(path.join(root, 'data', 'voice', 'models', 'ggml-base.bin'), BASE_SIZE);
  const v = VOICES.find((x) => x.id === 'fr_FR-siwis-medium')!;
  sparse(path.join(root, 'data', 'voice', 'voices', `${v.id}.onnx`), v.onnxSize);
  sparse(path.join(root, 'data', 'voice', 'voices', `${v.id}.onnx.json`), v.jsonSize);
}

/** The WAV of the question said twice, `gap` seconds of silence between (a sentence, then the answer to a question). */
function twice(file: string, gap: number) {
  const wav = fs.readFileSync(QUESTION);
  const data = wav.indexOf(Buffer.from('data')) + 8;
  const rate = wav.readUInt32LE(24);
  const speech = wav.subarray(data);
  const silence = Buffer.alloc(Math.round(rate * gap) * 2);
  const body = Buffer.concat([speech, silence, speech]);
  const head = Buffer.from(wav.subarray(0, 44));
  head.writeUInt32LE(36 + body.length, 4);
  head.writeUInt32LE(body.length, 40);
  fs.writeFileSync(file, Buffer.concat([head, body]));
}

const said = (log: string) => (fs.existsSync(log) ? fs.readFileSync(log, 'utf8').split('\n').filter(Boolean) : []);

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

const button = (page: Page) => page.getByRole('button', { name: 'Conversation', exact: true });
const panel = (page: Page) => page.locator('[data-conversation-phase]');

const SPOKEN_LOG = path.join(os.tmpdir(), `escouade-e2e-spoken-${process.pid}.txt`);

test.describe('a voice conversation from a WAV file', () => {
  test.use({
    appEnv: { ESCOUADE_VOICE_WAV: QUESTION, ESCOUADE_VOICE_TRANSCRIPT: SAID, ESCOUADE_SPEECH_FAKE: SPOKEN_LOG },
    preStart: {
      run: (root: string) => {
        placeModels(root);
        fs.rmSync(SPOKEN_LOG, { force: true });
      },
    },
  });

  test('the question said is sent after its silence, the passage of the answer is read aloud, then it listens again', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    // Every subtitle shown, in order: the user's lasts until the agent answers, at once with the fake claude.
    await page.evaluate(() => {
      const w = window as unknown as { subs: string[] };
      w.subs = [];
      new MutationObserver(() => {
        for (const e of document.querySelectorAll<HTMLElement>('[data-subtitle]')) {
          const v = `${e.dataset.subtitle}: ${e.querySelector('p')?.textContent ?? ''}`;
          if (w.subs.at(-1) !== v) w.subs.push(v);
        }
      }).observe(document.body, { subtree: true, childList: true, characterData: true });
    });
    await button(page).click();
    // The conversation fills the window.
    await expect(page.getByRole('dialog', { name: /^Conversation vocale avec / })).toBeVisible();
    // The WAV is heard: a sentence, then its silence ends it.
    await expect(panel(page)).toHaveAttribute('data-conversation-phase', /listening|hearing/);
    await expect(panel(page)).toHaveAttribute('data-conversation-phase', 'hearing', { timeout: 10_000 });
    // Sent by itself, with the instruction to end with a passage to be read.
    await expect.poll(() => sent(app).find((m) => m.includes('réglages')) ?? '', { timeout: 20_000 }).toContain('<escouade-voice>');
    // The answer: the passage apart, the whole answer in the conversation.
    const passage = page.locator('[data-spoken]');
    await expect(passage).toContainText("C'est noté, je m'en occupe.");
    await expect(page.getByText('détail technique', { exact: false })).toBeVisible();
    await expect(page.getByText('<spoken>')).toHaveCount(0);
    // The user's message, as said: without the instruction.
    await expect(page.locator('.bubble').getByText(SAID)).toBeVisible();
    await expect(page.locator('[data-said-aloud]')).toBeVisible();
    await expect(page.getByText('Conversation vocale :')).toHaveCount(0);
    // Subtitles: what the user said, once transcribed, then the sentence of the voice as it is heard.
    await expect(page.locator('[data-subtitle="agent"]')).toContainText("C'est noté, je m'en occupe.", { timeout: 15_000 });
    await expect(page.locator('[data-subtitle]')).toHaveCount(1);
    expect(await page.evaluate(() => (window as unknown as { subs: string[] }).subs)).toEqual([
      `user: ${SAID}`,
      "agent: C'est noté, je m'en occupe.",
    ]);
    // Read by the voice, then listening again.
    await expect.poll(() => said(SPOKEN_LOG), { timeout: 15_000 }).toEqual(["C'est noté, je m'en occupe."]);
    await expect(panel(page)).toHaveAttribute('data-conversation-phase', 'listening', { timeout: 15_000 });
    const log = fs.readFileSync(path.join(app.data, 'app.log'), 'utf8');
    expect(log).toMatch(/voice: conversation listening at 16000 Hz/);
    // Escape: the panel goes and the microphone closes.
    await page.keyboard.press('Escape');
    await expect(panel(page)).toHaveCount(0);
    await expect(button(page)).toHaveAttribute('aria-pressed', 'false');
    await expect.poll(() => fs.readFileSync(path.join(app.data, 'app.log'), 'utf8')).toMatch(/voice: conversation stopped/);
  });
});

const TWICE = path.join(os.tmpdir(), `escouade-e2e-twice-${process.pid}.wav`);
twice(TWICE, 12);

test.describe('a question of the agent answered aloud', () => {
  test.use({
    appEnv: {
      ESCOUADE_VOICE_WAV: TWICE,
      ESCOUADE_VOICE_TRANSCRIPT: 'Crée la base de données|La deuxième.',
      ESCOUADE_SPEECH_FAKE: SPOKEN_LOG,
    },
    preStart: {
      run: (root: string) => {
        placeModels(root);
        fs.rmSync(SPOKEN_LOG, { force: true });
      },
    },
  });

  test('the question is read without its choices, the option said answers it, and the answer is read', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await button(page).click();
    await expect.poll(() => said(SPOKEN_LOG)[0] ?? '', { timeout: 25_000 }).toBe('Quelle base de données?');
    // « La deuxième »: SQLite, answered for the user.
    await expect.poll(() => said(SPOKEN_LOG).at(-1), { timeout: 30_000 }).toBe('Tu as choisi SQLite.');
    await expect(page.getByText('Tu as choisi SQLite.').first()).toBeVisible();
    await expect(panel(page)).toHaveAttribute('data-conversation-phase', 'listening', { timeout: 15_000 });
    // « Arrêter » closes it too.
    await page
      .getByRole('dialog')
      .getByRole('button', { name: /Arrêter/ })
      .click();
    await expect(panel(page)).toHaveCount(0);
  });
});

/** The WAV of the question said once, then again after each silence of `gaps` (seconds): a sentence, then answers. */
function repeated(file: string, gaps: number[]) {
  const wav = fs.readFileSync(QUESTION);
  const data = wav.indexOf(Buffer.from('data')) + 8;
  const rate = wav.readUInt32LE(24);
  const speech = wav.subarray(data);
  const body = Buffer.concat([speech, ...gaps.flatMap((gap) => [Buffer.alloc(Math.round(rate * gap) * 2), speech])]);
  const head = Buffer.from(wav.subarray(0, 44));
  head.writeUInt32LE(36 + body.length, 4);
  head.writeUInt32LE(body.length, 40);
  fs.writeFileSync(file, Buffer.concat([head, body]));
}

const FIVE = path.join(os.tmpdir(), `escouade-e2e-five-${process.pid}.wav`);
repeated(FIVE, [12, 7, 7, 7]);

test.describe('a request of three questions asked aloud one at a time (ESC-131)', () => {
  test.use({
    appEnv: {
      ESCOUADE_VOICE_WAV: FIVE,
      // The sentence, then an answer not understood (a rank beyond the list), then the three answers.
      ESCOUADE_VOICE_TRANSCRIPT: 'Configure le projet|La cinquième.|SQLite.|La première et la troisième.|Escouade-front',
      ESCOUADE_SPEECH_FAKE: SPOKEN_LOG,
    },
    preStart: {
      run: (root: string) => {
        placeModels(root);
        fs.rmSync(SPOKEN_LOG, { force: true });
      },
    },
  });

  test('each question is said without its choices, shown with them, answered by what is said, ticked in the card; the three answers go together', async ({
    app,
  }) => {
    test.setTimeout(180_000);
    const { page } = app;
    await addProject(page, app.repo);
    await button(page).click();
    const ask = page.locator('[data-conversation-ask]');
    // The first question: said, its choices shown under the orb with their ranks.
    await expect.poll(() => said(SPOKEN_LOG), { timeout: 30_000 }).toContain('Question 1 sur 3.');
    await expect(ask).toHaveAttribute('data-conversation-ask', '0');
    await expect(ask.getByRole('button', { name: /PostgreSQL/ })).toBeVisible();
    await expect(ask.getByRole('button', { name: /SQLite/ })).toBeVisible();
    await expect(ask).toContainText('Autre : dis ta réponse');
    // « La cinquième »: not understood, asked again.
    await expect.poll(() => said(SPOKEN_LOG).some((s) => /pas compris/.test(s)), { timeout: 30_000 }).toBe(true);
    // « SQLite »: ticked in the card, then the second question.
    await expect(ask).toHaveAttribute('data-conversation-ask', '1', { timeout: 30_000 });
    const ticked = page.locator('[data-testid="question-pending"] .opt.on');
    await expect(ticked).toHaveCount(1);
    await expect(ticked.first()).toContainText('SQLite');
    await expect(ask).toContainText('Plusieurs réponses possibles');
    // A capture of the overlay asking (`ESCOUADE_E2E_SHOT`: where to write it).
    if (process.env.ESCOUADE_E2E_SHOT) await page.screenshot({ path: process.env.ESCOUADE_E2E_SHOT });
    // « La première et la troisième », then the third, answered freely.
    await expect(ask).toHaveAttribute('data-conversation-ask', '2', { timeout: 30_000 });
    await expect(ticked).toHaveCount(3);
    await expect(ticked.nth(1)).toContainText('Unitaires');
    await expect(ticked.nth(2)).toContainText('Lint');
    // The three answers together, as the card's button sends them: the agent says them back.
    // (In the order of the backend's map, by question.)
    await expect.poll(() => said(SPOKEN_LOG).at(-1), { timeout: 40_000 }).toMatch(/^Tu as choisi /);
    const passage = said(SPOKEN_LOG).at(-1)!;
    expect(
      passage
        .split(/ *; */)
        .map((s) => s.replace(/^Tu as choisi |\.$/g, ''))
        .sort(),
    ).toEqual(['Escouade-front', 'SQLite', 'Unitaires, Lint']);
    const spoken = said(SPOKEN_LOG);
    expect(spoken.filter((s) => s === 'Question 1 sur 3.')).toHaveLength(2);
    expect(spoken).toContain('Question 2 sur 3.');
    expect(spoken).toContain('Question 3 sur 3.');
    expect(spoken).toContain('Plusieurs réponses possibles.');
    // The voice never read the choices.
    expect(spoken.join('\n')).not.toMatch(/PostgreSQL|Unitaires, E2E|web-shop|Les choix sont/);
    await expect(panel(page)).toHaveAttribute('data-conversation-phase', 'listening', { timeout: 15_000 });
    await expect(ask).toHaveCount(0);
    await page.keyboard.press('Escape');
    await expect(panel(page)).toHaveCount(0);
  });
});

const realModel = process.env.ESCOUADE_E2E_WHISPER_MODEL;
const realVoice = process.env.ESCOUADE_E2E_PIPER_VOICE;

test.describe('a voice conversation with the real Whisper and the real Piper', () => {
  test.skip(!realModel || !realVoice, 'ESCOUADE_E2E_WHISPER_MODEL and ESCOUADE_E2E_PIPER_VOICE (fr_FR-siwis-medium.onnx) not given');
  test.use({
    appEnv: { ESCOUADE_VOICE_WAV: QUESTION, ESCOUADE_SPEECH_SILENT: '1' },
    preStart: {
      run: (root: string) => {
        const models = path.join(root, 'data', 'voice', 'models');
        const voices = path.join(root, 'data', 'voice', 'voices');
        fs.mkdirSync(models, { recursive: true });
        fs.mkdirSync(voices, { recursive: true });
        fs.copyFileSync(realModel!, path.join(models, 'ggml-base.bin'));
        fs.copyFileSync(realVoice!, path.join(voices, 'fr_FR-siwis-medium.onnx'));
        fs.copyFileSync(`${realVoice}.json`, path.join(voices, 'fr_FR-siwis-medium.onnx.json'));
      },
    },
  });

  test('transcribes the question on the machine, sends it, and reads the passage with Piper', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await button(page).click();
    await expect.poll(() => sent(app).find((m) => m.includes('<escouade-voice>')) ?? '', { timeout: 60_000 }).toMatch(/r[ée]glages/i);
    await expect(page.locator('[data-spoken]')).toBeVisible({ timeout: 30_000 });
    // A capture of the panel while Piper speaks (`ESCOUADE_E2E_SHOT`: where to write it).
    await expect(page.locator('[data-subtitle="agent"]')).toBeVisible({ timeout: 30_000 });
    if (process.env.ESCOUADE_E2E_SHOT) await page.screenshot({ path: process.env.ESCOUADE_E2E_SHOT });
    await expect
      .poll(() => fs.readFileSync(path.join(app.data, 'app.log'), 'utf8'), { timeout: 60_000 })
      .toMatch(/speech: voice loaded in/);
    await expect(panel(page)).toHaveAttribute('data-conversation-phase', 'listening', { timeout: 30_000 });
    const log = fs.readFileSync(path.join(app.data, 'app.log'), 'utf8');
    console.log(
      log
        .split('\n')
        .filter((l) => /voice:|speech:/.test(l))
        .join('\n'),
    );
  });
});
