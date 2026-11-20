// ESC-512 (ON-1b): « Ce que tu veux brancher », the station of the first launch after the starting settings, with the
// fake `claude` as Claude Code signed in on the machine. A data folder as a new installation leaves it. Run at both
// sizes (ESCOUADE_E2E_VIEWPORT=1024x728 and the default one).
//
// - The seed sorted (tests/fixtures/store/fake-store.mjs, its seed holding Trello): the station gone through with nothing
//   ticked installs nothing; Trello chosen later installs its plugin alone, from the seed, with no network.
// - The workspace's plugins (a build of development): a source of tickets connected on the fake Trello of
//   tests/fixtures/fake-trackers.mjs, by the form of Réglages › Services connectés, right in the station; the voice on the
//   machine, its model of a fake server downloaded, cancelled, downloaded again, then a sentence dictated from a WAV file
//   (ESCOUADE_VOICE_WAV, ESCOUADE_VOICE_TRANSCRIPT), sent to nobody.

import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

const SIGNED_IN = JSON.stringify({ claudeAiOauth: { accessToken: 'tok', refreshToken: 'ref', expiresAt: 4_102_444_800_000 } });
const NEW_INSTALLATION = { noAccounts: true, agentMode: '', allowBypass: false, telemetry: null };
const TRELLO = 'dev.escouade.trello';

/** Claude Code signed in on the machine, the rest as a new installation leaves it. */
function signedIn(root: string, more: Record<string, unknown> = {}) {
  const dir = path.join(root, 'claude');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, '.credentials.json'), SIGNED_IN);
  fs.writeFileSync(path.join(dir, '.claude.json'), JSON.stringify({ oauthAccount: { emailAddress: 'ada@atlas.dev' } }));
  return { ...NEW_INSTALLATION, ...more };
}

const read = (file: string) => JSON.parse(fs.readFileSync(file, 'utf8'));
/** The ids `plugins.json` lists. */
function installed(data: string): string[] {
  const file = path.join(data, 'plugins.json');
  if (!fs.existsSync(file)) return [];
  return (read(file).plugins as { id: string }[]).map((p) => p.id);
}

async function fitsAcross(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
}

/** From the harness to « Ce que tu veux brancher », the recommended choices taken by Entrée. */
async function toHookups(page: Page) {
  // The first station's question: its focus at the start is onboarding.spec.ts' to check; here the keys need it.
  const first = page.getByRole('heading', { level: 1, name: 'Avec quel harness tes agents travaillent-ils ?' });
  await expect(first).toBeVisible();
  await first.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { level: 1, name: 'Quand tes agents doivent-ils te demander ?' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { level: 1, name: 'Que veux-tu brancher ?' })).toBeFocused();
}

test.describe('the seed sorted', () => {
  let fake: { url: string; key: string; close: () => Promise<void> };
  test.beforeAll(async () => {
    fake = await startFakeStore(0);
    process.env.ESCOUADE_STORE_TEST_KEY = fake.key;
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-seed-'));
    await fetch(`${fake.url}/_fake/seed`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ dir, plugins: [{ id: TRELLO, version: '2.2.0' }] }),
    });
    process.env.ESCOUADE_SEED_DIR = dir;
  });
  test.afterAll(async () => {
    await fake?.close();
    delete process.env.ESCOUADE_STORE_TEST_KEY;
    delete process.env.ESCOUADE_SEED_DIR;
  });
  test.use({ preStart: { run: (root: string) => signedIn(root) } });

  test('gone through with nothing ticked, installs nothing; a source chosen later installs its plugin alone, from the seed', async ({
    app,
  }) => {
    let page = app.page;
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await toHookups(page);
    const cards = journey.getByRole('group', { name: 'Les branchements' }).getByRole('checkbox');
    await expect(cards).toHaveCount(3);
    for (const c of await cards.all()) await expect(c).toHaveAttribute('aria-checked', 'false');
    await fitsAcross(page);
    // « Continuer » with nothing ticked: each card passed, nothing installed, nothing asked of the store.
    const asked = async () =>
      ((await (await fetch(`${fake.url}/_fake/state`)).json()).requests as string[]).filter((r) => !r.includes('/_fake/'));
    const ofTrello = async () => (await asked()).filter((r) => r.includes(TRELLO));
    await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { level: 1, name: 'Où travaille ton premier agent ?' })).toBeVisible();
    const state = () => read(path.join(app.data, 'state.json'));
    await expect.poll(() => state().onboarding?.hookups).toEqual({ sources: 'skipped', voice: 'skipped', forge: 'skipped' });
    expect(installed(app.data)).not.toContain(TRELLO);
    const line = page.getByRole('navigation', { name: 'Les stations du premier lancement' });
    await expect(line.getByRole('listitem').nth(3)).toContainText('rien de branché');

    // Back to it: Trello chosen, its plugin from the seed, with no network; its form of Réglages, here.
    await page.getByRole('button', { name: 'Retour' }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Que veux-tu brancher ?' })).toBeVisible();
    await journey.getByRole('checkbox', { name: 'Une source de tickets' }).click();
    // Ticked: its choices, nothing installed yet.
    await expect(journey.getByRole('radiogroup', { name: 'Quelle source ?' })).toBeVisible();
    expect(installed(app.data)).not.toContain(TRELLO);
    await journey.getByRole('radio', { name: /Trello/ }).click();
    await expect(journey.getByRole('form', { name: 'Connexion à Trello' })).toBeVisible();
    await expect(journey).toContainText('Plugin Trello installé.');
    // Its plugin, from the seed: nothing asked of the store for it. (That it is the only one the station installs is
    // `onboarding::to_install`'s test and HookupsStation.test.ts': a build of development lists its workspace's here.)
    await expect.poll(() => installed(app.data)).toContain(TRELLO);
    expect(await ofTrello()).toEqual([]);
    page = app.page;
  });
});

test.describe('a source connected and the voice on the machine', () => {
  let trackers: { url: string; close: () => Promise<void> };
  const SAID = 'importe le ticket du tableau';
  const BASE = 147_951_465;
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  const PORT = base ? base + 8 : 20_000 + Math.floor(Math.random() * 9_000);
  let models: http.Server;
  const WAV = path.join(os.tmpdir(), `escouade-e2e-hookups-${process.pid}.wav`);

  test.beforeAll(async () => {
    trackers = await startFakeTrackers(0);
    process.env.ESCOUADE_TRELLO_API = `${trackers.url}/1`;
    // A tone, as the app hears it in place of a microphone; the fake transcriber gives SAID whatever it hears.
    const rate = 16_000;
    const n = rate * 20;
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
    fs.writeFileSync(WAV, buf);
    // The model « base », of its size, its bytes not the model's, slowly enough (about 3 s) for its progress to show.
    models = http.createServer((_req, res) => {
      res.writeHead(200, { 'Content-Length': String(BASE) });
      const chunk = Buffer.alloc(1024 * 1024, 7);
      let sent = 0;
      const next = () => {
        if (res.destroyed) return;
        if (sent >= BASE) return res.end();
        const part = chunk.subarray(0, Math.min(chunk.length, BASE - sent));
        sent += part.length;
        res.write(part);
        setTimeout(next, 20);
      };
      next();
    });
    await new Promise<void>((r) => models.listen(PORT, '127.0.0.1', r));
  });
  test.afterAll(async () => {
    await trackers?.close();
    await new Promise((r) => models.close(r));
    delete process.env.ESCOUADE_TRELLO_API;
    fs.rmSync(WAV, { force: true });
  });
  test.use({
    appEnv: {
      ESCOUADE_VOICE_WAV: WAV,
      ESCOUADE_VOICE_TRANSCRIPT: SAID,
      ESCOUADE_VOICE_MODELS_URL: `http://127.0.0.1:${PORT}/models`,
    },
    preStart: { run: (root: string) => signedIn(root) },
  });

  test('connects Trello by Réglages’ form, and offers Whisper’s model (said, followed, cancelled, nothing kept that does not check)', async ({
    app,
  }) => {
    test.setTimeout(180_000);
    const { page } = app;
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await toHookups(page);

    // A source of tickets: Trello, its key and its token, checked by the fake Trello.
    await journey.getByRole('checkbox', { name: 'Une source de tickets' }).click();
    const sources = journey.getByRole('radiogroup', { name: 'Quelle source ?' });
    await expect(sources.getByRole('radio', { name: /Trello/ })).toContainText(
      'Les cartes d’un tableau Trello · une clé et un jeton d’API',
    );
    await sources.getByRole('radio', { name: /Trello/ }).click();
    const form = journey.getByRole('form', { name: 'Connexion à Trello' });
    await form.getByLabel("Clé d'API").fill('cle');
    await form.getByLabel('Jeton').fill('jeton');
    await form.getByRole('button', { name: 'Connecter' }).click();
    await expect(journey.getByText('Connecté · @ada')).toBeVisible();
    await fitsAcross(page);

    // The voice: Whisper on the machine, its weight said before anything is downloaded.
    await journey.getByRole('checkbox', { name: 'La voix' }).click();
    await journey.getByRole('radio', { name: /Whisper, sur ta machine/ }).click();
    await expect(journey).toContainText('Le modèle « base » : 141 Mo à télécharger une fois');
    const dir = path.join(app.data, 'voice', 'models');
    const has = () => fs.existsSync(path.join(dir, 'ggml-base.bin'));
    expect(has()).toBe(false);
    // Downloaded, followed, cancelled: nothing kept.
    await journey.getByRole('button', { name: 'Télécharger (141 Mo)' }).click();
    await expect(journey.getByRole('progressbar')).toBeVisible();
    await journey.getByRole('button', { name: 'Annuler le téléchargement' }).click();
    await expect(journey.getByRole('button', { name: 'Télécharger (141 Mo)' })).toBeVisible();
    expect(has()).toBe(false);
    // Downloaded to its end: the fake server's bytes are not the model's, so its checksum refuses them; nothing kept,
    // as Réglages › Voix (voice.spec.ts).
    await journey.getByRole('button', { name: 'Télécharger (141 Mo)' }).click();
    await expect(journey.getByRole('progressbar')).toBeVisible();
    await expect(journey.getByRole('button', { name: 'Télécharger (141 Mo)' })).toBeVisible({ timeout: 60_000 });
    expect(has()).toBe(false);

    // « Continuer »: the source plugged in (the core reads its account), the voice not ready and the code host passed:
    // they wait in the discovery.
    await journey.getByRole('button', { name: /^Continuer/ }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Où travaille ton premier agent ?' })).toBeVisible();
    const state = () => read(path.join(app.data, 'state.json'));
    await expect.poll(() => state().onboarding?.hookups?.voice).toBe('skipped');
    expect(state().onboarding.hookups.forge).toBe('skipped');
    const line = page.getByRole('navigation', { name: 'Les stations du premier lancement' });
    await expect(line.getByRole('listitem').nth(3)).toContainText('1 branché');
  });

  test.describe('its model on the disk', () => {
    test.use({
      preStart: {
        run: (root: string) => {
          // The model the app uses, a file of its size (the fake transcriber reads nothing of it), as voice.spec.ts.
          const dir = path.join(root, 'data', 'voice', 'models');
          fs.mkdirSync(dir, { recursive: true });
          const fd = fs.openSync(path.join(dir, 'ggml-base.bin'), 'w');
          fs.ftruncateSync(fd, BASE);
          fs.closeSync(fd);
          return signedIn(root);
        },
      },
    });

    test('says Whisper ready, and dictates a sentence into the station’s field, sent to nobody', async ({ app }) => {
      const { page } = app;
      const journey = page.getByRole('region', { name: 'Premier lancement' });
      await toHookups(page);
      await journey.getByRole('checkbox', { name: 'La voix' }).click();
      await journey.getByRole('radio', { name: /Whisper, sur ta machine/ }).click();
      await expect(journey.getByText('Modèle « base » prêt')).toBeVisible();
      const said = journey.getByRole('textbox', { name: 'Ta phrase dictée' });
      await journey.getByRole('button', { name: 'Dicter' }).click();
      const panel = page.locator('[data-voice-phase="listening"]');
      await expect(panel).toBeVisible();
      // The WAV heard for a second, then the dictation ended.
      await expect(panel).toContainText('0:01');
      await journey.getByRole('button', { name: 'Finir la dictée' }).click();
      await expect(said).toHaveValue(SAID);
      await fitsAcross(page);
      // « Continuer »: the voice plugged in, the two others passed.
      await journey.getByRole('button', { name: /^Continuer/ }).click();
      await expect(page.getByRole('heading', { level: 1, name: 'Où travaille ton premier agent ?' })).toBeVisible();
      const state = () => read(path.join(app.data, 'state.json'));
      await expect.poll(() => state().onboarding?.hookups).toEqual({ sources: 'skipped', voice: 'done', forge: 'skipped' });
      // The voice as Réglages › Voix sets it: Whisper on the machine; no agent made, nothing sent to one.
      expect(read(path.join(app.data, 'settings.json')).voice?.transcription ?? 'local').toBe('local');
      expect(read(path.join(app.data, 'state.json')).agents ?? []).toEqual([]);
    });
  });
});

test.describe('a device whose policy turns the dictation off', () => {
  test.use({
    appEnv: async ({}, use) => {
      const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-policies-'));
      const file = path.join(dir, 'policies.json');
      fs.writeFileSync(file, JSON.stringify({ 'voice.dictation': false }));
      await use({ ESCOUADE_TEST_POLICIES: file });
      fs.rmSync(dir, { recursive: true, force: true });
    },
    preStart: { run: (root: string) => signedIn(root) },
  });

  test('shows the voice locked, with who locks it; the other cards stay', async ({ app }) => {
    const { page } = app;
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await toHookups(page);
    const voice = journey.getByRole('checkbox', { name: 'La voix' });
    await expect(voice).toHaveAttribute('aria-disabled', 'true');
    await expect(journey).toContainText('Verrouillé · Imposé par la stratégie de ce poste');
    // Espace on it ticks nothing.
    await voice.focus();
    await page.keyboard.press(' ');
    await expect(voice).toHaveAttribute('aria-checked', 'false');
    await expect(journey.getByRole('checkbox', { name: 'Une source de tickets' })).not.toHaveAttribute('aria-disabled', 'true');
  });
});

test.describe('in English', () => {
  test.use({ language: 'en', preStart: { run: (root: string) => signedIn(root) } });

  test('asks what to plug in, nothing ticked', async ({ app }) => {
    const { page } = app;
    await expect(page.getByRole('heading', { level: 1, name: 'Which harness will your agents run on?' })).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { level: 1, name: 'When should your agents ask you?' })).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { level: 1, name: 'What would you like to plug in?' })).toBeFocused();
    const cards = page.getByRole('group', { name: 'Hookups' }).getByRole('checkbox');
    await expect(cards).toHaveText(['A ticket source', 'Voice', 'A code host']);
    for (const c of await cards.all()) await expect(c).toHaveAttribute('aria-checked', 'false');
    await fitsAcross(page);
  });
});
