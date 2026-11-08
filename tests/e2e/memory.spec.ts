// The shared memory (ESC-255) on a fake memory of Claude Code: what Principal's Claude Code (the test's folder, never the
// user's) remembers of the project is offered once when the project is added, memory by memory and none ticked; only
// what is ticked goes to `.escouade/memory/` of the project's folder, Claude Code's files untouched; the offer does not
// come back after a restart; Réglages › Projets › Équipe lists the memory and forgets one. `ESCOUADE_E2E_SHOTS`: screenshots in
// both themes.

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** The theme of the app, set as « Thème » would (its effect without its dialog). */
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
    await page.mouse.move(0, 0);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

/** Claude Code's folder of the project's memory, as it names it: the path with every character but a letter or a digit a dash. */
const claudeMemory = (root: string, repo: string) => path.join(root, 'claude', 'projects', repo.replace(/[^a-zA-Z0-9]/g, '-'), 'memory');

test('Claude Code’s memory is offered once, taken memory by memory, then listed and forgotten in the settings', async ({ app }) => {
  test.setTimeout(180_000);
  let { page } = app;
  await page.setViewportSize({ width: 1280, height: 800 });
  const claude = claudeMemory(app.root, app.repo);
  fs.mkdirSync(claude, { recursive: true });
  const files: Record<string, string> = {
    'MEMORY.md': '- [e2e-apres-build](e2e-apres-build.md) — Lancer les e2e après un build de debug\n',
    'e2e-apres-build.md':
      '---\nname: e2e-apres-build\ndescription: Lancer les e2e après un build de debug\nmetadata:\n  type: feedback\n---\n\n' +
      '`npx tauri build --debug --no-bundle` avant les e2e : sinon l’app testée est l’ancienne.\n\n' +
      '**Why:** les e2e lancent le binaire de debug.\n',
    'tests-instables.md':
      '---\nname: tests-instables\ndescription: Relancer seul un test Rust qui échoue sous charge\ntype: project\n---\n\nnextest -j 32 les fait échouer.\n',
    'preferences.md': '---\nname: preferences\ndescription: L’utilisateur veut des messages courts\ntype: user\n---\n\nCourt.\n',
  };
  for (const [name, text] of Object.entries(files)) fs.writeFileSync(path.join(claude, name), text);

  await addProject(page, app.repo, { firstAgent: false });
  const offer = page.getByRole('dialog', { name: 'Reprendre la mémoire de Claude Code ?' });
  await expect(offer).toBeVisible();
  // Each memory, none ticked: nothing goes without a tick.
  const boxes = offer.getByRole('checkbox');
  await expect(boxes).toHaveCount(3);
  for (const b of await boxes.all()) await expect(b).not.toBeChecked();
  await expect(offer.getByRole('button', { name: 'Reprendre', exact: true })).toBeDisabled();
  await offer.getByRole('checkbox', { name: /e2e-apres-build/ }).check();
  await offer.getByRole('checkbox', { name: /tests-instables/ }).check();
  await offer.getByText('Voir le texte').first().click();
  await shots(page, 'reprise');
  await offer.getByRole('button', { name: 'Reprendre 2 souvenirs' }).click();
  await expect(offer).toBeHidden();
  await expect(page.getByText('2 souvenirs repris dans la mémoire partagée.')).toBeVisible();

  // In the project's folder, as Escouade writes it; the one not ticked left out; Claude Code's files as they were.
  const own = path.join(app.repo, '.escouade', 'memory');
  expect(fs.readdirSync(own).sort()).toEqual(['MEMORY.md', 'e2e-apres-build.md', 'tests-instables.md']);
  expect(fs.readFileSync(path.join(own, 'e2e-apres-build.md'), 'utf8')).toContain(
    'name: e2e-apres-build\ndescription: Lancer les e2e après un build de debug\nmetadata:\n  type: feedback\n---',
  );
  expect(fs.readFileSync(path.join(own, 'tests-instables.md'), 'utf8')).toContain('  type: project');
  for (const [name, text] of Object.entries(files)) expect(fs.readFileSync(path.join(claude, name), 'utf8')).toBe(text);

  // Once: not offered again after a restart.
  page = await app.restart();
  await page.setViewportSize({ width: 1280, height: 800 });
  await expect(page.getByRole('button', { name: 'Réglages (Ctrl+,)' })).toBeVisible();
  // The offer, made at once on a project not asked yet (as at the add above), would be there by now.
  await page.waitForTimeout(1500);
  await expect(page.getByRole('dialog', { name: 'Reprendre la mémoire de Claude Code ?' })).toHaveCount(0);

  // Réglages › Projets › Équipe: the list, a text read, one forgotten on a second click.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Équipe');
  const list = dialog.getByRole('list', { name: 'Mémoire partagée' });
  await list.scrollIntoViewIfNeeded();
  await expect(list.getByRole('listitem')).toHaveCount(2);
  await expect(list.getByText('Relancer seul un test Rust qui échoue sous charge')).toBeVisible();
  await list.getByText('e2e-apres-build').click();
  await expect(list.getByText(/npx tauri build --debug --no-bundle/)).toBeVisible();
  await shots(page, 'reglages');
  await list.getByRole('button', { name: 'Oublier le souvenir tests-instables' }).click();
  await list.getByRole('button', { name: 'Confirmer l’oubli' }).click();
  await expect(list.getByRole('listitem')).toHaveCount(1);
  expect(fs.existsSync(path.join(own, 'tests-instables.md'))).toBe(false);
  expect(fs.readFileSync(path.join(own, 'MEMORY.md'), 'utf8')).not.toContain('tests-instables');
});
