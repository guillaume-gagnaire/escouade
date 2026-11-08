// Gemini CLI (ESC-274): agents on the fake `gemini --acp` (tests/fixtures/fake-gemini.mjs), which speaks ACP as Gemini CLI
// 0.66.0 does in the study (tests/fixtures/gemini/), and whose message picks what a turn does. What the app sent it is in
// its log (FAKE_GEMINI_LOG). ESCOUADE_E2E_SHOTS=<folder>: its screenshots, in both themes.

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_GEMINI, send, test, type App, openSettingsPage } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

type Entry = {
  launch?: {
    argv: string[];
    cwd: string;
    home: string | null;
    trust: boolean;
    policy: string | null;
    auth: string | null;
    /** Its account's sign-in (ESC-275): the file of its Google sign-in, its key's last four characters. */
    credentials?: string | null;
    keyEnd?: string | null;
    /** A text by `gemini -p` (ESC-275): its question, on its input. */
    prompt?: boolean;
    input?: string;
  };
  dir?: 'in' | 'out';
  msg?: { method?: string; params?: Record<string, unknown> };
};

function geminiLog(app: App): Entry[] {
  const file = path.join(app.root, 'fake-gemini.jsonl');
  if (!fs.existsSync(file)) return [];
  return fs
    .readFileSync(file, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));
}

/** The launches of an agent's process: not those of its models read nor of a text written, each in a home of their own. */
function agentLaunches(app: App) {
  return geminiLog(app)
    .filter((e) => e.launch && !path.basename(e.launch.home ?? '').startsWith('escouade-gemini-'))
    .map((e) => e.launch!);
}

/** The methods the app asked the fake, in order. */
const asked = (app: App) => geminiLog(app).flatMap((e) => (e.dir === 'out' && e.msg?.method ? [e.msg] : []));

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

/** A new agent on Gemini CLI, from the menu beside « Nouvel agent ». */
async function newGeminiAgent(page: Page) {
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await expect(page.getByRole('menuitemradio', { name: 'Gemini CLI' })).toBeVisible();
  await shots(page, 'harness-menu');
  await page.getByRole('menuitemradio', { name: 'Gemini CLI' }).click();
  await expect(page.locator('main.conv header.head')).toContainText('Gemini');
  await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à Gemini…' })).toBeVisible();
}

test.describe('an agent on Gemini CLI', () => {
  test.use({ appEnv: { ESCOUADE_GEMINI: FAKE_GEMINI } });

  test('converses over ACP, asks its permissions and questions from the window and the list, and resumes', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    // Réglages › Comptes et harness: found, with its version.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    const gemini = settings.getByRole('region', { name: 'Gemini CLI', exact: true });
    await expect(gemini.locator('[data-state]')).toHaveText('0.66.0');
    await gemini.scrollIntoViewIfNeeded();
    await shots(page, 'harness');
    await page.keyboard.press('Escape');

    await newGeminiAgent(page);
    await send(page, 'bonjour Gemini');
    await expect(page.getByText('Gemini a reçu : bonjour Gemini')).toBeVisible();
    // Started as Escouade drives it: `--acp`, in a GEMINI_CLI_HOME of the agent's own (never the user's ~/.gemini), its
    // folder trusted, its settings written there; Escouade's server by its session, never on its command line.
    const launch = agentLaunches(app)[0];
    expect(launch.argv).toEqual(['--acp']);
    expect(launch.trust).toBe(true);
    const homes = path.join(app.data, 'agent-homes');
    expect(path.dirname(launch.home!).toLowerCase()).toBe(homes.toLowerCase());
    expect(path.resolve(launch.cwd).toLowerCase()).toBe(path.resolve(app.repo).toLowerCase());
    const settingsFile = JSON.parse(fs.readFileSync(path.join(launch.home!, '.gemini', 'settings.json'), 'utf8'));
    expect(settingsFile.general.enableAutoUpdate).toBe(false);
    expect(launch.argv.join(' ')).not.toContain('Bearer');

    // A command: its card; « Pour cet agent » is « Allow for this session », asked no more.
    await send(page, 'commande');
    const card = page.getByTestId('permission-pending');
    await expect(card).toContainText('npm test');
    await shots(page, 'permission');
    await card.getByRole('button', { name: 'Pour cet agent' }).click();
    await expect(page.getByText('La commande a marché.')).toBeVisible();
    await send(page, 'commande encore');
    await expect(page.getByText('La commande a marché.')).toHaveCount(2);
    await expect(card).toHaveCount(0);

    // A write in its folder goes by Escouade's rules, its diff on its row.
    await send(page, 'fichier');
    await expect(page.getByText('J’ai modifié notes.txt.')).toBeVisible();
    await expect(page.locator('main.conv').getByText('notes.txt').first()).toBeVisible();
    expect(fs.readFileSync(path.join(app.repo, 'notes.txt'), 'utf8')).toContain('note de Gemini');

    // A question (its block, Gemini having no tool to ask in ACP): its card, answered from the list (Ctrl+J then 1).
    await send(page, 'question');
    const question = page.getByTestId('question-pending');
    await expect(question).toContainText('Quelle couleur préfères-tu ?');
    await expect(page.locator('main.conv')).not.toContainText('escouade_question');
    await shots(page, 'question');
    await page.waitForTimeout(600);
    await page.locator('body').evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    await page.keyboard.press('Control+j');
    await page.keyboard.press('1');
    await expect(page.getByText('Tu as choisi rouge.')).toBeVisible();
    await shots(page, 'turn');

    // Restarted, the app resumes its session (`session/load`), its history not told again.
    const page2 = await app.restart();
    await page2.locator('.card', { hasText: 'Gemini' }).first().click();
    await send(page2, 'tu te souviens ?');
    await expect(page2.getByText('Gemini a reçu : tu te souviens ?')).toBeVisible();
    expect(asked(app).some((m) => m.method === 'session/load')).toBe(true);
    await expect(page2.getByText('Gemini a reçu : bonjour Gemini')).toHaveCount(1);
  });

  test('its models are its session’s, its mode plan asks a plan to approve, and a turn stops', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newGeminiAgent(page);
    await send(page, 'bonjour');
    await expect(page.getByText('Gemini a reçu : bonjour')).toBeVisible();
    // The models of its session (`session/new`), set by `session/set_model`.
    await page.getByRole('button', { name: /^Modèle/ }).click();
    const menu = page.getByRole('menu', { name: 'Modèle' });
    await expect(menu.getByRole('menuitemradio', { name: /gemini-3\.8-flash/ })).toBeVisible();
    await menu.getByRole('menuitemradio', { name: /gemini-2\.5-pro/ }).click();
    await send(page, 'modèle');
    await expect(page.getByText('Je tourne sur gemini-2.5-pro, mode default.')).toBeVisible();

    // « Plan »: its mode plan, its plan read from the file it wrote, approved on its card, then carried out.
    await page.getByRole('button', { name: /^Mode/ }).click();
    await page.getByRole('menuitemradio', { name: /^Plan/ }).click();
    await send(page, 'propose un plan');
    const plan = page.getByRole('group', { name: 'Gemini propose un plan' });
    await expect(plan).toContainText('Écrire son test.');
    await shots(page, 'plan');
    await plan.getByRole('button', { name: 'Approuver le plan' }).click();
    await expect(page.getByText('Plan approuvé, je le mets en œuvre.')).toBeVisible();
    expect(asked(app).some((m) => m.method === 'session/set_mode' && m.params?.modeId === 'plan')).toBe(true);
    await expect(page.locator('main.conv')).not.toContainText('MODE_UPDATE');

    // A turn stopped (`session/cancel`), the next message goes.
    await send(page, 'lent');
    await expect(page.getByText('Je commence un long travail…')).toBeVisible();
    await page.getByRole('button', { name: 'Stop' }).click();
    await expect(page.getByRole('button', { name: 'Stop' })).toHaveCount(0);
    expect(asked(app).some((m) => m.method === 'session/cancel')).toBe(true);
    await page.waitForTimeout(4500);
    await expect(page.getByText('Long travail fini.')).toHaveCount(0);
    await send(page, 'encore');
    await expect(page.getByText('Gemini a reçu : encore')).toBeVisible();
  });
});

// ESC-275: its accounts, by a Google sign-in `gemini` makes in a terminal of the account's folder, or by an API key in the
// keychain; its usage on its page, its limit, its texts by `gemini -p`. No key of the machine's (`GEMINI_API_KEY` empty):
// its machine's account is not offered, its Google sign-in in ~/.gemini never touched.
test.describe('Gemini CLI’s accounts (ESC-275)', () => {
  test.use({
    appEnv: { ESCOUADE_GEMINI: FAKE_GEMINI, GEMINI_API_KEY: '', GOOGLE_API_KEY: '' },
    preStart: {
      run: () => ({
        noAccounts: true,
        accounts: [{ id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true }],
        // The texts (an agent's name…) written by Gemini CLI.
        writerEngine: 'gemini',
      }),
    },
  });

  test('signs a new account in to Google by gemini in a terminal, adds one by its key, links its usage, stops at its limit', async ({
    app,
  }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    await expect(settings.getByRole('region', { name: 'Gemini CLI', exact: true })).toContainText('Aucun compte');
    await settings.getByRole('button', { name: 'Ajouter un compte Gemini CLI…' }).click();
    const modal = page.getByRole('dialog', { name: 'Ajouter un compte Gemini CLI' });
    // The machine's: its sign-in is in ~/.gemini, which Escouade leaves alone.
    await expect(modal.getByText('Gemini CLI garde sa connexion Google dans ~/.gemini', { exact: false })).toBeVisible();
    await expect(modal.getByRole('radio', { name: /Le compte de la machine/ })).toBeDisabled();
    await expect(modal.getByRole('radio', { name: /Un nouveau compte/ })).toBeChecked();
    await expect(modal.getByRole('radio', { name: /Compte Google/ })).toBeChecked();
    await modal.getByRole('textbox', { name: 'Nom' }).fill('Perso');
    await shots(page, 'account-google');
    await modal.getByRole('button', { name: 'Créer et se connecter' }).click();
    // Its sign-in, by gemini in the terminal here, in the account's folder; seen, then the terminal goes.
    const signIn = page.getByRole('dialog', { name: 'Se connecter au compte « Perso »' });
    await expect(signIn.getByRole('status')).toContainText('gemini ouvre ton navigateur sur ton compte Google');
    await expect(signIn.getByRole('status')).toContainText('Connecté au compte moi@exemple.fr', { timeout: 20_000 });
    await shots(page, 'account-signed-in');
    await signIn.getByRole('button', { name: 'Terminé' }).click();
    const list = settings.getByRole('list', { name: 'Comptes Gemini CLI' });
    const perso = list.getByRole('listitem', { name: 'Perso' });
    await expect(perso).toContainText('Connecté · moi@exemple.fr');
    await expect(perso.getByRole('button', { name: 'Se reconnecter…' })).toBeVisible();
    // Another, by its key.
    await settings.getByRole('button', { name: 'Ajouter un compte Gemini CLI…' }).click();
    const add = page.getByRole('dialog', { name: 'Ajouter un compte Gemini CLI' });
    await add.getByRole('radio', { name: /Clé d’API/ }).click();
    await add.getByRole('textbox', { name: 'Nom' }).fill('Clé');
    await add.getByLabel('Clé d’API', { exact: true }).fill('AIzaCleGeminiE2e0000wxyz');
    await shots(page, 'account-key');
    await add.getByRole('button', { name: 'Ajouter le compte' }).click();
    const keyed = list.getByRole('listitem', { name: 'Clé' });
    await expect(keyed).toContainText('Connecté · clé …wxyz');
    await expect(keyed.getByRole('button', { name: 'Changer la clé…' })).toBeVisible();
    // Neither cost nor budget: its usage is on its page and its /stats.
    const usage = keyed.getByRole('button', { name: 'Usage sur aistudio.google.com' });
    await expect(usage).toBeVisible();
    await expect(keyed.getByRole('textbox', { name: 'Budget par jour ($)' })).toHaveCount(0);
    await expect(async () => {
      await page.mouse.move(0, 0);
      await usage.hover();
      await expect(page.getByRole('tooltip')).toContainText('se lit par /stats dans son interface', { timeout: 1500 });
    }).toPass({ timeout: 15_000 });
    await shots(page, 'accounts');
    const saved = fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8');
    expect(saved).not.toContain('AIzaCle');
    expect(saved).toMatch(/"apiKey":\s*true/);
    await page.keyboard.press('Escape');

    // New agents go to « Perso », signed in to Google: its process reads the sign-in of the account's folder, from its own
    // home, by GOOGLE_APPLICATION_CREDENTIALS; no key.
    await newGeminiAgent(page);
    await send(page, 'bonjour');
    await expect(page.getByText('Gemini a reçu : bonjour')).toBeVisible();
    const google = agentLaunches(app).at(-1)!;
    expect(google.credentials!.endsWith(path.join('.gemini', 'oauth_creds.json'))).toBe(true);
    expect(fs.existsSync(google.credentials!)).toBe(true);
    expect(path.dirname(google.home!).toLowerCase()).toBe(path.join(app.data, 'agent-homes').toLowerCase());
    expect(google.keyEnd).toBeNull();
    expect(google.auth).toBe('oauth-personal');
    // Its name, a text written by `gemini -p` without any tool, its question on its input.
    const texts = () => geminiLog(app).flatMap((e) => (e.launch?.prompt ? [e.launch] : []));
    await expect.poll(() => texts().length, { timeout: 20_000 }).toBeGreaterThan(0);
    const text = texts()[0];
    expect(text.argv[0]).toBe('-p');
    expect(text.argv).toContain('json');
    expect(text.policy).toContain('toolName = "*"');
    expect(text.input).toContain('bonjour');
    expect(text.argv.join(' ')).not.toContain('bonjour');
    // Its daily quota met: « limité », its resume planned.
    await send(page, 'quota du jour');
    const card = page.locator('.card.error.limit');
    await expect(card).toContainText('Limite d’usage de Gemini atteinte');
    await expect(card).toContainText('You have exhausted your daily quota on this model.');
    await expect(card).toContainText('Reprise automatique');
    await shots(page, 'limit');

    // « Perso » off: new agents go to the key's, which runs as it, its key never on its command line.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    await openSettingsPage(settings, 'Comptes et harness');
    await perso.getByRole('switch', { name: 'Actif' }).click();
    await expect(perso.getByRole('switch', { name: 'Actif' })).toHaveAttribute('aria-checked', 'false');
    await page.keyboard.press('Escape');
    await newGeminiAgent(page);
    await send(page, 'bonjour clé');
    await expect(page.getByText('Gemini a reçu : bonjour clé')).toBeVisible();
    const byKey = agentLaunches(app).at(-1)!;
    expect(byKey.keyEnd).toBe('wxyz');
    expect(byKey.credentials).toBeNull();
    expect(byKey.auth).toBe('gemini-api-key');
    expect(byKey.argv.join(' ')).not.toContain('AIzaCle');
  });
});

// ESC-275: the agent's session in Gemini's own interface (fake-gemini-tui.mjs), in its home, with Escouade's server in its
// settings (never on its command line), its hooks telling Escouade; back in Escouade's view, nothing lost.
test.describe('« Vue Gemini » (ESC-275)', () => {
  test.use({ appEnv: { ESCOUADE_GEMINI: FAKE_GEMINI }, preStart: { run: () => ({ mcpEnabled: true }) } });

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

  test('opens Gemini’s interface on the agent’s session with Escouade’s server, follows its turns by its hooks, and hands it back', async ({
    app,
  }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newGeminiAgent(page);
    await send(page, 'bonjour Gemini');
    await expect(page.getByText('Gemini a reçu : bonjour Gemini')).toBeVisible();

    await page.getByRole('button', { name: 'Vue Gemini CLI' }).click();
    const view = page.locator('main .tui');
    await expect(view.locator('.xterm')).toBeVisible({ timeout: 20_000 });
    await expect.poll(() => ofHomes(app, 'fake-tui-launches.jsonl').length, { timeout: 20_000 }).toBe(1);
    // Its interface on the session (one store for both), in the agent's home, with Escouade's server in its settings: its
    // token in that file, never on its command line.
    const launch = ofHomes(app, 'fake-tui-launches.jsonl')[0];
    expect(launch.argv).toContain('--resume');
    const session = launch.argv[launch.argv.indexOf('--resume') + 1];
    const header: string = launch.settings.mcpServers.escouade.headers.Authorization;
    const token = header.replace(/^Bearer /, '');
    expect(token).toBeTruthy();
    expect(launch.argv.join(' ')).not.toContain(token);
    expect(launch.trust).toBe(true);
    expect(launch.settings.hooksConfig.enabled).toBe(true);
    expect(Object.keys(launch.settings.hooks).sort()).toEqual([
      'AfterAgent',
      'AfterTool',
      'BeforeAgent',
      'BeforeTool',
      'Notification',
      'SessionStart',
    ]);

    // Messages typed there: the card says it works, then that it is done. The terminal is drawn on a canvas: what the
    // fake does is read from its hooks and the conversation.
    await view.locator('.xterm').click();
    const ended = () => ofHomes(app, 'fake-tui-hooks.jsonl').filter((e) => e.event === 'AfterAgent').length;
    // Its first screen drawn (its SessionStart run) before a key: a slow machine (Windows CI) would else type into an
    // interface not ready yet, which leaves the line.
    await expect.poll(() => ofHomes(app, 'fake-tui-hooks.jsonl').some((e) => e.event === 'SessionStart'), { timeout: 20_000 }).toBe(true);
    await page.keyboard.type('Bonjour la vue');
    await page.keyboard.press('Enter');
    await expect.poll(ended, { timeout: 20_000 }).toBe(1);
    await page.keyboard.type('commande');
    await page.keyboard.press('Enter');
    await expect.poll(ended, { timeout: 20_000 }).toBe(2);
    expect(ofHomes(app, 'fake-tui-hooks.jsonl').map((e) => e.event)).toEqual([
      'SessionStart',
      'BeforeAgent',
      'AfterAgent',
      'BeforeAgent',
      'BeforeTool',
      'AfterTool',
      'AfterAgent',
    ]);
    await page.waitForTimeout(500);
    await shots(page, 'view');

    // Back in Escouade's view: the interface's turns in the conversation, its command among them; the same session
    // loaded by `gemini --acp`, the hooks of its view gone from its settings.
    await page.getByRole('button', { name: 'Vue Escouade' }).click();
    await expect(page.getByText('Fini : Bonjour la vue')).toBeVisible();
    await expect(page.getByText('Fini : commande')).toBeVisible();
    await expect(page.locator('main.conv .bubble', { hasText: /^\s*Bonjour la vue\s*$/ })).toHaveCount(1);
    await expect(page.locator('main.conv').getByText('npm test').first()).toBeVisible();
    await send(page, 'encore');
    await expect(page.getByText('Gemini a reçu : encore')).toBeVisible();
    expect(asked(app).some((m) => m.method === 'session/load' && m.params?.sessionId === session)).toBe(true);
    const after = JSON.parse(fs.readFileSync(path.join(launch.home, '.gemini', 'settings.json'), 'utf8'));
    expect(after.hooks).toBeUndefined();
    await shots(page, 'view-back');
  });
});
