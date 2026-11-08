import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';

// « Pause du pilote auto au-delà du quota » from its page of the 2.2 settings (R4): « Comptes et harness › Quand un quota
// est épuisé », beside « Changer de compte à », no longer in a project's Kanban. Its key is the one it always had
// (`quotaPause` of settings.json). Principal alone, its fake `claude` saying 85 % of its 5-hour window at each turn.

/** A sign-in that is not out of date (the usage endpoint is a closed port: the quota is the one the turns tell). */
const SIGNED_IN = JSON.stringify({ claudeAiOauth: { accessToken: 'tok', refreshToken: 'ref', expiresAt: 4_102_444_800_000 } });

test.use({
  preStart: {
    run: (root: string) => {
      const dir = path.join(root, 'claude');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, '.credentials.json'), SIGNED_IN);
      fs.writeFileSync(path.join(dir, 'fake-pct'), '85');
      return {};
    },
  },
});

async function setPause(page: Page, label: string) {
  const dialog = await openSettingsPage(page, 'Comptes et harness');
  const pause = dialog.getByRole('group', { name: 'Pause du pilote auto au-delà du quota' });
  // Beside « Changer de compte à », with what it holds back said in a few words.
  await expect(dialog.getByRole('group', { name: 'Changer de compte à' })).toBeVisible();
  await expect(dialog.getByText('pour tous les projets')).toBeVisible();
  await pause.getByRole('button', { name: label }).click();
  await expect(pause.getByRole('button', { name: label })).toHaveAttribute('aria-pressed', 'true');
  await closeSettings(dialog);
}

test('pauses the autopilot past the threshold set in « Comptes et harness », and starts it again under a higher one', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  const saved = () => JSON.parse(fs.readFileSync(path.join(app.root, 'data', 'settings.json'), 'utf8'));
  await addProject(page, app.repo, { firstAgent: false });

  // Nothing of it in the project's Kanban any more.
  const board = await openSettingsPage(page, 'Kanban');
  await expect(board.getByRole('heading', { name: 'Pilote auto' })).toBeVisible();
  await expect(board.getByRole('group', { name: /au-delà du quota/ })).toHaveCount(0);
  await closeSettings(board);

  await page.getByRole('button', { name: /^Kanban/ }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  const todo = page.getByRole('region', { name: 'À faire' });
  const add = async (title: string) => {
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    await page.getByRole('textbox', { name: 'Titre du ticket' }).fill(title);
    await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  };

  // At 100 % (the default), Principal's 85 % holds nothing back: the first ticket runs, and its turn tells the quota.
  await add('Premier [ok]');
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });

  // 80 %: the autopilot pauses, the next ticket waits in « À faire », and the band says why (one account: no list of
  // the accounts past the threshold).
  await setPause(page, '80 %');
  await expect.poll(() => saved().quotaPause).toBe(80);
  await add('Deuxième [ok]');
  await expect(page.getByRole('status').filter({ hasText: /Pilote auto en pause\s:\squota de 5\sh à 85\s%/ })).toBeVisible({
    timeout: 30_000,
  });
  await expect(todo.getByRole('button', { name: /DEM-2/ }).getByText('En attente : pilote auto en pause')).toBeVisible();
  await page.waitForTimeout(3000);
  await expect(todo.getByText('DEM-2', { exact: true })).toBeVisible();

  // 90 %: above Principal's 85 %, the autopilot starts it.
  await setPause(page, '90 %');
  await expect.poll(() => saved().quotaPause).toBe(90);
  await expect(review.getByText('DEM-2', { exact: true })).toBeVisible({ timeout: 60_000 });
});
