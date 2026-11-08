// Antigravity CLI (ESC-270): agents on the fake `agy` (tests/fixtures/fake-agy.mjs), which speaks stream-json as `agy`
// 1.4.9 does in `-p`, and whose message picks what a turn does. What the app sent it is in its log (FAKE_AGY_LOG).
// ESCOUADE_E2E_SHOTS=<folder>: its screenshots, in both themes.

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_AGY, send, test, type App, openSettingsPage } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

type Entry = {
  launch?: { argv: string[]; cwd: string; home: string; geminiKey: string | null; allow?: string[] | null };
  user?: string;
};

function agyLog(app: App): Entry[] {
  const file = path.join(app.root, 'fake-agy.jsonl');
  if (!fs.existsSync(file)) return [];
  return fs
    .readFileSync(file, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));
}

/** The launches of an agent's program: not `agy models` nor a text written, each in a home of their own. */
function agentLaunches(app: App) {
  return agyLog(app)
    .filter((e) => e.launch && e.launch.argv[0] === '-p=' && !path.basename(e.launch.home).startsWith('escouade-agy-'))
    .map((e) => e.launch!);
}

/** The messages the app sent the fake, in order. */
const said = (app: App) => agyLog(app).flatMap((e) => (e.user === undefined ? [] : [e.user]));

/** The theme of the app, set as « Thème » would. */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

/** A new agent on Antigravity CLI, from the menu beside « Nouvel agent ». */
async function newAgyAgent(page: Page) {
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitemradio', { name: 'Antigravity CLI' }).click();
  await expect(page.locator('main.conv header.head')).toContainText('Antigravity');
  await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à Antigravity…' })).toBeVisible();
}

test.describe('an agent on Antigravity CLI', () => {
  test.use({ appEnv: { ESCOUADE_AGY: FAKE_AGY } });

  test('converses, asks its permissions once its turn is over, from the window and the list, with a write’s diff, and resumes', async ({
    app,
  }) => {
    const { page } = app;
    await addProject(page, app.repo);
    // Réglages › Comptes et harness: found, with its version.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    const agy = settings.getByRole('region', { name: 'Antigravity CLI', exact: true });
    await expect(agy.locator('[data-state]')).toHaveText('1.4.9');
    await agy.scrollIntoViewIfNeeded();
    await shots(page, 'harness');
    await page.keyboard.press('Escape');

    await newAgyAgent(page);
    await send(page, 'bonjour Antigravity');
    await expect(page.getByText('Antigravity a reçu : bonjour Antigravity')).toBeVisible();
    // Started as Escouade drives it: `-p=` in stream-json, in a home of the agent's own (never the user's ~/.gemini),
    // without the machine's Gemini key; Escouade's server in its own MCP config, never on its command line.
    const launch = agentLaunches(app)[0];
    expect(launch.argv.slice(0, 5)).toEqual(['-p=', '--input-format', 'stream-json', '--output-format', 'stream-json']);
    // The machine's key given empty, never its value.
    expect(launch.geminiKey || null).toBeNull();
    const homes = path.join(app.data, 'agent-homes');
    expect(path.dirname(launch.home).toLowerCase()).toBe(homes.toLowerCase());
    expect(path.resolve(launch.cwd).toLowerCase()).toBe(path.resolve(app.repo).toLowerCase());
    expect(launch.argv.join(' ')).not.toContain('Bearer');
    // Its MCP config is its own folder's (the server's entry there when the agents may drive Escouade).
    expect(fs.existsSync(path.join(launch.home, '.gemini', 'config', 'mcp_config.json'))).toBe(true);

    // A command: refused by `agy`, asked once the turn is over; « Pour cet agent » is kept by the plugin, and the turn goes
    // on with the rule written for it alone (ESC-371).
    await send(page, 'commande');
    const card = page.getByTestId('permission-pending');
    await expect(card).toContainText('npm test');
    await shots(page, 'permission');
    await card.getByRole('button', { name: 'Pour cet agent' }).click();
    await expect(page.getByText('Fait.')).toBeVisible();
    expect(said(app).at(-1)).toContain('The user allowed the command `npm test`');
    // The rule is in the settings `agy` read when launched again for the turn, then taken back once it is over (ESC-371):
    // « Pour cet agent » is kept by the plugin, which allows the very same call again.
    expect(agentLaunches(app).some((l) => (l.allow ?? []).includes('command(npm test)'))).toBe(true);
    const settingsFile = path.join(launch.home, '.gemini', 'antigravity-cli', 'settings.json');
    await expect.poll(() => fs.readFileSync(settingsFile, 'utf8')).not.toContain('command(npm test)');
    // Asked again: `agy` refuses it (its rule is gone), the plugin allows it at once for « Pour cet agent », without a card.
    await send(page, 'commande encore');
    await expect(page.getByText('Fait.')).toHaveCount(2);
    await expect(card).toHaveCount(0);
    expect(said(app).at(-1)).toContain('The user allowed the command `npm test`');

    // A write in its folder: refused by `agy` (no rule lets a write through), accepted by Escouade's rules without a card,
    // its diff on its row; one beside it asks, with its diff, answered from the list (Ctrl+J then 1).
    await send(page, 'fichier');
    await expect(page.getByText('Fait.')).toHaveCount(3);
    await expect(card).toHaveCount(0);
    expect(fs.existsSync(path.join(app.repo, 'notes.txt'))).toBe(true);
    await expect(page.locator('main.conv').getByText('notes.txt').first()).toBeVisible();
    await send(page, 'fichier dehors');
    await expect(card).toContainText('Hors du dossier de l’agent');
    await expect(card.locator('.patch, .patch-view, pre').first()).toContainText('bonjour');
    await shots(page, 'write-outside');
    await page.waitForTimeout(600);
    await page.locator('body').evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    await page.keyboard.press('Control+j');
    await page.keyboard.press('1');
    await expect(page.getByText('Fait.')).toHaveCount(4);
    expect(fs.existsSync(path.join(path.dirname(app.repo), 'dehors.txt'))).toBe(true);

    // A question `agy` skipped: its card, the answer sent as the turn goes on.
    await send(page, 'question');
    const question = page.getByTestId('question-pending');
    await expect(question).toContainText('Bleu ou rouge ?');
    await shots(page, 'question');
    await question.getByRole('button', { name: 'Rouge' }).click();
    await expect(page.getByText('Réponse reçue : Rouge')).toBeVisible();
    await shots(page, 'turn');

    // Restarted, the app resumes its conversation (`--conversation`).
    const page2 = await app.restart();
    await page2.locator('.card', { hasText: 'Antigravity' }).first().click();
    await send(page2, 'tu te souviens ?');
    // It remembers its last message: the answers Escouade sent it.
    await expect(page2.getByText(/^Tu as dit : The user answered your questions/)).toBeVisible();
    const last = agentLaunches(app).at(-1)!;
    expect(last.argv.join(' ')).toMatch(/--conversation [0-9a-f-]{36}/);
  });

  test('its models and efforts are its account’s, its mode Plan asks a plan to approve, and a turn stops', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newAgyAgent(page);
    await send(page, 'bonjour');
    await expect(page.getByText('Antigravity a reçu : bonjour')).toBeVisible();
    // The models of `agy models`, Gemini's efforts apart; a model changed starts its program again on the conversation.
    await page.getByRole('button', { name: /^Modèle/ }).click();
    const menu = page.getByRole('menu', { name: 'Modèle' });
    await expect(menu.getByRole('menuitemradio', { name: /Gemini 3\.8 Flash/ })).toBeVisible();
    await menu.getByRole('menuitemradio', { name: /Claude Sonnet 4\.6/ }).click();
    await send(page, 'modèle');
    await expect(page.getByText(/^Modèle : claude-sonnet-4-6, effort : aucun, mode : default$/)).toBeVisible();

    // « Plan »: its mode `plan`, its plan written in its conversation's files, approved on its card, then carried out.
    await page.getByRole('button', { name: /^Mode/ }).click();
    await page.getByRole('menuitemradio', { name: /^Plan/ }).click();
    await send(page, 'propose un plan');
    const plan = page.getByRole('group', { name: 'Antigravity propose un plan' });
    await expect(plan).toContainText('Écrire notes.txt');
    await shots(page, 'plan');
    await plan.getByRole('button', { name: 'Approuver le plan' }).click();
    await expect(page.getByText('Plan mis en œuvre.')).toBeVisible();
    expect(agentLaunches(app).some((l) => l.argv.includes('plan'))).toBe(true);
    expect(agentLaunches(app).at(-1)!.argv).not.toContain('plan');

    // A turn stopped: its program killed, the next message resumes the conversation.
    await send(page, 'lent');
    await expect(page.getByText('Je commence un long travail…')).toBeVisible();
    await page.getByRole('button', { name: 'Stop' }).click();
    await expect(page.getByRole('button', { name: 'Stop' })).toHaveCount(0);
    await page.waitForTimeout(6500);
    await expect(page.getByText('Fini.')).toHaveCount(0);
    await send(page, 'encore');
    await expect(page.getByText('Antigravity a reçu : encore')).toBeVisible();
  });
});

// ESC-271: the machine's Antigravity account is added by hand (none is made by itself, ESC-128), even before it is signed
// in: its sign-in is made by `agy` itself in a terminal of the window (the fake writes the file that stands for the
// keychain's entry). Another account is a Gemini API key, kept in the system keychain (the sandbox's entries) and given
// to its agents by GEMINI_API_KEY with `modelProvider: gemini` in their own settings. Its usage is on its plans' page
// and its interface's /usage; its quota met ends the turn « limité », its resume planned.
test.describe('Antigravity CLI’s accounts', () => {
  test.use({
    appEnv: { ESCOUADE_AGY: FAKE_AGY },
    // Claude Code's machine account alone: Antigravity's is the test's to add.
    preStart: {
      run: () => ({
        agySignedOut: true,
        noAccounts: true,
        accounts: [{ id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true }],
      }),
    },
  });

  test('adds the machine’s account and signs it in by agy in a terminal, adds one by its Gemini key, links its usage', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    // None of its own (ESC-128): Antigravity CLI installed, « Aucun compte ».
    await expect(settings.getByRole('region', { name: 'Antigravity CLI', exact: true })).toContainText('Aucun compte');
    await settings.getByRole('button', { name: 'Ajouter un compte Antigravity CLI…' }).click();
    const modal = page.getByRole('dialog', { name: 'Ajouter un compte Antigravity CLI' });
    await expect(modal.getByText('Antigravity CLI n’est pas encore connecté sur cette machine', { exact: false })).toBeVisible();
    await expect(modal.getByRole('radio', { name: /Le compte de la machine/ })).toBeChecked();
    await shots(page, 'account-machine');
    await modal.getByRole('button', { name: 'Ajouter ce compte' }).click();
    // Its sign-in, by agy in the terminal here; seen, then the terminal goes.
    const signIn = page.getByRole('dialog', { name: 'Se connecter au compte « Principal »' });
    await expect(signIn.getByRole('status')).toContainText('agy ouvre ton navigateur sur ton compte Google');
    await expect(signIn.getByRole('status')).toContainText('Connecté', { timeout: 15_000 });
    await shots(page, 'account-signed-in');
    await signIn.getByRole('button', { name: 'Terminé' }).click();
    const list = settings.getByRole('list', { name: 'Comptes Antigravity CLI' });
    const machine = list.getByRole('listitem', { name: 'Principal' });
    await expect(machine).toContainText('Connecté');
    await expect(machine.getByRole('button', { name: 'Usage sur antigravity.google' })).toBeVisible();
    // Another, by its key.
    await settings.getByRole('button', { name: 'Ajouter un compte Antigravity CLI…' }).click();
    const add = page.getByRole('dialog', { name: 'Ajouter un compte Antigravity CLI' });
    await expect(add.getByRole('radio', { name: /Un nouveau compte/ })).toBeChecked();
    await add.getByRole('textbox', { name: 'Nom' }).fill('Équipe');
    await add.getByLabel('Clé d’API').fill('AIzaCleGeminiE2e0000wxyz');
    await expect(add.getByLabel('Clé d’API')).toHaveAttribute('placeholder', 'Sur aistudio.google.com, Get API key');
    await shots(page, 'account-key');
    await add.getByRole('button', { name: 'Ajouter le compte' }).click();
    const team = list.getByRole('listitem', { name: 'Équipe' });
    await expect(team).toContainText('Connecté · clé …wxyz');
    await expect(team.getByRole('button', { name: 'Changer la clé…' })).toBeVisible();
    // Neither cost nor budget: its usage is on its page and its /usage.
    const usage = team.getByRole('button', { name: 'Usage sur antigravity.google' });
    await expect(usage).toBeVisible();
    await expect(team.getByRole('textbox', { name: 'Budget par jour ($)' })).toHaveCount(0);
    // Hovered again if the row is drawn anew meanwhile (its status, its models read).
    await expect(async () => {
      await page.mouse.move(0, 0);
      await usage.hover();
      await expect(page.getByRole('tooltip')).toContainText('se lit par /usage dans son interface', { timeout: 1500 });
    }).toPass({ timeout: 15_000 });
    await shots(page, 'accounts');
    expect(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8')).not.toContain('AIzaCle');
    // The machine's account off: new Antigravity agents go to the key's, which runs as it.
    await machine.getByRole('switch', { name: 'Actif' }).click();
    await expect(machine.getByRole('switch', { name: 'Actif' })).toHaveAttribute('aria-checked', 'false');
    await page.keyboard.press('Escape');
    await newAgyAgent(page);
    await send(page, 'compte');
    await expect(page.getByText('Compte : clé …wxyz')).toBeVisible();
    const launch = agentLaunches(app).at(-1)!;
    expect(launch.geminiKey).toBe('AIzaCleGeminiE2e0000wxyz');
    expect(launch.argv.join(' ')).not.toContain('AIzaCle');
    // Its quota met: « limité », its resume planned.
    await send(page, 'limite');
    const card = page.locator('.card.error.limit');
    await expect(card).toContainText('Limite d’usage d’Antigravity atteinte');
    await expect(card).toContainText('You have exhausted your quota on this model.');
    await expect(card).toContainText('Reprise automatique');
    await shots(page, 'limit');
  });
});

// ESC-272: the agent's conversation in Antigravity's own interface (fake-agy-tui.mjs), in its home, with Escouade's server
// in its MCP config (never on its command line), its hooks telling Escouade; back in Escouade's view, nothing lost.
test.describe('« Vue Antigravity » (ESC-272)', () => {
  test.use({ appEnv: { ESCOUADE_AGY: FAKE_AGY }, preStart: { run: () => ({ mcpEnabled: true }) } });

  /** What the fake interface wrote in the agents' homes: its launches, the hooks it ran. */
  const ofHomes = (app: App, name: string): any[] => {
    const homes = path.join(app.data, 'agent-homes');
    if (!fs.existsSync(homes)) return [];
    return fs
      .readdirSync(homes)
      .map((id) => path.join(homes, id, name))
      .filter((f) => fs.existsSync(f))
      .flatMap((f) => fs.readFileSync(f, 'utf8').split('\n').filter(Boolean))
      .map((l) => JSON.parse(l));
  };

  test('opens Antigravity’s interface on the agent’s conversation with Escouade’s server, follows its turns, and hands it back', async ({
    app,
  }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newAgyAgent(page);
    await send(page, 'bonjour Antigravity');
    await expect(page.getByText('Antigravity a reçu : bonjour Antigravity')).toBeVisible();
    expect(agentLaunches(app)).toHaveLength(1);

    await page.getByRole('button', { name: 'Vue Antigravity CLI' }).click();
    const view = page.locator('main .tui');
    await expect(view.locator('.xterm')).toBeVisible({ timeout: 20_000 });
    await expect.poll(() => ofHomes(app, 'fake-tui-launches.jsonl').length, { timeout: 20_000 }).toBe(1);
    // Its interface on the conversation (one store for both), in the agent's home, with Escouade's server in its MCP
    // config: its token in that file, never on its command line nor in its variables.
    const launch = ofHomes(app, 'fake-tui-launches.jsonl')[0];
    expect(launch.argv).toContain('--conversation');
    const conversation = launch.argv[launch.argv.indexOf('--conversation') + 1];
    const header: string = launch.mcp.mcpServers.escouade.headers.Authorization;
    const token = header.replace(/^Bearer /, '');
    expect(token).toBeTruthy();
    expect(launch.argv.join(' ')).not.toContain(token);
    // Its first screens passed, the account's address kept off its screen, its hooks for Escouade.
    expect(launch.onboarding.onboardingComplete).toBe(true);
    expect(launch.settings.trustedWorkspaces.map((w: string) => path.resolve(w))).toContain(path.resolve(app.repo));
    expect(launch.hideAccount).toBe('1');
    expect(Object.keys(launch.hooks.escouade).sort()).toEqual(['PostToolUse', 'PreToolUse', 'SessionStart', 'Stop']);
    await expect(view.getByRole('status')).toHaveCount(0);

    // Messages typed there: the card says it works, then that it is done. The terminal is drawn on a canvas: what the
    // fake does is read from its hooks and the conversation.
    await view.locator('.xterm').click();
    const ended = () => ofHomes(app, 'fake-tui-hooks.jsonl').filter((e) => e.event === 'Stop').length;
    await page.keyboard.type('Bonjour la vue');
    await page.keyboard.press('Enter');
    await expect.poll(ended, { timeout: 20_000 }).toBe(1);
    await page.keyboard.type('commande');
    await page.keyboard.press('Enter');
    await expect.poll(ended, { timeout: 20_000 }).toBe(2);
    expect(ofHomes(app, 'fake-tui-hooks.jsonl').map((e) => e.event)).toEqual(['Stop', 'PreToolUse', 'PostToolUse', 'Stop']);
    await expect(page.locator('.card', { hasText: 'Antigravity' })).toContainText('Terminé');
    await page.waitForTimeout(500);
    await shots(page, 'view');

    // Back in Escouade's view: the interface's turns in the conversation, its command among them; the same
    // conversation resumed by `agy -p`, the hooks of its view gone.
    await page.getByRole('button', { name: 'Vue Escouade' }).click();
    await expect(page.getByText('Fini : Bonjour la vue')).toBeVisible();
    await expect(page.getByText('Fini : commande')).toBeVisible();
    await expect(page.locator('main.conv .bubble', { hasText: /^\s*Bonjour la vue\s*$/ })).toHaveCount(1);
    await expect(page.locator('main.conv').getByText('npm test').first()).toBeVisible();
    await send(page, 'encore');
    await expect(page.getByText('Antigravity a reçu : encore')).toBeVisible();
    const last = agentLaunches(app).at(-1)!;
    expect(last.argv[last.argv.indexOf('--conversation') + 1]).toBe(conversation);
    expect(fs.existsSync(path.join(launch.home, '.gemini', 'config', 'hooks.json'))).toBe(false);
    await shots(page, 'view-back');
  });
});
