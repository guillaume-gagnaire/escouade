// Codex (ESC-37): agents on the fake `codex app-server` (tests/fixtures/fake-codex.mjs), which plays Codex 0.163.0's
// JSON-RPC protocol and whose message picks what a turn does. What the app sent it is in its log (FAKE_CODEX_LOG).

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_CODEX, send, test, type App, openSettingsPage } from './fixture';

/** What the fake codex was sent, launch by launch: its arguments, then the messages of the app. */
function codexLog(
  app: App,
): { launch?: { argv: string[]; codexHome: string | null }; in?: { method?: string; params?: any; result?: any } }[] {
  const file = path.join(app.root, 'fake-codex.jsonl');
  if (!fs.existsSync(file)) return [];
  return fs
    .readFileSync(file, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));
}

/** The requests of `method` the app sent Codex. */
const requests = (app: App, method: string) =>
  codexLog(app)
    .map((e) => e.in)
    .filter((m) => m?.method === method);

/** A new agent on Codex, from the menu beside « Nouvel agent ». */
async function newCodexAgent(page: Page) {
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitemradio', { name: 'Codex' }).click();
  // Shown before anything is typed: a message typed sooner would go to the agent shown before it.
  await expect(page.locator('main.conv header.head')).toContainText('Codex');
  await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à Codex…' })).toBeVisible();
}

test.describe('an agent on Codex', () => {
  test.use({ appEnv: { ESCOUADE_CODEX: FAKE_CODEX } });

  test('goes with the model, the effort and the mode its account sets, among those of Codex (ESC-128)', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    // A first agent: Codex lists its models for its account (model/list).
    await newCodexAgent(page);
    await send(page, 'bonjour Codex');
    await expect(page.getByText('Bonjour, tu as dit : bonjour Codex')).toBeVisible();
    // Réglages › Comptes et harness: the fiche of the Codex account offers Codex's models, efforts and modes, none of Claude Code's.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(dialog, 'Comptes et harness');
    // No group « Nouveaux agents » for every harness any more.
    await openSettingsPage(dialog, 'Comptes et harness');
    await expect(dialog.getByRole('group', { name: 'Modèle par défaut' })).toHaveCount(0);
    await openSettingsPage(dialog, 'Comptes et harness');
    const codex = dialog.getByRole('list', { name: 'Comptes Codex' }).getByRole('listitem', { name: 'Principal' });
    const defaults = codex.getByRole('group', { name: 'Nouveaux agents' });
    const model = defaults.getByRole('combobox', { name: 'Modèle' });
    await expect(model.getByRole('option')).toHaveText(['Celui du harness (GPT-6-Luna)', 'GPT-6-Luna', 'GPT-5.6-Terra']);
    const mode = defaults.getByRole('combobox', { name: 'Mode' });
    await expect(mode.getByRole('option', { name: 'Plan' })).toHaveCount(1);
    await model.selectOption('gpt-5.6-terra');
    await defaults.getByRole('combobox', { name: 'Effort' }).selectOption('high');
    await mode.selectOption('plan');
    const settings = () => JSON.parse(fs.readFileSync(path.join(app.root, 'data', 'settings.json'), 'utf8'));
    await expect
      .poll(() => {
        const a = settings().accounts.find((x: { id: string }) => x.id === 'principal-codex');
        return [a?.defaultModel, a?.defaultEffort, a?.defaultMode];
      })
      .toEqual(['gpt-5.6-terra', 'high', 'plan']);
    // Claude Code's account keeps Claude Code's.
    const claude = dialog.getByRole('list', { name: 'Comptes Claude Code' }).getByRole('listitem', { name: 'Principal' });
    await expect(
      claude
        .getByRole('combobox', { name: 'Modèle' })
        .getByRole('option', { name: /Sonnet/ })
        .first(),
    ).toBeAttached();
    await page.keyboard.press('Escape');
    // A new agent on Codex goes with them.
    const before = requests(app, 'turn/start').length;
    await newCodexAgent(page);
    await send(page, 'bonjour encore');
    // In the mode Plan, it asks first: its turn started is what tells.
    await expect.poll(() => requests(app, 'turn/start').length).toBeGreaterThan(before);
    const start = requests(app, 'turn/start')[before]!.params;
    expect(start.model).toBe('gpt-5.6-terra');
    expect(start.effort).toBe('high');
    expect(start.collaborationMode.mode).toBe('plan');
  });

  test('is not offered by the menu « Harness » while its plugin is off, and is again once it is on (ESC-391)', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    const more = page.getByRole('button', { name: 'Autres façons de créer un agent' });
    await more.click();
    await expect(page.getByRole('menuitemradio', { name: /^Codex/ })).toBeVisible();
    await page.keyboard.press('Escape');
    // Its plugin turned off: its program found, its account there, not offered any more.
    const toggle = (on: boolean) =>
      page.evaluate(
        ([on]) =>
          (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
            'plugin_set_enabled',
            { id: 'dev.escouade.codex', on },
          ),
        [on] as const,
      );
    await toggle(false);
    await more.click();
    await expect(page.getByRole('menuitemradio', { name: /^Claude Code/ })).toBeVisible();
    await expect(page.getByRole('menuitemradio', { name: /^Codex/ })).toHaveCount(0);
    await page.keyboard.press('Escape');
    // Back on: offered, and its agent runs.
    await toggle(true);
    await newCodexAgent(page);
    await send(page, 'bonjour Codex');
    await expect(page.getByText('Bonjour, tu as dit : bonjour Codex')).toBeVisible();
  });

  test('offers every model of its account before any agent, read without a turn on every page, in the runner’s window (ESC-186)', async ({
    app,
  }) => {
    const { page } = app;
    // The runner's screen is 1024 × 768.
    await page.setViewportSize({ width: 1024, height: 728 });
    await addProject(page, app.repo);
    // Read at the start of the app: each page of model/list (the fake gives one model a page), no thread, no turn.
    await expect.poll(() => requests(app, 'model/list').map((r) => r?.params.cursor ?? null)).toEqual([null, '1']);
    expect(requests(app, 'thread/start')).toEqual([]);
    expect(requests(app, 'turn/start')).toEqual([]);
    // The fiche of the account offers them all.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(dialog, 'Comptes et harness');
    const codex = dialog.getByRole('list', { name: 'Comptes Codex' }).getByRole('listitem', { name: 'Principal' });
    const model = codex.getByRole('group', { name: 'Nouveaux agents' }).getByRole('combobox', { name: 'Modèle' });
    await expect(model.getByRole('option')).toHaveText(['Celui du harness (GPT-6-Luna)', 'GPT-6-Luna', 'GPT-5.6-Terra']);
    await page.keyboard.press('Escape');
    // So does the menu of a new agent's, before its first message: no stand-in « liste en cours de lecture ».
    await newCodexAgent(page);
    await page.getByRole('button', { name: /^Modèle/ }).click();
    const terra = page.getByRole('menuitemradio', { name: /GPT-5\.6-Terra/ });
    await expect(terra).toBeVisible();
    await expect(terra).toBeInViewport();
    await expect(page.getByRole('menuitemradio', { name: /GPT-6-Luna/ })).toBeVisible();
    await expect(page.getByText('liste en cours de lecture')).toHaveCount(0);
    await page.keyboard.press('Escape');
  });

  test('answers live, runs a command once allowed, changes a file with its diff, plans, steers, resumes and is copied', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newCodexAgent(page);
    // Its brand, since agents run on two harnesses now.
    await expect(page.locator('main.conv header.head')).toContainText('Codex');
    await send(page, 'bonjour Codex');
    await expect(page.getByText('Bonjour, tu as dit : bonjour Codex')).toBeVisible();
    // Started as Escouade drives Codex: its app-server, the plan tool given to the model, a thread in the agent's folder.
    // The agent's, not the bare app-servers that read its account's models and quota (ESC-186).
    const launch = codexLog(app).find((e) => e.launch && e.launch.argv.join(' ') !== 'app-server')!.launch!;
    expect(launch.argv.slice(0, 3)).toEqual(['app-server', '-c', 'tools.update_plan.enabled=true']);
    // The agent's (the readings of its account's models and quota ask for no capability).
    expect(requests(app, 'initialize').some((r) => r?.params.capabilities?.experimentalApi === true)).toBe(true);
    const start = requests(app, 'turn/start')[0]!.params;
    expect(start.approvalPolicy).toBe('on-request');
    expect(start.sandboxPolicy.type).toBe('workspaceWrite');
    expect(start.collaborationMode.mode).toBe('default');

    // The models and their efforts are those model/list gave.
    await page.getByRole('button', { name: /^Modèle/ }).click();
    await expect(
      page.getByRole('menuitemradio', { name: /GPT-5\.6-Terra/ }).or(page.getByRole('option', { name: /GPT-5\.6-Terra/ })),
    ).toBeVisible();
    await page.keyboard.press('Escape');
    // The modes say what they mean in Codex, sandbox included.
    await page.getByRole('button', { name: /^Mode/ }).click();
    await expect(page.getByText(/bac à sable en lecture seule/).first()).toBeVisible();
    await page.keyboard.press('Escape');

    // A command, run once allowed: its card says why Codex asks, and offers « Pour cette session ».
    await send(page, 'commande');
    const card = page.getByTestId('permission-pending');
    await expect(card).toContainText('rm -rf build');
    await expect(card).toContainText('Autorisez-vous la suppression du dossier build ?');
    await expect(card.getByRole('button', { name: 'Pour cette session' })).toBeVisible();
    await card.getByRole('button', { name: 'Autoriser' }).click();
    await expect(page.getByText('Commande exécutée.')).toBeVisible();
    await expect.poll(() => codexLog(app).some((e) => e.in?.result?.decision === 'accept')).toBe(true);

    // A file changed, its diff in the conversation, as Claude Code's Edit.
    await send(page, 'fichier');
    await expect(page.getByText('Fichier modifié.')).toBeVisible();
    const edit = page.locator('.tool', { hasText: 'app.ts' }).last();
    await expect(edit).toContainText('+1');
    await expect(edit).toContainText('−1');
    expect(fs.readFileSync(path.join(app.repo, 'src', 'app.ts'), 'utf8')).toBe('const a = 2;\n');

    // The « Plan » banner: the steps of the plan tool, then a subagent.
    await send(page, 'étapes');
    await expect(page.getByText('Les trois étapes sont faites.')).toBeVisible();
    const plan = page.locator('main.conv .plan');
    await expect(plan).toContainText('3/3');
    await send(page, 'sous-agent');
    await expect(page.getByText('Rapport reçu : le filtre refuse les PDF.')).toBeVisible();
    await expect(page.getByText('cause du bug').first()).toBeVisible();

    // A message sent while a turn runs goes into it (turn/steer).
    await send(page, 'lent');
    await expect(page.getByText('Je commence…')).toBeVisible();
    await send(page, 'ajoute aussi un test');
    await expect(page.getByText("J'ai bien reçu en cours de route : ajoute aussi un test")).toBeVisible();
    expect(requests(app, 'turn/steer').length).toBe(1);
    // And one interrupted.
    await send(page, 'lent');
    await expect(page.getByText('Je commence…').nth(1)).toBeVisible();
    await page.getByRole('button', { name: 'Stop' }).click();
    await expect.poll(() => requests(app, 'turn/interrupt').length).toBe(1);
    await expect(page.getByRole('button', { name: 'Stop' })).toHaveCount(0);

    // Restarted, the app resumes its thread.
    const page2 = await app.restart();
    await page2.locator('.card', { hasText: 'Codex' }).first().click();
    await send(page2, 'encore là ?');
    await expect(page2.getByText('Bonjour, tu as dit : encore là ?')).toBeVisible();
    const thread = requests(app, 'thread/start')[0]!;
    expect(requests(app, 'thread/resume').length).toBeGreaterThanOrEqual(1);
    // Copied: a thread of its own, forked from the original at its last turn.
    await page2.locator('.card', { hasText: 'Codex' }).first().click({ button: 'right' });
    await page2.getByRole('menuitem', { name: 'Dupliquer la conversation' }).click();
    await send(page2, 'et la copie ?');
    await expect(page2.getByText('Bonjour, tu as dit : et la copie ?')).toBeVisible();
    const fork = requests(app, 'thread/fork')[0]!;
    expect(fork.params.threadId).toMatch(/^thr-/);
    expect(fork.params.lastTurnId).toMatch(/^turn-/);
    expect(thread).toBeTruthy();
  });

  test('compacts its context by /compact from the composer’s menu, never through its model, in the runner’s window (ESC-205)', async ({
    app,
  }) => {
    const { page } = app;
    // The runner's screen is 1024 × 768.
    await page.setViewportSize({ width: 1024, height: 728 });
    await addProject(page, app.repo);
    await newCodexAgent(page);
    await send(page, 'bonjour Codex');
    await expect(page.getByText('Bonjour, tu as dit : bonjour Codex')).toBeVisible();
    await send(page, 'encore');
    await expect(page.getByText('Bonjour, tu as dit : encore')).toBeVisible();
    // The context gauge of its header, in % of the model's window.
    const gauge = page.locator('header.head .fig', { hasText: 'Contexte' }).locator('.ctx-meter i');
    const filled = async () => parseFloat(((await gauge.getAttribute('style')) ?? '').replace(/[^\d.]/g, '')) || 0;
    await expect.poll(filled).toBeGreaterThan(0);
    const before = await filled();
    // « / » offers Codex's one command there, `/compact`, and none of Claude Code's.
    const box = page.getByRole('textbox').last();
    await box.pressSequentially('/');
    const menu = page.getByRole('listbox');
    await expect(menu.getByRole('option')).toHaveCount(1);
    await expect(menu.getByRole('option')).toContainText('/compact');
    await expect(menu.getByRole('option')).toContainText('Compacter le contexte de la conversation');
    await expect(menu).toBeInViewport();
    await box.press('Enter');
    await expect(box).toHaveValue('/compact ');
    await box.press('Enter');
    // Its start, then its end; the gauge goes down.
    await expect(page.getByText('Compaction du contexte…')).toBeVisible();
    await expect(page.getByText('Contexte compacté')).toBeVisible();
    await expect.poll(filled).toBeLessThan(before);
    expect(requests(app, 'thread/compact/start')).toHaveLength(1);
    expect(requests(app, 'turn/start')).toHaveLength(2);
    await expect(page.getByText('tu as dit : /compact')).toHaveCount(0);
    // In the middle of a message too, never in a path.
    await box.pressSequentially('Relis src/');
    await expect(menu).toHaveCount(0);
    await box.pressSequentially(' puis /');
    await expect(menu.getByRole('option')).toContainText('/compact');
    await box.press('Escape');
    await box.fill('');
    // With an instruction, which Codex does not take: refused, the text kept.
    await box.fill('/compact garde les décisions');
    await box.press('Enter');
    await expect(page.getByText('Codex ne prend pas d’instruction pour /compact : envoie /compact seul.')).toBeVisible();
    await expect(box).toHaveValue('/compact garde les décisions');
    expect(requests(app, 'thread/compact/start')).toHaveLength(1);
  });

  test('a question of the mode Plan is answered on its card, and the plan approved starts its work', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newCodexAgent(page);
    await send(page, 'question');
    const question = page.getByTestId('question-pending');
    await expect(question).toContainText('Quelle base de données ?');
    await question.getByRole('button', { name: 'SQLite' }).click();
    await expect(page.getByText('Choix retenu : SQLite')).toBeVisible();
    await send(page, 'propose un plan');
    const plan = page.getByTestId('permission-pending');
    await expect(plan).toContainText('Codex propose un plan');
    await expect(plan).toContainText('Script de la date');
    await plan.getByRole('button', { name: 'Approuver le plan' }).click();
    await expect(page.getByText('Plan mis en œuvre.')).toBeVisible();
    // Its work out of Plan, in the mode it was in before it (ESC-277): Codex's collaboration mode `default`.
    const last = requests(app, 'turn/start').at(-1)!.params;
    expect(last.collaborationMode.mode).toBe('default');
    expect(last.input[0].text).toBe('Mets le plan en œuvre.');
  });
});

test.describe('a ticket on Codex', () => {
  test.use({ appEnv: { ESCOUADE_CODEX: FAKE_CODEX } });

  test('is launched on Codex by the autopilot, and its agent reports until it is ready to test', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: /^Kanban/ }).click();
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Ajouter le fichier [ok]');
    await page.getByRole('group', { name: 'Harness' }).getByRole('button').click();
    await page.getByRole('menuitemradio', { name: 'Codex', exact: true }).click();
    await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
    const review = page.getByRole('region', { name: 'À tester' });
    await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
    await expect(review.locator('.crit li.met')).toHaveCount(2);
    // Its protocol went to Codex as the instructions of its thread.
    const start = requests(app, 'thread/start')[0]!.params;
    expect(start.developerInstructions).toContain('DEM-1');
    expect(start.cwd).toContain('demo-api');
    const worktree = start.cwd as string;
    expect(fs.readFileSync(path.join(worktree, 'dem-1.txt'), 'utf8')).toMatch(/^Boucle \d+\n$/);
  });
});

// The voice conversation (ESC-97) whatever the harness: the passage to be read aloud is taken out of the agent's text by
// the common part of the conversation, for Codex as for Claude Code (see conversation.spec.ts for the WAV and the fakes).
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const QUESTION = path.join(ROOT, 'src-tauri', 'src', 'voice', 'testdata', 'question-fr.wav');
const SAID = 'Ajoute un test pour la page des réglages, puis lance la suite.';
const SPOKEN_LOG = path.join(os.tmpdir(), `escouade-e2e-codex-spoken-${process.pid}.txt`);
const VOICES = JSON.parse(fs.readFileSync(path.join(ROOT, 'src-tauri', 'src', 'voice', 'speech', 'voices.json'), 'utf8')) as {
  id: string;
  onnxSize: number;
  jsonSize: number;
}[];

/** A file of `size` bytes that holds nothing (the fake transcriber and voice read nothing of it). */
function sparse(file: string, size: number) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const fd = fs.openSync(file, 'w');
  fs.ftruncateSync(fd, size);
  fs.closeSync(fd);
}

test.describe('a voice conversation with an agent on Codex', () => {
  test.use({
    appEnv: { ESCOUADE_CODEX: FAKE_CODEX, ESCOUADE_VOICE_WAV: QUESTION, ESCOUADE_VOICE_TRANSCRIPT: SAID, ESCOUADE_SPEECH_FAKE: SPOKEN_LOG },
    preStart: {
      run: (root: string) => {
        sparse(path.join(root, 'data', 'voice', 'models', 'ggml-base.bin'), 147_951_465);
        const v = VOICES.find((x) => x.id === 'fr_FR-siwis-medium')!;
        sparse(path.join(root, 'data', 'voice', 'voices', `${v.id}.onnx`), v.onnxSize);
        sparse(path.join(root, 'data', 'voice', 'voices', `${v.id}.onnx.json`), v.jsonSize);
        fs.rmSync(SPOKEN_LOG, { force: true });
      },
    },
  });

  test('has the passage of its answer shown apart and read aloud', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newCodexAgent(page);
    await page.getByRole('button', { name: 'Conversation', exact: true }).click();
    // Sent to Codex with the instruction to end with a passage to be read.
    await expect
      .poll(() => requests(app, 'turn/start').find((m) => m?.params.input[0].text.includes('réglages'))?.params.input[0].text ?? '', {
        timeout: 20_000,
      })
      .toContain('<escouade-voice>');
    await expect(page.locator('[data-spoken]')).toContainText("C'est noté, je m'en occupe avec Codex.");
    await expect(page.getByText('détail technique', { exact: false })).toBeVisible();
    await expect(page.getByText('<spoken>')).toHaveCount(0);
    await expect
      .poll(() => (fs.existsSync(SPOKEN_LOG) ? fs.readFileSync(SPOKEN_LOG, 'utf8').split('\n').filter(Boolean) : []), {
        timeout: 15_000,
      })
      .toEqual(["C'est noté, je m'en occupe avec Codex."]);
    await page.keyboard.press('Escape');
  });

  test('has the cost of its turns estimated, and an account of its own added and signed in by codex login (ESC-38)', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newCodexAgent(page);
    await send(page, 'bonjour Codex');
    // Codex tells no cost: Escouade's estimate, said so.
    const done = page.getByTestId('turn-done');
    await expect(done).toContainText(/≈\s.*\$ estimé/);
    await expect(done.getByText(/estimé/)).toHaveAttribute('data-tip', /Coût estimé/);

    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    // Codex's group has its own button (2.0 design): the window opens on Codex.
    await settings.getByRole('button', { name: 'Ajouter un compte Codex…' }).click();
    const codex = page.getByRole('dialog', { name: 'Ajouter un compte Codex' });
    await expect(codex.getByText(/La connexion \(auth\.json\) n’est jamais copiée/)).toBeVisible();
    await codex.getByRole('textbox', { name: 'Nom' }).fill('Travail');
    await codex.getByRole('button', { name: 'Créer et se connecter' }).click();
    // `codex login` in a terminal, in the account's own folder: signed in once its auth.json is there.
    const signIn = page.getByRole('dialog', { name: 'Se connecter au compte « Travail »' });
    await expect(signIn.getByRole('status')).toHaveText('Connecté au compte codex@example.com', { timeout: 20_000 });
    const folder = path.join(app.root, 'data', 'codex', 'travail');
    expect(fs.existsSync(path.join(folder, 'auth.json'))).toBe(true);
    const login = codexLog(app).find((e: any) => e.login) as any;
    expect(login.login).toEqual({ argv: ['login'], codexHome: folder });
    await signIn.getByRole('button', { name: 'Terminé' }).click();
    // Listed with Codex's Principal, which it brought.
    const list = page.getByRole('dialog', { name: 'Réglages' }).getByRole('list', { name: 'Comptes Codex' });
    await expect(list.getByRole('listitem')).toHaveCount(2);
    await expect(list.getByRole('listitem', { name: 'Travail' })).toContainText('Connecté · codex@example.com');
  });
});

// A harness installed while the app runs (ESC-186): no Codex on the PATH at the start, then a `codex` put in a folder the
// PATH of the user gains (a file of the test's stands for the registry: `ESCOUADE_TEST_PATH_FILE`; never the real one).
const installed = fs.mkdtempSync(path.join(os.tmpdir(), 'esc186-bin-'));
const registryPath = `${installed}.path`;
fs.writeFileSync(registryPath, '');
const FAKE_CODEX_MJS = path.join(path.dirname(FAKE_CODEX), 'fake-codex.mjs');
// The PATH of the tests without the folders a `codex` of the machine is in.
const withoutCodex = (process.env.PATH ?? '')
  .split(path.delimiter)
  .filter((d) => !['codex', 'codex.cmd', 'codex.exe'].some((n) => d && fs.existsSync(path.join(d, n))))
  .join(path.delimiter);

test.describe('a harness installed while the app runs (ESC-186)', () => {
  test.use({ appEnv: { ESCOUADE_CODEX: 'path', ESCOUADE_TEST_PATH_FILE: registryPath, PATH: withoutCodex } });

  test('is offered by the menu « Harness » without a restart, and its agent runs, in the runner’s window', async ({ app }) => {
    test.skip(process.platform !== 'win32', 'the PATH of the registry is Windows’');
    const { page } = app;
    await page.setViewportSize({ width: 1024, height: 728 });
    await addProject(page, app.repo);
    const more = page.getByRole('button', { name: 'Autres façons de créer un agent' });
    await more.click();
    // Not found: not in the menu, only the harnesses set up are.
    await expect(page.getByRole('menuitemradio', { name: /^Claude Code/ })).toBeVisible();
    await expect(page.getByRole('menuitemradio', { name: /^Codex/ })).toHaveCount(0);
    await page.keyboard.press('Escape');
    // `npm install -g @openai/codex` meanwhile: its program, and its folder in the PATH of the user.
    fs.writeFileSync(path.join(installed, 'codex.cmd'), `@echo off\r\nnode "${FAKE_CODEX_MJS}" %*\r\n`);
    fs.writeFileSync(registryPath, `${installed}\n`);
    await more.click();
    const codex = page.getByRole('menuitemradio', { name: /^Codex/ });
    await expect(codex).toBeEnabled();
    await expect(codex).toBeInViewport();
    await codex.click();
    await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à Codex…' })).toBeVisible();
    // Its agent's process finds it by the same PATH.
    await send(page, 'bonjour Codex');
    await expect(page.getByText('Bonjour, tu as dit : bonjour Codex')).toBeVisible();
  });
});
