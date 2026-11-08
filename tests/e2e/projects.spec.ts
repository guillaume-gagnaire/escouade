import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, openNewProject, send, test, type App, openSettingsPage } from './fixture';

// Closing a project keeps its history (ESC-89): adding its folder again gives it back, unless it was erased.

/** The placeholder of the folder field in « Nouveau projet ». */
const FOLDER_PLACEHOLDER = 'C:\\chemin\\vers\\le\\projet';
const HISTORY_FOUND = /Historique retrouvé\s*:\s*1 agent, 1 ticket, dernière activité le \S+/;
const BLANK = /Partir d.un projet vierge/;
const ERASE_BOX = 'Effacer aussi son historique (conversations, tickets, réglages)';

/** A git repository with one commit, named `name`, in the test's folder. */
function repoNamed(root: string, name: string): string {
  const dir = path.join(root, name);
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(dir, 'README.md'), `# ${name}\n`);
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

/** The same folder, written another way: other case, `/` for `\`, and a separator at the end. */
function rewritten(dir: string): string {
  return `${dir.replaceAll('\\', '/').toUpperCase()}/`;
}

/** What the app keeps of the projects closed with their history, in its data folder. */
function closedFiles(app: App): string[] {
  const dir = path.join(app.data, 'closed-projects');
  return fs.existsSync(dir) ? fs.readdirSync(dir).filter((f) => f.endsWith('.json')) : [];
}

/** The conversation journals of the agents, in the data folder. */
function conversationLogs(app: App): string[] {
  const dir = path.join(app.data, 'conversations');
  return fs.existsSync(dir) ? fs.readdirSync(dir).filter((f) => f.endsWith('.jsonl')) : [];
}

/** The ids of the open projects, as the app saved them. */
function savedProjectIds(app: App): string[] {
  const file = path.join(app.data, 'state.json');
  if (!fs.existsSync(file)) return [];
  return (JSON.parse(fs.readFileSync(file, 'utf8')).projects ?? []).map((p: { id: string }) => p.id);
}

/** What a project keeps: an agent that answered, and a ticket left « À faire » on its board (DEM-1). */
async function doSomeWork(page: Page) {
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  await expect(page.getByText('Tâche terminée')).toBeVisible();
  // Named from the first request: the name is saved with the agent.
  await expect(page.locator('.card .name').first()).toHaveText('bonjour-fake');
  await page.getByRole('button', { name: /^Kanban/ }).click();
  // Nothing starts: the ticket stays « À faire ».
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Le ticket gardé');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(page.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /DEM-1/ })).toBeVisible();
}

/** Closes the project from its tile's menu, keeping its history unless `erase` ticks the box. */
async function closeProject(page: Page, name: string, opts: { erase?: boolean } = {}) {
  const tab = page.getByRole('tab', { name: new RegExp(`^${name}`) });
  await tab.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Fermer le projet…' }).click();
  const dialog = page.getByRole('dialog', { name: `Fermer « ${name} » ?` });
  await expect(dialog.getByText(/Son historique \(conversations, tickets, réglages\) est gardé/)).toBeVisible();
  // Unchecked: closing keeps the history unless the user says otherwise.
  const erase = dialog.getByRole('checkbox', { name: ERASE_BOX });
  await expect(erase).not.toBeChecked();
  if (opts.erase) await erase.check();
  await dialog.getByRole('button', { name: 'Fermer le projet', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(tab).toHaveCount(0);
}

/**
 * Opens « Nouveau projet » and types `folder`; the dialog is returned once the folder was read. Found by its folder field,
 * not by its title: the title may change once the folder has a history.
 */
async function typeFolder(page: Page, folder: string) {
  await openNewProject(page);
  const dialog = page.getByRole('dialog').filter({ has: page.getByPlaceholder(FOLDER_PLACEHOLDER) });
  await dialog.getByPlaceholder(FOLDER_PLACEHOLDER).fill(folder);
  await expect(dialog.getByText(/Dépôt git détecté/)).toBeVisible();
  return dialog;
}

/** Turns the « Créer un premier agent » switch off, if the dialog has it on. */
async function noFirstAgent(dialog: Locator) {
  const first = dialog.getByRole('switch', { name: 'Créer un premier agent' });
  if ((await first.count()) && (await first.getAttribute('aria-checked')) === 'true') await first.click();
}

/** Opens the settings on « Tous les projets », where the closed ones are (they may be open already: a question asked from them can bring them back). */
async function projectsSettings(page: Page) {
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  if (!(await settings.isVisible())) await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  await openSettingsPage(settings, 'Tous les projets');
  return settings;
}

/** The group of the settings that lists the projects closed with their history. */
function closedGroup(page: Page) {
  const title = page.getByRole('heading', { name: 'Projets fermés' });
  return page.getByRole('dialog', { name: 'Réglages' }).locator('section.sg').filter({ has: title });
}

test('a closed project comes back with its agent, its conversation and its ticket when its folder is added again', async ({ app }) => {
  test.setTimeout(150_000);
  const { page } = app;
  await addProject(page, app.repo);
  await doSomeWork(page);
  await expect.poll(() => savedProjectIds(app)).toHaveLength(1);
  const [projectId] = savedProjectIds(app);
  await closeProject(page, 'demo-api');

  // Its history is kept, in a file of its own named after the project's id; the conversation stays on the disk.
  await expect.poll(() => closedFiles(app)).toEqual([`${projectId}.json`]);
  expect(conversationLogs(app)).toHaveLength(1);

  // The same folder, written another way (case, slashes, a separator at the end): the history is found.
  const dialog = await typeFolder(page, rewritten(app.repo));
  await expect(dialog).toContainText(HISTORY_FOUND);
  await expect(dialog.getByRole('switch', { name: BLANK })).toHaveAttribute('aria-checked', 'false');
  // The name comes from the history, not from the folder as it was typed.
  await expect(dialog.locator('input').nth(1)).toHaveValue('demo-api');
  await expect(dialog.getByRole('button', { name: 'Créer le projet' })).toHaveCount(0);
  await noFirstAgent(dialog);
  await dialog.getByRole('button', { name: 'Rouvrir le projet' }).click();
  await expect(dialog).toBeHidden();

  // The project is back, with its own id, its agent and its ticket; the file of its history is gone.
  await expect(page.getByRole('tab', { name: /^demo-api/ })).toHaveAttribute('aria-selected', 'true');
  await expect.poll(() => savedProjectIds(app)).toEqual([projectId]);
  await expect.poll(() => closedFiles(app)).toEqual([]);
  // Its agent's card in the sidebar (the conversation shown has cards of its own: its turns).
  await expect(page.locator('.card .name')).toHaveCount(1);
  await expect(page.locator('.card .name')).toHaveText('bonjour-fake');
  await page.locator('.card', { has: page.locator('.name') }).click();
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const todo = page.getByRole('region', { name: 'À faire' });
  await expect(todo.getByRole('button', { name: /DEM-1/ })).toBeVisible();
  await expect(todo.getByText('Le ticket gardé')).toBeVisible();
});

test('a blank project leaves the history among the closed projects, where it is erased for good', async ({ app }) => {
  test.setTimeout(150_000);
  const { page } = app;
  // A project that stays open: the settings of a project are the ones the closed projects are listed in.
  await addProject(page, repoNamed(app.root, 'autre-projet'), { firstAgent: false });
  await addProject(page, app.repo);
  await doSomeWork(page);
  await closeProject(page, 'demo-api');
  await expect.poll(() => closedFiles(app)).toHaveLength(1);
  const history = closedFiles(app);
  expect(conversationLogs(app)).toHaveLength(1);

  // The history is found, and the user starts from nothing: the project has neither the agent nor the ticket.
  const dialog = await typeFolder(page, rewritten(app.repo));
  await expect(dialog).toContainText(HISTORY_FOUND);
  await dialog.getByRole('switch', { name: BLANK }).click();
  await expect(dialog.getByRole('switch', { name: BLANK })).toHaveAttribute('aria-checked', 'true');
  await dialog.locator('input').nth(1).fill('demo-vierge');
  // Created from scratch, without a first agent: through the assistant's summary.
  await dialog.getByRole('radio', { name: /^Partir de zéro/ }).click();
  await dialog.getByRole('button', { name: /Récapitulatif/ }).click();
  // Past the first step, the folder field is gone: the window is found by its title.
  const wizard = page.getByRole('dialog', { name: 'Nouveau projet' });
  await noFirstAgent(wizard);
  await wizard.getByRole('button', { name: 'Créer le projet' }).click();
  await expect(wizard).toBeHidden();
  await expect(page.getByRole('tab', { name: /^demo-vierge/ })).toHaveAttribute('aria-selected', 'true');
  await expect(page.locator('.card .name')).toHaveCount(0);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByRole('button', { name: 'Nouveau ticket' })).toBeVisible();
  await expect(page.getByText('Le ticket gardé')).toHaveCount(0);
  // Nothing was erased on the way: the history waits in its file.
  expect(closedFiles(app)).toEqual(history);
  expect(conversationLogs(app)).toHaveLength(1);

  // The settings list it among the closed projects, with its name and its folder.
  let settings = await projectsSettings(page);
  const group = closedGroup(page);
  await expect(group).toContainText('demo-api');
  // The folder, as the list writes it (« ~ » for the home folder): the test's folder is in it.
  await expect(group).toContainText(path.basename(app.root));

  // « Rouvrir » on its row: the window that adds a project, on its folder, already read, with its history found.
  await group.getByRole('button', { name: 'Rouvrir', exact: true }).click();
  const reopen = page.getByRole('dialog').filter({ has: page.getByPlaceholder(FOLDER_PLACEHOLDER) });
  await expect(reopen.getByText(/Dépôt git détecté/)).toBeVisible();
  await expect(reopen).toContainText(HISTORY_FOUND);
  await expect(reopen.getByPlaceholder(FOLDER_PLACEHOLDER)).not.toHaveValue('');
  await expect(reopen.getByRole('switch', { name: BLANK })).toHaveAttribute('aria-checked', 'false');
  await expect(reopen.locator('input').nth(1)).toHaveValue('demo-api');
  await expect(reopen.getByRole('button', { name: 'Rouvrir le projet' })).toBeEnabled();
  // Cancelled: nothing is reopened or erased, and the settings open again on the same list.
  await reopen.getByRole('button', { name: 'Annuler' }).click();
  await expect(reopen).toBeHidden();
  expect(closedFiles(app)).toEqual(history);
  settings = await projectsSettings(page);
  await expect(closedGroup(page)).toContainText('demo-api');

  await group.getByRole('button', { name: /Effacer l.historique/ }).click();
  const ask = page.getByRole('dialog', { name: /Effacer l.historique de/ });
  await expect(ask).toContainText('demo-api');
  await ask.getByRole('button', { name: /^Effacer l.historique$/ }).click();
  await expect(ask).toBeHidden();

  // Erased: out of the list, its file and its conversation gone; the folder has nothing to give back.
  settings = await projectsSettings(page);
  // The list is read when the tab opens: give it the time to be, were it still there.
  await page.waitForTimeout(500);
  await expect(settings.getByRole('heading', { name: 'Projets fermés' })).toHaveCount(0);
  await expect.poll(() => closedFiles(app)).toEqual([]);
  await expect.poll(() => conversationLogs(app)).toEqual([]);
  await page.keyboard.press('Escape');
  await expect(settings).toBeHidden();
  const again = await typeFolder(page, rewritten(app.repo));
  await expect(again.getByText(/Historique retrouvé/)).toHaveCount(0);
  await expect(again.getByRole('switch', { name: BLANK })).toHaveCount(0);
  await again.getByRole('button', { name: 'Annuler' }).click();
  await expect(again).toBeHidden();
});

test('a project closed with its history erased leaves nothing to find', async ({ app }) => {
  test.setTimeout(150_000);
  const { page } = app;
  await addProject(page, repoNamed(app.root, 'autre-projet'), { firstAgent: false });
  await addProject(page, app.repo);
  await doSomeWork(page);
  expect(conversationLogs(app)).toHaveLength(1);
  await closeProject(page, 'demo-api', { erase: true });

  // Nothing is kept: no file, no conversation, no closed project in the settings.
  await expect.poll(() => conversationLogs(app)).toEqual([]);
  expect(closedFiles(app)).toEqual([]);
  const settings = await projectsSettings(page);
  await expect(settings.getByRole('heading', { name: 'Nouveaux projets' })).toBeVisible();
  // The list is read when the tab opens: give it the time to be, were it there.
  await page.waitForTimeout(500);
  await expect(settings.getByRole('heading', { name: 'Projets fermés' })).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(settings).toBeHidden();

  // Added again, the folder is a new project: no history found.
  const dialog = await typeFolder(page, rewritten(app.repo));
  await expect(dialog.getByText(/Historique retrouvé/)).toHaveCount(0);
  await expect(dialog.getByRole('switch', { name: BLANK })).toHaveCount(0);
  await dialog.getByRole('button', { name: /^Créer sans l.assistant$/ }).click();
  await expect(dialog).toBeHidden();
  // A new agent, with nothing of the old conversation.
  await expect(page.locator('.card .name')).toHaveCount(1);
  await expect(page.locator('.card .name')).not.toHaveText('bonjour-fake');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toHaveCount(0);
});
