// The real trial of the store (PLG-12, ESC-184): the app (debug build, data of the test's, its public keys embedded) reads
// store.escouade.dev and installs Escouade's own plugins from it, their seals checked here against those keys: the permissions
// shown, the activation and what the plugin does, the deactivation, the uninstallation. The plugins the app gives are
// started too, one by one. No webhook nor token: Discord and Slack stop at their configuration.
//
// Skipped unless ESCOUADE_E2E_REAL_STORE is set: it reads the real store (a GET of the catalogue, of the versions and of their
// archives; nothing is counted, the statistics being off). ESCOUADE_E2E_REAL_OUT: a folder kept for the report and the
// screenshots. Never run it with ESCOUADE_STORE_TEST_KEY: the keys would no longer be the store's.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test, openSettingsPage } from './fixture';

const OUT = process.env.ESCOUADE_E2E_REAL_OUT ?? path.join(os.tmpdir(), 'escouade-real-store');

/** What the trial saw, kept for its report. */
function note(line: string) {
  fs.mkdirSync(OUT, { recursive: true });
  fs.appendFileSync(path.join(OUT, 'report.md'), `${line}\n`);
}

const shot = (page: Page, name: string) => page.screenshot({ path: path.join(OUT, `${name}.png`) });

test.skip(!process.env.ESCOUADE_E2E_REAL_STORE, 'ESCOUADE_E2E_REAL_STORE: reads the real store');

test.beforeAll(() => {
  process.env.ESCOUADE_STORE_URL = 'https://store.escouade.dev';
  process.env.ESCOUADE_STORE_MIRROR_URL = 'https://escouade.dev';
  delete process.env.ESCOUADE_STORE_TEST_KEY;
});

test.afterAll(() => {
  delete process.env.ESCOUADE_STORE_URL;
  delete process.env.ESCOUADE_STORE_MIRROR_URL;
});

async function openPlugins(page: Page) {
  // A second click of the rail's icon goes back to the project: only when the view is closed.
  if (!(await page.locator('.plugins-side').isVisible())) await page.getByRole('button', { name: 'Plugins', exact: true }).click();
}

async function openStore(page: Page) {
  await openPlugins(page);
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Store/ })
    .click();
  await expect(page.locator('.plugins-work .sub')).toContainText('catalogue lu à', { timeout: 30_000 });
}

async function openInstalled(page: Page) {
  await openPlugins(page);
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
}

/** From the store: its line, the window of its permissions, installed. */
async function install(page: Page, name: string, shots: string) {
  await openStore(page);
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill(name);
  const row = page.locator('.plugins-work .rowp', { hasText: name }).first();
  await expect(row).toContainText('Vérifié');
  await row.getByRole('button', { name: 'Installer' }).click();
  const dialog = page.getByRole('dialog', { name: `Installer ${name} ?` });
  await expect(dialog).toContainText('Vérifié par Escouade', { timeout: 30_000 });
  await expect(dialog).toContainText('vérifiée ici, par l’app');
  await expect(dialog).toContainText('archive reçue identique');
  await expect(dialog).toContainText('store.escouade.dev');
  note(`\n## ${name}\n\n### La fenêtre d'installation\n\n${(await dialog.innerText()).replace(/\n+/g, ' · ')}`);
  await shot(page, `${shots}-install`);
  await dialog.getByRole('button', { name: 'Installer', exact: true }).click();
  await expect(dialog).toBeHidden({ timeout: 30_000 });
  await expect(page.locator('.phead')).toContainText('Vérifié par Escouade');
  await expect(page.locator('.pp-aside')).toContainText('Relu par Escouade le');
  note(`\n### Sa page\n\n${(await page.locator('.pp-aside').innerText()).replace(/\n+/g, ' · ')}`);
  await shot(page, `${shots}-page`);
}

/** The page of an installed plugin, from the installed ones. */
async function openInstalledPlugin(page: Page, name: string) {
  await openInstalled(page);
  await page.getByRole('table', { name: 'Ajoutés' }).locator('.nm', { hasText: name }).click();
  await expect(page.locator('.phead')).toContainText(name);
}

async function uninstall(page: Page, name: string) {
  await openInstalledPlugin(page, name);
  await page.getByRole('button', { name: 'Désinstaller' }).click();
  await page
    .getByRole('dialog', { name: `Désinstaller ${name} ?` })
    .getByRole('button', { name: 'Désinstaller' })
    .click();
}

/** The folder of an installed plugin, and its proofs. */
function folders(data: string, id: string) {
  return [path.join(data, 'plugins', id), path.join(data, 'plugins', '.store', id)];
}

test('the store for real: four signed plugins installed, tried, turned off, taken away', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  await addProject(page, app.repo);
  fs.rmSync(path.join(OUT, 'report.md'), { force: true });
  note(`# L'essai réel du store (${new Date().toISOString()})`);

  // The catalogue, as read.
  await openStore(page);
  await shot(page, 'store');
  note(`\n## Le store\n\n${(await page.locator('.plugins-work').innerText()).replace(/\n+/g, ' · ').slice(0, 1500)}`);

  // High contrast: chosen, on the window; Escouade's colours back.
  await install(page, 'Contraste élevé', 'high-contrast');
  await page.keyboard.press('Control+,');
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Général');
  const colors = settings.getByRole('combobox', { name: 'Couleurs' });
  await expect(colors.locator('option', { hasText: 'Contraste élevé' }).first()).toBeAttached();
  const before = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--text'));
  await colors.selectOption({ index: 1 });
  await expect(page.locator('#escouade-plugin-theme')).toBeAttached();
  await shot(page, 'high-contrast-on');
  await expect
    .poll(() => JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8')).pluginTheme)
    .toContain('dev.escouade.high-contrast');
  const after = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--text'));
  note(`\n### Ce qu'il fait\n\nThème choisi dans Réglages › Général › Couleurs : --text ${before.trim()} → ${after.trim()}.`);
  await colors.selectOption({ index: 0 });
  await expect(page.locator('#escouade-plugin-theme')).toHaveCount(0);
  await settings.getByRole('button', { name: 'Fermer' }).click();

  // Discord: its webhook is a secret; the trial stops at the field.
  await install(page, 'Discord', 'discord');
  await page.getByRole('tab', { name: /Réglages/ }).click();
  note(`\n### Sa configuration\n\n${(await page.getByRole('tabpanel').innerText()).replace(/\n+/g, ' · ')}`);
  await shot(page, 'discord-settings');

  // Slack: its program starts; without a token nothing goes out, its configuration is the same field.
  await install(page, 'Slack', 'slack');
  await page.getByRole('tab', { name: 'Journal' }).click();
  const slackJournal = page.getByRole('tabpanel', { name: 'Journal' });
  await slackJournal.getByRole('button', { name: 'Démarrer' }).click();
  await expect(slackJournal).toContainText('En marche', { timeout: 30_000 });
  await expect(slackJournal.getByRole('list', { name: 'Journal' })).toContainText('Prêt', { timeout: 15_000 });
  note(`\n### Son journal\n\n${(await slackJournal.innerText()).replace(/\n+/g, ' · ')}`);
  await page.getByRole('tab', { name: /Réglages/ }).click();
  note(`\n### Sa configuration\n\n${(await page.getByRole('tabpanel').innerText()).replace(/\n+/g, ' · ')}`);
  await shot(page, 'slack-settings');

  // Team rules: the agent's `rm -rf build` refused by the plugin, before the user is asked.
  await install(page, "Règles de l'équipe", 'team-rules');
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await expect(page.locator('.plugins-side')).toBeHidden();
  const refusals = () => {
    const log = path.join(app.data, 'plugin-logs', 'dev.escouade.team-rules.log');
    return fs.existsSync(log) ? fs.readFileSync(log, 'utf8').split('refused Bash rm -rf build').length - 1 : 0;
  };
  // Cold, just installed: its process starts with the request, whose card is there meanwhile. Past 3 s the request stays
  // with the user, as it must.
  await send(page, 'permission');
  const answered = (on: Page) => on.locator('.req-card.done', { hasText: 'rm -rf build' });
  const pending = (on: Page) => on.getByTestId('permission-pending');
  await expect(pending(page).or(answered(page)).first()).toBeVisible({ timeout: 30_000 });
  const cold = await answered(page)
    .waitFor({ timeout: 8_000 })
    .then(() => true)
    .catch(() => false);
  if (!cold) await pending(page).getByRole('button', { name: 'Refuser' }).click();
  await expect(answered(page)).toHaveCount(1);
  note(
    `\n### À froid\n\n${cold ? 'Refusé par le plugin dans ses 3 s' : 'Pas de réponse dans ses 3 s : la demande est restée à l’utilisateur'}.`,
  );

  // Started again: the seals hold, read again from their proofs, and the plugin is running before it is asked.
  let restarted = await app.restart();
  await openInstalled(restarted);
  const added = restarted.getByRole('table', { name: 'Ajoutés' });
  note(`\n## Après un redémarrage\n\n${(await added.innerText()).replace(/\n+/g, ' · ')}`);
  await shot(restarted, 'installed-after-restart');
  await openInstalledPlugin(restarted, 'Discord');
  await expect(restarted.locator('.phead')).toContainText('Vérifié par Escouade');
  await openInstalledPlugin(restarted, "Règles de l'équipe");
  await restarted.getByRole('tab', { name: 'Journal' }).click();
  await expect(restarted.getByRole('tabpanel', { name: 'Journal' })).toContainText('En marche', { timeout: 30_000 });
  await restarted.getByRole('button', { name: 'Plugins', exact: true }).click();
  await expect(restarted.locator('.plugins-side')).toBeHidden();
  const refusedBefore = refusals();
  await send(restarted, 'permission');
  await expect.poll(refusals, { timeout: 15_000 }).toBe(refusedBefore + 1);
  const refused = answered(restarted).last();
  await expect(refused.locator('.by')).toContainText("Règles de l'équipe");
  await expect(refused.locator('.by')).toContainText('Refusé');
  note(`\n### Ce qu'il fait\n\n${(await refused.innerText()).replace(/\n+/g, ' · ')}`);
  await shot(restarted, 'team-rules-refused');

  // Team rules turned off: the same command goes to the user.
  await restarted.getByRole('button', { name: 'Plugins', exact: true }).click();
  await openInstalledPlugin(restarted, "Règles de l'équipe");
  await restarted.getByRole('button', { name: 'Désactiver' }).click();
  await expect(restarted.getByRole('button', { name: 'Activer' })).toBeVisible();
  note(`\n## Désactivé\n\nLe bouton dit « Activer ».`);
  restarted = await app.restart();
  // The window comes back on the view it was left on.
  await openPlugins(restarted);
  await restarted.getByRole('button', { name: 'Plugins', exact: true }).click();
  await expect(restarted.locator('.plugins-side')).toBeHidden();
  await send(restarted, 'permission');
  await expect(pending(restarted)).toContainText('rm -rf build', { timeout: 30_000 });
  note('Avec le plugin désactivé, la même commande arrive à l’utilisateur.');
  await shot(restarted, 'team-rules-off');
  await restarted
    .getByTestId('permission-pending')
    .getByRole('button', { name: /Refuser/ })
    .first()
    .click();

  // Uninstalled: their folders and their proofs gone.
  for (const [name, id] of [
    ['Contraste élevé', 'dev.escouade.high-contrast'],
    ['Discord', 'dev.escouade.discord'],
    ['Slack', 'dev.escouade.slack'],
    ["Règles de l'équipe", 'dev.escouade.team-rules'],
  ]) {
    expect(folders(app.data, id).every(fs.existsSync)).toBe(true);
    await uninstall(restarted, name);
    await expect.poll(() => folders(app.data, id).filter(fs.existsSync)).toEqual([]);
  }
  await expect(restarted.getByRole('table', { name: 'Ajoutés' })).toHaveCount(0);
  note('\n## Désinstallés\n\nLeur dossier et leurs preuves sont partis.');
});

test('the plugins the app gives: each one that has a program starts', async ({ app }) => {
  test.setTimeout(600_000);
  const { page } = app;
  await addProject(page, app.repo);
  await openInstalled(page);
  const table = page.getByRole('table', { name: 'Livrés avec Escouade' });
  await expect(table).toBeVisible();
  const names = await table.locator('.nm b').allInnerTexts();
  note(`\n## Les plugins livrés (${names.length})\n`);
  await shot(page, 'builtin');
  for (const name of names) {
    await openInstalled(page);
    await page.getByRole('table', { name: 'Livrés avec Escouade' }).locator('.nm', { hasText: name }).first().click();
    await expect(page.locator('.phead')).toContainText(name);
    const journal = page.getByRole('tab', { name: 'Journal' });
    if ((await journal.count()) === 0) {
      note(`- ${name} : pas de journal (déclaratif).`);
      continue;
    }
    await journal.click();
    const panel = page.getByRole('tabpanel', { name: 'Journal' });
    const start = panel.getByRole('button', { name: 'Démarrer' });
    // The panel is drawn a moment after its tab is chosen; a plugin already running has no button.
    await start.or(panel.getByText('En marche')).first().waitFor();
    // A plugin of a newer Escouade than this build does not start, and says so: written down, not a failure of the trial.
    if ((await start.count()) > 0 && (await start.isDisabled())) {
      note(`- ${name} : ne démarre pas ici : ${(await panel.innerText()).split('\n')[0]}`);
      continue;
    }
    if ((await start.count()) > 0) await start.click();
    await expect(panel).toContainText('En marche', { timeout: 30_000 });
    await expect(panel.getByRole('list', { name: 'Journal' })).toContainText('Prêt', { timeout: 15_000 });
    note(`- ${name} : en marche.`);
  }
});
