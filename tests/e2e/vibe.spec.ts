// Mistral Vibe (ESC-266): agents on the fake `vibe-acp` (tests/fixtures/fake-vibe.mjs), which speaks ACP v1 as Mistral
// Vibe 3.0.3 does, and whose message picks what a turn does. What the app sent it is in its log (FAKE_VIBE_LOG).
// ESCOUADE_E2E_SHOTS=<folder>: its screenshots, in both themes.

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_VIBE, send, test, type App, openSettingsPage } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

type Entry = {
  launch?: { argv: string[]; cwd: string; home: string | null; apiKey: string | null; telemetry: string | null };
  dir?: string;
  msg?: any;
};

function vibeLog(app: App): Entry[] {
  const file = path.join(app.root, 'fake-vibe.jsonl');
  if (!fs.existsSync(file)) return [];
  return fs
    .readFileSync(file, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));
}

/** The launch of an agent's process: not a `vibe-acp` that read its account's models, in a VIBE_HOME of its own. */
function agentLaunch(app: App) {
  return vibeLog(app).find((e) => e.launch && !path.basename(e.launch.home ?? '').startsWith('escouade-vibe-'))!.launch!;
}

/** The parameters of each request `method` the app sent, in order. */
const sent = (app: App, method: string) =>
  vibeLog(app)
    .filter((e) => e.dir === 'out' && e.msg?.method === method)
    .map((e) => e.msg.params);

/** The answers the app gave to the requests of the fake (permissions, questions). */
const replies = (app: App) =>
  vibeLog(app)
    .filter((e) => e.dir === 'out' && e.msg && e.msg.method === undefined)
    .map((e) => e.msg.result);

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

/** A new agent on Mistral Vibe, from the menu beside « Nouvel agent ». */
async function newVibeAgent(page: Page) {
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitemradio', { name: 'Mistral Vibe' }).click();
  await expect(page.locator('main.conv header.head')).toContainText('Mistral');
  await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à Vibe…' })).toBeVisible();
}

test.describe('an agent on Mistral Vibe', () => {
  test.use({ appEnv: { ESCOUADE_VIBE: FAKE_VIBE } });

  test('converses, asks its permissions with a write’s diff, from the window and the list, and ends its turns', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    // Réglages › Comptes et harness: found, with its version.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    const vibe = settings.getByRole('region', { name: 'Mistral Vibe', exact: true });
    await expect(vibe.locator('[data-state]')).toHaveText('3.0.3');
    await vibe.scrollIntoViewIfNeeded();
    await shots(page, 'harness');
    await page.keyboard.press('Escape');

    await newVibeAgent(page);
    await send(page, 'bonjour Vibe');
    await expect(page.getByText('Vibe a reçu : bonjour Vibe')).toBeVisible();
    // Started as Escouade drives Vibe: `vibe-acp` without arguments, in a VIBE_HOME of the agent's own, its telemetry off,
    // asking for its questions; Escouade's server by its session, never its command line.
    const launch = agentLaunch(app);
    expect(launch.argv).toEqual([]);
    expect(launch.telemetry).toBe('false');
    const homes = path.join(app.data, 'agent-homes');
    expect(path.dirname(launch.home!).toLowerCase()).toBe(homes.toLowerCase());
    expect(sent(app, 'initialize').some((p) => p.clientCapabilities?.elicitation?.form)).toBe(true);
    const opened = sent(app, 'session/new').at(-1);
    expect(path.resolve(opened.cwd).toLowerCase()).toBe(path.resolve(app.repo).toLowerCase());
    // Its folder trusted in its own VIBE_HOME.
    expect(fs.readFileSync(path.join(launch.home!, 'trusted_folders.toml'), 'utf8')).toContain('trusted = [');

    // A command: its permission names its call alone, the card has its command; « Pour cet agent » keeps the rule in its
    // VIBE_HOME (Vibe's « Always allow »).
    await send(page, 'commande');
    const card = page.getByTestId('permission-pending');
    await expect(card).toContainText('npm test');
    await shots(page, 'permission');
    await card.getByRole('button', { name: 'Pour cet agent' }).click();
    await expect(page.getByText('La commande a marché.')).toBeVisible();
    expect(replies(app).at(-1)).toEqual({ outcome: { outcome: 'selected', optionId: 'allow_always_permanent' } });
    expect(fs.readFileSync(path.join(launch.home!, 'config.toml'), 'utf8')).toContain('"npm"');
    await send(page, 'commande encore');
    await expect(page.getByText('La commande a marché.')).toHaveCount(2);
    await expect(card).toHaveCount(0);

    // A write in its folder goes by Escouade's rules, without a card, its diff on its row; one beside it asks, with its
    // diff, answered from the list (Ctrl+J then 1).
    await send(page, 'fichier');
    await expect(page.getByText('J’ai modifié notes.txt.')).toBeVisible();
    await send(page, 'fichier dehors');
    await expect(card).toContainText('Hors du dossier de l’agent');
    await expect(card.locator('.patch, .patch-view, pre').first()).toContainText('note de Vibe');
    await shots(page, 'write-outside');
    await page.waitForTimeout(600);
    await page.locator('body').evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    await page.keyboard.press('Control+j');
    await page.keyboard.press('1');
    await expect(page.getByText('J’ai modifié vibe-dehors.txt.')).toBeVisible();
    expect(replies(app).at(-1)).toEqual({ outcome: { outcome: 'selected', optionId: 'allow_once' } });

    // A question (elicitation/create): its card, answered by its choice's value.
    await send(page, 'question');
    const question = page.getByTestId('question-pending');
    await expect(question).toContainText('Quelle couleur préfères-tu ?');
    await shots(page, 'question');
    await question.getByRole('button', { name: 'bleu' }).click();
    await expect(page.getByText('Tu as choisi bleu.')).toBeVisible();
    expect(replies(app).at(-1)).toEqual({ action: 'accept', content: { q0: 'bleu' } });

    // Its tasks and its subagent in the « Plan » banner.
    await send(page, 'sous-agent');
    await expect(page.getByText('Tâches faites, le sous-agent a résumé le README.')).toBeVisible();
    const banner = page.locator('main.conv .plan:has(> .inner)');
    await expect(banner).toContainText('2/2');
    await shots(page, 'turn');

    // Restarted, the app resumes its session (session/load).
    const page2 = await app.restart();
    await page2.locator('.card', { hasText: 'Mistral' }).first().click();
    await send(page2, 'encore là ?');
    await expect(page2.getByText('Vibe a reçu : encore là ?')).toBeVisible();
    expect(sent(app, 'session/load').length).toBe(1);
    await expect(page2.getByText('Vibe a reçu : bonjour Vibe')).toHaveCount(1);
  });

  test('its models and thinking are its session’s, its mode Plan asks a plan to approve, and a turn stops', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newVibeAgent(page);
    await send(page, 'bonjour');
    await expect(page.getByText('Vibe a reçu : bonjour')).toBeVisible();
    // The models of its session (configOptions), and its thinking as the effort.
    await page.getByRole('button', { name: /^Modèle/ }).click();
    const menu = page.getByRole('menu', { name: 'Modèle' });
    await expect(menu.getByRole('menuitemradio', { name: /Mistral Medium 3\.5/ })).toBeVisible();
    await menu.getByRole('menuitemradio', { name: /Devstral 2/ }).click();
    await expect.poll(() => sent(app, 'session/set_config_option').map((p) => `${p.configId}=${p.value}`)).toContain('model=devstral-2');
    await send(page, 'modèle');
    await expect(page.getByText(/^Je tourne sur devstral-2, réflexion high, agent ask\.$/)).toBeVisible();

    // « Plan »: Vibe stays in its agent `ask`, its last words a plan to approve; approved, carried out in the same turn.
    await page.getByRole('button', { name: /^Mode/ }).click();
    await expect(page.getByText(/Vibe n’a pas de mode Plan/).first()).toBeVisible();
    await page.getByRole('menuitemradio', { name: /^Plan/ }).click();
    await send(page, 'propose un plan');
    const plan = page.getByRole('group', { name: 'Vibe propose un plan' });
    await expect(plan).toContainText('Ajouter sub');
    await shots(page, 'plan');
    await plan.getByRole('button', { name: 'Approuver le plan' }).click();
    await expect(page.getByText('Plan approuvé, je le mets en œuvre.')).toBeVisible();
    await expect.poll(() => sent(app, 'session/set_mode').at(-1)?.modeId).toBe('ask');

    // A turn stopped (session/cancel).
    await send(page, 'lent');
    await expect(page.getByText('Je commence un long travail…')).toBeVisible();
    await page.getByRole('button', { name: 'Stop' }).click();
    await expect.poll(() => sent(app, 'session/cancel').length).toBe(1);
    await expect(page.getByRole('button', { name: 'Stop' })).toHaveCount(0);
    await expect(page.getByText('Long travail fini.')).toHaveCount(0);
  });
});

test.describe('« Vue Mistral Vibe » (ESC-268)', () => {
  test.use({ appEnv: { ESCOUADE_VIBE: FAKE_VIBE }, preStart: { run: () => ({ mcpEnabled: true }) } });

  /** The sessions opened in the agent's folder (not the one that read the account's models). */
  const agentSessions = (app: App) => sent(app, 'session/new').filter((p) => path.resolve(p.cwd) === path.resolve(app.repo));

  /** The hooks the fake interface ran, in the agent's VIBE_HOME `home`. */
  const hooksRun = (home: string): { event: string }[] => {
    const file = path.join(home, 'fake-tui-hooks.jsonl');
    if (!fs.existsSync(file)) return [];
    return fs
      .readFileSync(file, 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((l) => JSON.parse(l));
  };

  /** The launches of the fake interface, in the agent's own VIBE_HOME. */
  const launches = (app: App): any[] => {
    const homes = path.join(app.data, 'agent-homes');
    if (!fs.existsSync(homes)) return [];
    return fs
      .readdirSync(homes)
      .map((id) => path.join(homes, id, 'fake-tui-launches.jsonl'))
      .filter((f) => fs.existsSync(f))
      .flatMap((f) => fs.readFileSync(f, 'utf8').split('\n').filter(Boolean))
      .map((l) => JSON.parse(l));
  };

  test('opens Vibe’s interface on the agent’s session with Escouade’s server, follows its turns, and hands the session back', async ({
    app,
  }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newVibeAgent(page);
    await send(page, 'bonjour Vibe');
    await expect(page.getByText('Vibe a reçu : bonjour Vibe')).toBeVisible();
    expect(agentSessions(app)).toHaveLength(1);

    await page.getByRole('button', { name: 'Vue Mistral Vibe' }).click();
    const view = page.locator('main .tui');
    await expect(view.locator('.xterm')).toBeVisible({ timeout: 20_000 });
    await expect.poll(() => launches(app).length, { timeout: 20_000 }).toBe(1);
    // Its interface on the session (one store for both), in the agent's VIBE_HOME, with Escouade's server in its
    // variables: its token in a variable of its own, never on its command line.
    const launch = launches(app)[0];
    const session = launch.argv[launch.argv.indexOf('--resume') + 1];
    expect(launch.argv).toContain('--resume');
    expect(launch.argv).toContain('--agent');
    const servers = JSON.parse(launch.mcpServers);
    expect(servers[0]).toMatchObject({ name: 'escouade', transport: 'http', api_key_env: 'ESCOUADE_MCP_TOKEN' });
    expect(launch.mcpToken).toBeTruthy();
    expect(launch.argv.join(' ')).not.toContain(launch.mcpToken);
    expect(launch.mcpServers).not.toContain(launch.mcpToken);
    // Its hooks and the trust of the folder, in its VIBE_HOME: no question about the folder, no band.
    expect(launch.hooks).toContain('post_agent');
    expect(launch.trusted).toContain('trusted = [');
    await expect(view.getByRole('status')).toHaveCount(0);

    // A message typed there: the card says it works, then that it is done. The terminal is drawn on a canvas: what the
    // fake does is read from the conversation.
    await view.locator('.xterm').click();
    const ended = () => hooksRun(launch.home).filter((e) => e.event === 'post_agent').length;
    await page.keyboard.type('Bonjour la vue');
    await page.keyboard.press('Enter');
    await expect.poll(ended, { timeout: 20_000 }).toBe(1);
    // Then a command, in the next segment of its journal: its hooks before and after it.
    await page.keyboard.type('commande');
    await page.keyboard.press('Enter');
    await expect.poll(ended, { timeout: 20_000 }).toBe(2);
    expect(hooksRun(launch.home).map((e) => e.event)).toEqual(['post_agent', 'pre_tool', 'post_tool', 'post_agent']);
    await expect(page.locator('.card', { hasText: 'Mistral' })).toContainText('Terminé');
    await page.waitForTimeout(500);
    await shots(page, 'view');

    // Back in Escouade's view: the interface's turns in the conversation, its command among them; the same session
    // resumed by its process, the hooks of its view gone.
    await page.getByRole('button', { name: 'Vue Escouade' }).click();
    await expect(page.getByText('Fini : Bonjour la vue')).toBeVisible();
    await expect(page.getByText('Fini : commande')).toBeVisible();
    await expect(page.locator('main.conv .bubble', { hasText: /^\s*Bonjour la vue\s*$/ })).toHaveCount(1);
    await expect(page.locator('main.conv').getByText('npm test').first()).toBeVisible();
    await send(page, 'encore');
    await expect(page.getByText('Vibe a reçu : encore')).toBeVisible();
    expect(agentSessions(app)).toHaveLength(1);
    expect(sent(app, 'session/load').at(-1).sessionId).toBe(session);
    expect(fs.existsSync(path.join(launch.home, 'hooks.toml'))).toBe(false);
    await shots(page, 'view-back');
  });
});

// ESC-267: a second Mistral Vibe account is an API key, kept in the system keychain (the sandbox's entries, apart from
// the user's) and given to its agents by MISTRAL_API_KEY, never on their command line nor in a file of Escouade's; what a
// turn costs is what its session's cost grew by; a rate limit ends the turn « limité », its resume planned.
test.describe('a Mistral Vibe account added by its API key', () => {
  test.use({ appEnv: { ESCOUADE_VIBE: FAKE_VIBE } });

  test('is added by its name and key, shows its cost and links its usage, and its agents run as that key', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    await settings.getByRole('button', { name: 'Ajouter un compte Mistral Vibe…' }).click();
    const modal = page.getByRole('dialog', { name: 'Ajouter un compte Mistral Vibe' });
    // The machine's is there already: a new one, by its key.
    await expect(modal.getByRole('radio', { name: /Un nouveau compte/ })).toBeChecked();
    await modal.getByRole('textbox', { name: 'Nom' }).fill('Équipe');
    await modal.getByLabel('Clé d’API').fill('cle-mistral-e2e-0000wxyz');
    await expect(modal.getByLabel('Clé d’API')).toHaveAttribute('placeholder', 'Sur console.mistral.ai, API Keys');
    await shots(page, 'account-key');
    await modal.getByRole('button', { name: 'Ajouter le compte' }).click();
    const list = settings.getByRole('list', { name: 'Comptes Mistral Vibe' });
    const team = list.getByRole('listitem', { name: 'Équipe' });
    await expect(team).toContainText('Connecté · clé …wxyz');
    await expect(team.getByRole('button', { name: 'Changer la clé…' })).toBeVisible();
    await expect(team.getByRole('button', { name: 'Usage sur admin.mistral.ai' })).toBeVisible();
    await expect(team.getByRole('textbox', { name: 'Budget par jour ($)' })).toBeVisible();
    expect(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8')).not.toContain('cle-mistral');
    // The machine's account off: new Vibe agents go to the key's.
    const machine = list.getByRole('listitem', { name: 'Principal' });
    await machine.getByRole('switch', { name: 'Actif' }).click();
    await expect(machine.getByRole('switch', { name: 'Actif' })).toHaveAttribute('aria-checked', 'false');
    await page.keyboard.press('Escape');
    await newVibeAgent(page);
    await send(page, 'compte');
    await expect(page.getByText('Compte : clé …wxyz')).toBeVisible();
    const launch = agentLaunch(app);
    expect(launch.apiKey).toBe('cle-mistral-e2e-0000wxyz');
    expect(launch.argv.join(' ')).not.toContain('cle-mistral');
    // Its turn's cost and tokens, as the fake's session grew by them.
    await expect(page.locator('main.conv').getByText('0,012 $').first()).toBeVisible();
    await send(page, 'encore');
    await expect(page.getByText('Vibe a reçu : encore')).toBeVisible();
    await expect(page.locator('main.conv').getByText('0,012 $')).toHaveCount(2);
    await shots(page, 'turn-cost');
    // The day's cost of the account, in Réglages › Comptes et harness.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    await openSettingsPage(settings, 'Comptes et harness');
    await expect(team).toContainText('0,025 $');
    await team.scrollIntoViewIfNeeded();
    await shots(page, 'accounts');
  });

  // ESC-327: one Mistral account for the plugin. On the sandbox's keychain, never the user's; it outlives a run, so
  // the voice's key leaves it: a key left there would serve the other tests' accounts.
  test('shares the Mistral account with the voice, each side going by the other’s key', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    await settings.getByRole('button', { name: 'Ajouter un compte Mistral Vibe…' }).click();
    let modal = page.getByRole('dialog', { name: 'Ajouter un compte Mistral Vibe' });
    await modal.getByRole('textbox', { name: 'Nom' }).fill('Équipe');
    await modal.getByLabel('Clé d’API').fill('cle-mistral-e2e-0000wxyz');
    await modal.getByRole('button', { name: 'Ajouter le compte' }).click();
    const list = settings.getByRole('list', { name: 'Comptes Mistral Vibe' });
    await expect(list.getByRole('listitem', { name: 'Équipe' })).toContainText('Connecté · clé …wxyz');
    // The voice has no key: it goes by the Vibe account's, and says so.
    await openSettingsPage(settings, 'Voix');
    const showKey = async () => {
      await settings.getByRole('group', { name: 'Transcrite par' }).getByRole('button', { name: 'En ligne' }).click();
      await settings.getByRole('combobox', { name: 'Service' }).selectOption('mistral');
    };
    await showKey();
    const row = settings.locator('[data-online-key=mistral]');
    const erase = row.getByRole('button', { name: 'Effacer la clé' });
    // Left by a run that failed.
    await expect(row.getByText(/Celle du compte Vibe|Enregistrée dans le trousseau/)).toBeVisible();
    if (await erase.isVisible()) await erase.click();
    await expect(row).toContainText('Celle du compte Vibe « Équipe » · clé …wxyz');
    await expect(row.getByRole('button', { name: 'Effacer la clé' })).toHaveCount(0);
    // Its own key: the Mistral account, which a new Vibe account goes by without a key of its own.
    await row.getByRole('button', { name: 'Donner une autre clé' }).click();
    await row.getByLabel('Clé de Mistral').fill('cle-voix-e2e-00001234');
    await row.getByRole('button', { name: 'Enregistrer la clé' }).click();
    await expect(row).toContainText('Enregistrée dans le trousseau');
    await openSettingsPage(settings, 'Comptes et harness');
    await settings.getByRole('button', { name: 'Ajouter un compte Mistral Vibe…' }).click();
    modal = page.getByRole('dialog', { name: 'Ajouter un compte Mistral Vibe' });
    await modal.getByRole('textbox', { name: 'Nom' }).fill('Partagé');
    await expect(modal.getByLabel('Clé d’API')).toHaveAttribute('placeholder', 'Vide : la clé Mistral de Réglages › Voix (clé …1234)');
    await modal.getByRole('button', { name: 'Ajouter le compte' }).click();
    const shared = list.getByRole('listitem', { name: 'Partagé' });
    await expect(shared).toContainText('Connecté · clé …1234');
    // The other accounts off: the agent runs as the Mistral account's key.
    for (const name of ['Principal', 'Équipe']) {
      await list.getByRole('listitem', { name }).getByRole('switch', { name: 'Actif' }).click();
    }
    await page.keyboard.press('Escape');
    await newVibeAgent(page);
    await send(page, 'compte');
    await expect(page.getByText('Compte : clé …1234')).toBeVisible();
    expect(agentLaunch(app).apiKey).toBe('cle-voix-e2e-00001234');
    expect(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8')).not.toContain('cle-voix');
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    await openSettingsPage(settings, 'Voix');
    await showKey();
    await erase.click();
    await expect(erase).toHaveCount(0);
  });

  test('ends a turn stopped by Mistral’s rate limit on a card that says it, its resume planned', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await newVibeAgent(page);
    await send(page, 'limite');
    const card = page.locator('.card.error.limit');
    await expect(card).toContainText('Limite de débit de Mistral atteinte');
    await expect(card).toContainText('Rate limit exceeded for mistral');
    await expect(card).toContainText('Reprise automatique');
    await shots(page, 'rate-limit');
  });
});
