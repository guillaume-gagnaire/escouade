// The discussions of the Docs tab (ESC-261), with the fake claude: « Nouvelle discussion » (Ctrl+B) starts a brainstorm on a
// subject with a file joined, filed by itself in the Documents; its agent works in the docs' folder and proposes, by
// Escouade's docs tools, a page, a change of a page and a decision, shown in « Ce qui ira dans la spec » and written
// nowhere until « Verser dans la spec »; a write of the code it asks for is refused by Escouade. Then a discussion with the
// spec writer, and nothing committed by Escouade. The captures go to ESCOUADE_E2E_SHOTS, dark and light.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.mouse.move(0, 0);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `discussion-${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

// ESC-289: on a project without a spec, « Écrire une première spec depuis le code » opens « Nouvelle discussion » with its
// subject and its message; « Commencer » closes the window and opens the discussion, its first message sent. With new
// agents in the harness terminal too: a discussion never goes to the terminal view, where its first message was refused
// (the window stayed open, its agent made).
for (const terminalView of [false, true]) {
  test.describe(`the first spec, new agents ${terminalView ? 'in the harness terminal' : 'in Escouade'}`, () => {
    test.use({ preStart: { run: () => ({ mcpEnabled: true, terminalView }) } });

    test('the first spec from the code closes its window and opens its discussion, its first message sent', async ({ app }) => {
      test.setTimeout(120_000);
      const { page } = app;
      await page.setViewportSize({ width: 1004, height: 705 });
      await addProject(page, app.repo, { firstAgent: false, name: 'Portail' });
      expect(fs.existsSync(path.join(app.repo, '.escouade', 'spec'))).toBe(false);

      await page.getByRole('group', { name: 'Vue du projet' }).getByRole('button', { name: 'Docs' }).click();
      await page.getByRole('button', { name: 'Écrire une première spec depuis le code' }).click();
      const dialog = page.getByRole('dialog', { name: 'Nouvelle discussion' });
      await expect(dialog.getByRole('textbox').first()).toHaveValue('Première spec depuis le code');
      await dialog.getByRole('button', { name: 'Commencer' }).click();
      await expect(dialog).toBeHidden();

      const view = page.locator('.discussion[data-kind="discussion"]');
      await expect(view).toBeVisible();
      await expect(page.locator('button.disc[data-kind="discussion"]')).toContainText('Première spec depuis le code');
      // Its first message, the preset's, sent to the spec writer, in its conversation.
      await expect(view.getByText(/n’a pas encore de spécification/).first()).toBeVisible({ timeout: 30_000 });
    });
  });
}

test('a brainstorm proposes pages and a decision, poured into the spec on the docs branch, and never changes the code', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  await page.setViewportSize({ width: 1440, height: 900 });
  const git = (...args: string[]) => execFileSync('git', args, { cwd: app.repo, encoding: 'utf8' });
  // The spec the project has (a page of Notifications) and its code.
  const notif = path.join(app.repo, '.escouade', 'spec', 'notifications', 'general.md');
  fs.mkdirSync(path.dirname(notif), { recursive: true });
  fs.writeFileSync(
    notif,
    '---\ntitle: "Notifications"\nsubject: "Notifications"\nsources: []\ntickets: []\ndecisions: []\ncompared: null\n---\n\nAucune notification n’est planifiée.\n',
  );
  fs.mkdirSync(path.join(app.repo, 'src'), { recursive: true });
  const code = path.join(app.repo, 'src', 'relances.ts');
  fs.writeFileSync(code, 'export const RELANCES = [30, 7];\n');
  git('add', '.');
  git('commit', '-qm', 'spec et code');
  const head = git('rev-parse', 'HEAD').trim();
  // A file of the user's to join.
  const cr = path.join(app.root, 'CR atelier du 22-10.md');
  fs.writeFileSync(cr, '# Atelier du 22/10\n\nRelancer les adhérents dont l’adhésion expire, par e-mail.\n');
  await addProject(page, app.repo, { firstAgent: false, name: 'Portail' });

  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  await tabs.getByRole('button', { name: 'Docs' }).click();
  await expect(page.getByRole('button', { name: /Nouvelle discussion/ })).toBeVisible();

  // « Nouvelle discussion » by its shortcut: two agents to choose from, a subject, a file joined.
  await page.keyboard.press('Control+b');
  const dialog = page.getByRole('dialog', { name: 'Nouvelle discussion' });
  await expect(dialog.getByRole('radio', { name: /Discussion/ })).toHaveAttribute('aria-checked', 'true');
  await dialog.getByRole('radio', { name: /Brainstorm/ }).click();
  await dialog.getByRole('textbox').first().fill('Relance des adhésions [proposer:Portail]');
  await dialog.getByTestId('discussion-files').setInputFiles(cr);
  await expect(dialog.getByText('CR atelier du 22-10.md')).toBeVisible();
  await expect(dialog.getByRole('button', { name: /^Harness : Claude Code/ })).toBeVisible();
  await expect(dialog.getByText('branche courante')).toBeVisible();
  await shoot(page, 'new');
  await dialog.getByRole('button', { name: 'Commencer' }).click();
  await expect(dialog).toBeHidden();

  // The discussion open; its card marked Brainstorm.
  const view = page.locator('.discussion[data-kind="brainstorm"]');
  await expect(view).toBeVisible();
  const card = page.locator('button.disc[data-kind="brainstorm"]');
  await expect(card).toContainText('Brainstorm');
  await expect(card).toContainText('Relance des adhésions');
  // The file joined: filed by itself in the documents of the docs branch.
  await expect
    .poll(() => {
      const docs = path.join(app.repo, '.escouade', 'documents');
      const all = fs.existsSync(docs) ? fs.readdirSync(docs, { recursive: true }).map(String) : [];
      return all.some((f) => f.endsWith('CR atelier du 22-10.md'));
    })
    .toBe(true);

  // What it proposes, in « Ce qui ira dans la spec »; nothing written in the spec yet.
  const panel = page.getByRole('complementary', { name: 'Ce qui ira dans la spec' });
  const relances = panel.locator('[data-page="espace-adherent/relances"]');
  await expect(relances).toContainText('Trois relances par e-mail', { timeout: 60_000 });
  await expect(relances).toContainText('nouvelle section');
  const changed = panel.locator('[data-page="notifications/general"]');
  await expect(changed.locator('.del')).toHaveText('Aucune notification n’est planifiée.');
  await expect(changed.locator('.add')).toHaveText('Les relances d’adhésion sont planifiées chaque nuit.');
  await expect(panel.locator('[data-decision="1"]')).toContainText('Relances par e-mail seulement');
  await expect(card).toContainText('3 propositions');
  const relancesFile = path.join(app.repo, '.escouade', 'spec', 'espace-adherent', 'relances.md');
  expect(fs.existsSync(relancesFile)).toBe(false);
  expect(fs.readFileSync(notif, 'utf8')).toContain('Aucune notification n’est planifiée.');
  expect(fs.existsSync(path.join(app.repo, '.escouade', 'decisions'))).toBe(false);
  await shoot(page, 'brainstorm');

  // A write of the code it asks for: refused by Escouade on the spot, nobody asked.
  const composer = view.getByRole('textbox').last();
  await composer.fill('modifier-fichier src/relances.ts');
  await composer.press('Enter');
  await expect(view.getByText('Edit : deny')).toBeVisible({ timeout: 30_000 });
  expect(fs.readFileSync(code, 'utf8')).toBe('export const RELANCES = [30, 7];\n');
  await expect(card).not.toContainText('Question');

  // « Verser dans la spec »: written in the project's folder (the current branch), the decision proposed in the register.
  await view.getByRole('button', { name: 'Verser dans la spec' }).click();
  await expect(panel.locator('[data-page]')).toHaveCount(0);
  await expect(card).toContainText('Versé');
  // Proposed, cited by its id: it has no number until accepted (ESC-303).
  await expect(card).toContainText('relances-par-e-mail-seulement');
  expect(fs.readFileSync(relancesFile, 'utf8')).toContain('Trois relances par e-mail');
  expect(fs.readFileSync(notif, 'utf8')).toContain('Les relances d’adhésion sont planifiées chaque nuit.');
  const decisions = fs.readdirSync(path.join(app.repo, '.escouade', 'decisions'));
  expect(decisions).toHaveLength(1);
  const decision = fs.readFileSync(path.join(app.repo, '.escouade', 'decisions', decisions[0]), 'utf8');
  expect(decision).toContain('state: "proposed"');
  expect(decision).toContain('origin: "brainstorm"');
  // Not shared: the status bar says so, nothing committed.
  await expect(page.locator('[data-docs-share]')).toContainText('à partager');
  expect(git('rev-parse', 'HEAD').trim()).toBe(head);
  await shoot(page, 'poured');

  // A discussion with the spec writer, from the spec: « Demander à la spec ».
  await page
    .getByRole('region', { name: 'Espaces des docs' })
    .getByRole('button', { name: /Spécification/ })
    .click();
  await page.getByRole('button', { name: 'Demander à la spec' }).click();
  await expect(dialog.getByRole('radio', { name: /Discussion/ })).toHaveAttribute('aria-checked', 'true');
  await dialog.getByRole('textbox').first().fill('Que dit la spec sur les relances ?');
  await dialog.getByRole('button', { name: 'Commencer' }).click();
  const writer = page.locator('.discussion[data-kind="discussion"]');
  await expect(writer).toBeVisible();
  await expect(writer.getByText('Avec l’agent rédacteur')).toBeVisible();
  await expect(page.locator('button.disc[data-kind="discussion"]')).toContainText('Discussion');
  await expect(page.locator('button.disc')).toHaveCount(2);
  await expect(writer.getByRole('button', { name: 'Verser dans la spec' })).toBeDisabled();
  await shoot(page, 'writer');
  expect(git('rev-parse', 'HEAD').trim()).toBe(head);
});
