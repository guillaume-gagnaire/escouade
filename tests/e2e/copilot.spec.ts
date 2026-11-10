// GitHub Copilot CLI (ESC-39): agents on the fake `copilot --server --stdio` (tests/fixtures/fake-copilot.mjs), which
// speaks the protocol of the Copilot SDK as Copilot CLI 1.0.98 does, and whose message picks what a turn does. What
// the app sent it is in its log (FAKE_COPILOT_LOG).

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_COPILOT, send, test, type App } from './fixture';

type Entry = { launch?: { argv: string[] }; dir?: string; msg?: any };

/** What the fake copilot was told, launch by launch. */
function copilotLog(app: App): Entry[] {
  const file = path.join(app.root, 'fake-copilot.jsonl');
  if (!fs.existsSync(file)) return [];
  return fs
    .readFileSync(file, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));
}

/** The parameters of each request `method` the app sent, in order. */
const sent = (app: App, method: string) =>
  copilotLog(app)
    .filter((e) => e.dir === 'c2s' && e.msg?.method === method)
    .map((e) => e.msg.params);

/** The answers the app gave to the requests of the fake (`userInput.request`, `exitPlanMode.request`). */
const replies = (app: App) =>
  copilotLog(app)
    .filter((e) => e.dir === 'c2s' && e.msg && e.msg.method === undefined)
    .map((e) => e.msg.result);

/** A new agent on Copilot CLI, from the menu beside « Nouvel agent ». */
async function newCopilotAgent(page: Page) {
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitemradio', { name: 'Copilot CLI' }).click();
  await expect(page.locator('main.conv header.head')).toContainText('Copilot');
  await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à Copilot…' })).toBeVisible();
}

test.describe('an agent on Copilot CLI', () => {
  test.use({ appEnv: { ESCOUADE_COPILOT: FAKE_COPILOT } });

  test('converses, asks its permissions by kind and its questions, forms included, and plans', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newCopilotAgent(page);
    await send(page, 'bonjour Copilot');
    await expect(page.getByText('Copilot a reçu : bonjour Copilot')).toBeVisible();
    // Started as Escouade drives Copilot: its server without GitHub's MCP server nor the temporary folder, the tokens
    // kept from its commands; a session in the agent's folder, on a model of its own (never `auto`), in `manual`.
    const launch = copilotLog(app).find((e) => e.launch)!.launch!;
    expect(launch.argv.slice(0, 2)).toEqual(['--server', '--stdio']);
    for (const flag of ['--no-auto-update', '--disable-builtin-mcps', '--disallow-temp-dir']) expect(launch.argv).toContain(flag);
    expect(launch.argv.some((a) => a.startsWith('--secret-env-vars=') && a.includes('GH_TOKEN'))).toBe(true);
    const created = sent(app, 'session.create')[0];
    expect(path.resolve(created.workingDirectory).toLowerCase()).toBe(path.resolve(app.repo).toLowerCase());
    expect(created.model).toBe('gpt-6-luna');
    expect(created.requestPermission).toBe(true);
    expect(sent(app, 'session.permissions.setMode')[0].mode).toBe('manual');

    // A command: its card says the paths it touches; « Pour cette session », never « Toujours ».
    await send(page, 'commande');
    const card = page.getByTestId('permission-pending');
    await expect(card).toContainText('npm test');
    await expect(card).toContainText('Chemins : package.json');
    await expect(card.getByRole('button', { name: 'Pour cette session' })).toBeVisible();
    await expect(card.getByRole('button', { name: 'Refuser' })).toBeVisible();
    await expect(card.getByRole('button', { name: /Toujours/ })).toHaveCount(0);
    await card.getByRole('button', { name: 'Autoriser', exact: true }).click();
    await expect(page.getByText('La commande a marché.')).toBeVisible();
    expect(sent(app, 'session.permissions.handlePendingPermissionRequest').at(-1).result).toEqual({ kind: 'approve-once' });

    // A write in its folder goes by Escouade's rules, without a card; one beside it asks, with its diff.
    await send(page, 'fichier');
    await expect(page.getByText('J’ai modifié notes.txt.')).toBeVisible();
    await send(page, 'fichier dehors');
    await expect(card).toContainText('Hors du dossier de l’agent');
    await expect(card.locator('.patch, .patch-view, pre').first()).toContainText('note de Copilot');
    await card.getByRole('button', { name: 'Pour cette session' }).click();
    await expect(page.getByText('J’ai modifié copilot-dehors.txt.')).toBeVisible();
    expect(sent(app, 'session.permissions.handlePendingPermissionRequest').at(-1).result).toEqual({
      kind: 'approve-for-session',
      approval: { kind: 'write' },
    });

    // An address, refused.
    await send(page, 'web');
    await expect(card).toContainText('https://example.com');
    await card.getByRole('button', { name: 'Refuser' }).click();
    await expect(page.getByText('Accès au web refusé.')).toBeVisible();

    // A tool of an MCP server: its server, and that it only reads.
    await send(page, 'mcp');
    await expect(card).toContainText('Serveur MCP essai, outil ping : ne fait que lire');
    await card.getByRole('button', { name: 'Autoriser', exact: true }).click();
    await expect(page.getByText('Le ping a répondu.')).toBeVisible();

    // A question, asked twice by Copilot (its event and its request): one card, both answered.
    await send(page, 'question');
    const question = page.getByTestId('question-pending');
    await expect(question).toContainText('Quelle couleur préfères-tu ?');
    await question.getByRole('button', { name: 'bleu' }).click();
    await expect(page.getByText('Tu as choisi bleu.')).toBeVisible();
    expect(sent(app, 'session.ui.handlePendingUserInput').at(-1).response).toEqual({ answer: 'bleu', wasFreeform: false });
    expect(replies(app)).toContainEqual({ answer: 'bleu', wasFreeform: false });

    // A form of an MCP server: its fields.
    await send(page, 'formulaire');
    await expect(question.getByTestId('question-form')).toContainText('Formulaire de essai');
    await question.getByRole('textbox', { name: 'Prénom' }).fill('Camille');
    await question.getByRole('textbox', { name: 'Âge' }).fill('30');
    await question.getByRole('button', { name: 'Oui' }).click();
    await question.getByRole('button', { name: 'Valider' }).click();
    await expect(page.getByText('Formulaire reçu : Camille, 30 ans, abonné : true.')).toBeVisible();

    // The end of a plan: the card of a plan to approve.
    await send(page, 'propose un plan');
    const plan = page.getByRole('group', { name: 'Copilot propose un plan' });
    await expect(plan).toContainText('Ajouter sub');
    await plan.getByRole('button', { name: 'Approuver le plan' }).click();
    await expect(page.getByText('Plan approuvé, je commence.')).toBeVisible();
    expect(sent(app, 'session.ui.handlePendingExitPlanMode').at(-1).response).toMatchObject({ approved: true });

    // Its tasks and its subagent, in the « Plan » banner; the subagent's tool under its Task.
    await send(page, 'sous-agent');
    await expect(page.getByText('Tâches faites, le sous-agent a résumé le README.')).toBeVisible();
    const banner = page.locator('main.conv .plan');
    await expect(banner).toContainText('2/2');
    const fold = banner.locator('button.fold');
    if ((await fold.getAttribute('aria-expanded')) === 'false') await fold.click();
    await expect(banner).toContainText('Résumer');
    await expect(page.locator('.tool', { hasText: 'Résumé du README' }).first()).toBeVisible();

    // Restarted, the app resumes its session.
    const page2 = await app.restart();
    await page2.locator('.card', { hasText: 'Copilot' }).first().click();
    await send(page2, 'encore là ?');
    await expect(page2.getByText('Copilot a reçu : encore là ?')).toBeVisible();
    expect(sent(app, 'session.create').length).toBe(1);
    expect(sent(app, 'session.resume').length).toBe(1);
  });

  test('its models say their multiplier, its modes what they mean, and a message during a turn goes in at once', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newCopilotAgent(page);
    await send(page, 'bonjour');
    await expect(page.getByText('Copilot a reçu : bonjour')).toBeVisible();
    // The models of the account (`models.list`), each with its multiplier.
    await page.getByRole('button', { name: /^Modèle/ }).click();
    const menu = page.getByRole('menu', { name: 'Modèle' });
    await expect(menu.getByRole('menuitemradio', { name: /GPT-6 Luna/ })).toContainText('×1');
    await expect(menu.getByRole('menuitemradio', { name: /Claude Sonnet 4\.5/ })).toContainText('×3');
    await expect(menu.getByRole('menuitemradio', { name: /GPT-5 mini/ })).toContainText('×0,33');
    // Changed at once, even between turns.
    await menu.getByRole('menuitemradio', { name: /Claude Sonnet 4\.5/ }).click();
    await expect.poll(() => sent(app, 'session.model.switchTo').map((p) => p.modelId)).toContain('claude-sonnet-4.5');
    await send(page, 'modèle');
    await expect(page.getByText(/^Je tourne sur claude-sonnet-4.5/)).toBeVisible();

    // The modes say that Copilot has no sandbox in its server mode; « Bypass » is its `allow-all`, « Plan » its `plan`.
    await page.getByRole('button', { name: /^Mode/ }).click();
    await expect(page.getByText(/pas de bac à sable/).first()).toBeVisible();
    await page.getByRole('menuitemradio', { name: /^Plan/ }).click();
    await expect.poll(() => sent(app, 'session.mode.set').map((p) => p.mode)).toContain('plan');
    await page.getByRole('button', { name: /^Mode/ }).click();
    await page.getByRole('menuitemradio', { name: /^Demander/ }).click();
    await expect.poll(() => sent(app, 'session.mode.set').at(-1)?.mode).toBe('interactive');

    // A message sent during a turn goes into it at once (`immediate`), not after it.
    await send(page, 'lent');
    await expect(page.getByText('Je commence un long travail…')).toBeVisible();
    await send(page, 'et maintenant');
    await expect(page.getByText('Reçu pendant le tour : et maintenant')).toBeVisible();
    const modes = sent(app, 'session.send').map((p) => p.mode);
    expect(modes.at(-1)).toBe('immediate');
    expect(modes.at(-2)).toBe('enqueue');
  });
});

test.describe('the credits of a Copilot CLI account', () => {
  test.use({ appEnv: { ESCOUADE_COPILOT: FAKE_COPILOT } });

  test('show in the status bar as each call tells them, and a turn stopped by their end resumes when they come back', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newCopilotAgent(page);
    await send(page, 'bonjour');
    await expect(page.getByText('Copilot a reçu : bonjour')).toBeVisible();
    // The fake's calls say 90 % of the month's credits left: a bar « crédits », 10 % used.
    // Its harness's group, first (the agent shown is Copilot CLI's), then Claude Code's, whose first agent is at work too.
    const group = page.locator('footer .grp').first();
    await expect(group).toHaveAttribute('data-engine', 'copilot');
    const credits = group.getByRole('meter', { name: 'Crédits du mois' });
    await expect(credits).toHaveAttribute('aria-valuenow', '10');
    await expect(group.getByRole('meter', { name: 'Quota sur 5 heures' })).toHaveCount(0);
    // Their end stops the turn, which resumes by itself once they are given again: the first of next month.
    await send(page, 'limite');
    await expect(credits).toHaveAttribute('aria-valuenow', '100');
    const resume = page.getByText(/^Reprise automatique (le .+ )?à \d\d:\d\d$/);
    await expect(resume).toBeVisible();
    await page.getByRole('button', { name: 'Annuler la reprise' }).click();
    await expect(resume).toBeHidden();
  });
});

test.describe('« Vue Copilot CLI » (ESC-48)', () => {
  test.use({ appEnv: { ESCOUADE_COPILOT: FAKE_COPILOT } });

  /** A file of the fake interface in its COPILOT_HOME (one JSON a line). */
  const tuiLog = (app: App, name: string): any[] => {
    const file = path.join(app.root, 'copilot', name);
    return fs.existsSync(file)
      ? fs
          .readFileSync(file, 'utf8')
          .split('\n')
          .filter(Boolean)
          .map((l) => JSON.parse(l))
      : [];
  };

  test('opens Copilot’s interface on the agent’s session, follows its turn on the card, and comes back with the conversation', async ({
    app,
  }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newCopilotAgent(page);
    await send(page, 'bonjour Copilot');
    await expect(page.getByText('Copilot a reçu : bonjour Copilot')).toBeVisible();
    const session = sent(app, 'session.create').length;
    expect(session).toBe(1);

    await page.getByRole('button', { name: 'Vue Copilot CLI' }).click();
    const view = page.locator('main .tui');
    await expect(view.locator('.xterm')).toBeVisible({ timeout: 20_000 });
    await expect.poll(() => tuiLog(app, 'fake-tui-launches.jsonl').length, { timeout: 20_000 }).toBe(1);
    // The agent's session resumed, Escouade's hooks in a plugin of its own, their address and token in its environment.
    const launch = tuiLog(app, 'fake-tui-launches.jsonl')[0];
    expect(launch.argv[0]).toBe('--resume');
    expect(launch.argv).toContain('--plugin-dir');
    expect([launch.hookUrl, launch.hookToken]).toEqual([true, true]);
    // The folder trusted in the interface by the user (« Yes, and remember this folder »): written in its config. The
    // terminal is drawn on a canvas: what the fake does is read from its files.
    const config = path.join(app.root, 'copilot', 'config.json');
    await view.locator('.xterm').click();
    await expect
      .poll(
        async () => {
          await page.keyboard.press('ArrowDown');
          await page.keyboard.press('Enter');
          return fs.existsSync(config) && fs.readFileSync(config, 'utf8').includes('trustedFolders');
        },
        { timeout: 20_000, intervals: [1_000] },
      )
      .toBe(true);

    // A message typed there that asks a permission: the card says it waits, then that it is done.
    await page.keyboard.type('permission ?');
    await page.keyboard.press('Enter');
    await expect
      .poll(() => page.locator('.card').allTextContents(), { timeout: 20_000 })
      .toContainEqual(expect.stringMatching(/Question.*Copilot/s));
    await page.keyboard.press('Enter');
    await expect
      .poll(() => page.locator('.card').allTextContents(), { timeout: 20_000 })
      .toContainEqual(expect.stringMatching(/Terminé.*Copilot/s));
    expect(tuiLog(app, 'fake-tui-hooks.jsonl').map((h) => h.event)).toContain('agentStop');

    // Back in Escouade's view: the turn of the interface in the conversation, its message once; the same session resumed.
    await page.getByRole('button', { name: 'Vue Escouade' }).click();
    await expect(page.getByText('Fini : permission ?')).toBeVisible();
    await expect(page.locator('main.conv .bubble', { hasText: /^\s*permission \?\s*$/ })).toHaveCount(1);
    await send(page, 'encore');
    await expect(page.getByText('Copilot a reçu : encore')).toBeVisible();
    expect(sent(app, 'session.create')).toHaveLength(1);
    expect(sent(app, 'session.resume').at(-1).sessionId).toBe(launch.argv[1]);
  });
});

test.describe('« Continuer sur Copilot CLI » from a terminal view (ESC-120)', () => {
  test.use({
    appEnv: { ESCOUADE_COPILOT: FAKE_COPILOT },
    preStart: { run: () => ({ handoverModels: { copilot: 'gpt-6-luna' } }) },
  });

  /** The launches of the fake interface, in its COPILOT_HOME. */
  const tuiLaunches = (app: App): any[] => {
    const file = path.join(app.root, 'copilot', 'fake-tui-launches.jsonl');
    return fs.existsSync(file)
      ? fs
          .readFileSync(file, 'utf8')
          .split('\n')
          .filter(Boolean)
          .map((l) => JSON.parse(l))
      : [];
  };

  /** A new agent on Claude Code, from the menu beside « Nouvel agent ». */
  async function newClaudeAgent(page: Page) {
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await page.getByRole('menuitemradio', { name: 'Claude Code' }).click();
    await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à Claude…' })).toBeVisible();
  }

  /** « Continuer sur <harness> » from the menu of the card shown. */
  async function continueOn(page: Page, harness: string) {
    await page.locator('.card.sel').click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'Continuer sur un autre harness' }).hover();
    await page.getByRole('menuitem', { name: `Continuer sur ${harness}` }).click();
  }

  test('an agent in the Claude Code view goes on in the Copilot view on its new session, one in Escouade’s view stays there', async ({
    app,
  }) => {
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false });
    // Copilot used once in the project: its Principal serves it then.
    await newCopilotAgent(page);
    await send(page, 'bonjour Copilot');
    await expect(page.getByText('Copilot a reçu : bonjour Copilot')).toBeVisible();

    // A Claude Code agent in Escouade's view: on Copilot, in Escouade's view still once the turn of its handover file is over.
    await newClaudeAgent(page);
    await send(page, 'bonjour Claude');
    await expect(page.getByText('Bonjour, tu as dit : bonjour Claude')).toBeVisible();
    await continueOn(page, 'Copilot CLI');
    await expect(page.getByTestId('handover')).toContainText('Suite sur Copilot CLI');
    await expect(page.getByTestId('turn-done').filter({ hasText: 'sur Copilot CLI' })).toBeVisible({ timeout: 20_000 });
    await page.waitForTimeout(1_500);
    await expect(page.locator('main .xterm')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Vue Copilot CLI' })).toBeVisible();

    // Another in the Claude Code view: on Copilot, the file goes over the stream, then its interface on its new session.
    await newClaudeAgent(page);
    await send(page, 'bonjour Claude');
    await expect(page.getByText('Bonjour, tu as dit : bonjour Claude')).toBeVisible();
    await expect(page.locator('.card.sel')).toContainText('Terminé');
    await page.getByRole('button', { name: 'Vue Claude Code' }).click();
    await expect(page.locator('main [data-pastes="images"] .xterm textarea')).toBeAttached({ timeout: 20_000 });
    const created = sent(app, 'session.create').length;
    await continueOn(page, 'Copilot CLI');
    await expect.poll(() => sent(app, 'session.create').length, { timeout: 20_000 }).toBe(created + 1);
    await expect(page.locator('main .tui .xterm')).toBeVisible({ timeout: 20_000 });
    await expect(page.getByRole('button', { name: 'Vue Escouade' })).toBeVisible();
    await expect.poll(() => tuiLaunches(app).length, { timeout: 20_000 }).toBe(1);
    const argv: string[] = tuiLaunches(app)[0].argv;
    expect(argv[0]).toBe('--resume');
    expect(fs.existsSync(path.join(app.root, 'copilot', 'session-state', `${argv[1]}.json`))).toBe(true);
    // The turn of its handover file, in the conversation back in Escouade's view.
    await page.getByRole('button', { name: 'Vue Escouade' }).click();
    await expect(page.getByTestId('handover')).toContainText('Suite sur Copilot CLI');
    await expect(page.getByTestId('turn-done').filter({ hasText: 'sur Copilot CLI' })).toBeVisible();
  });
});
