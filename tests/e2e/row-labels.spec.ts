// The label of a setting is never squeezed (ESC-246): beside a wide control (the four actions of « Quand je valide un
// ticket « À tester » », the target branches), it keeps a readable width, and the control goes under it when both do
// not fit, in the assistant « Nouveau projet » (no `settings` container there) as in the settings. Every step of the
// assistant and every tab of the settings, at the CI's window (1024 × 728), 1100 × 800 and 1440 × 900, in both themes.
// The captures go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, openNewProject, openSettingsPage, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1024, height: 728 },
  { width: 1100, height: 800 },
  { width: 1440, height: 900 },
];
const THEMES = ['dark', 'light'] as const;
/** Under this, a label is squeezed: about « Supprimer le worktree » on one line. */
const READABLE = 150;

/** The labels of `.sr` rows narrower than their longest word, or than READABLE when they would fit in it. */
const squeezed = (root: Locator) =>
  root.evaluate((r, readable) => {
    const out: string[] = [];
    for (const l of r.querySelectorAll<HTMLElement>('.sr .lab > .l')) {
      if (!l.offsetParent) continue;
      const text = l.textContent?.replace(/\s+/g, ' ').trim() ?? '';
      // Its width on one line, and its longest word's, measured in the same font.
      const probe = document.createElement('span');
      probe.style.cssText = 'position:absolute;visibility:hidden;white-space:nowrap;font:inherit';
      l.append(probe);
      const width = (s: string) => {
        probe.textContent = s;
        return probe.getBoundingClientRect().width;
      };
      const whole = width(text);
      const word = Math.max(...text.split(' ').map(width));
      probe.remove();
      const w = l.getBoundingClientRect().width;
      if (w + 1 < word || w + 1 < Math.min(whole, readable)) out.push(`« ${text} » ${Math.round(w)} px`);
    }
    return out;
  }, READABLE);

/** What reaches out of the window or of its row. */
const overflows = (root: Locator) =>
  root.evaluate((r) => {
    const out: string[] = [];
    for (const row of r.querySelectorAll<HTMLElement>('.sr')) {
      if (!row.offsetParent) continue;
      const box = row.getBoundingClientRect();
      for (const el of row.querySelectorAll<HTMLElement>('.ctl > *, .lab > *')) {
        const a = el.getBoundingClientRect();
        if (a.width && (a.left < box.left - 1 || a.right > box.right + 1))
          out.push(`« ${el.textContent?.trim().slice(0, 40)} » out of « ${row.querySelector('.l')?.textContent?.trim()} »`);
      }
    }
    return out;
  });

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

const frames = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));

/** For each size and theme: the rows of `root` read, and captured as `<what>-<width>[-light]`. */
async function review(page: Page, root: Locator, what: string, problems: string[]) {
  for (const size of SIZES)
    for (const t of THEMES) {
      await page.setViewportSize(size);
      await theme(page, t);
      await frames(page);
      for (const p of [...(await squeezed(root)), ...(await overflows(root))]) problems.push(`${what}, ${size.width}: ${p}`);
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `${what}-${size.width}${t === 'light' ? '-light' : ''}.png`) });
      }
    }
}

function billingService(root: string): string {
  const dir = path.join(root, 'billing-service');
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(dir, 'README.md'), '# billing-service\n');
  git('add', '-A');
  git('commit', '-qm', 'init');
  // A second branch with a long name, as the capture of the ticket had.
  git('branch', 'chore/mco-tool-vendredinnovation');
  return dir;
}

test('the labels of the assistant keep their width at every step, from 1024 × 728 to 1440 × 900', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await openNewProject(page);
  const dialog = page.getByRole('dialog', { name: 'Nouveau projet' });
  await dialog.getByPlaceholder('C:\\chemin\\vers\\le\\projet').fill(billingService(app.root));
  await expect(dialog.getByText(/Dépôt git détecté/)).toBeVisible();
  await page.mouse.move(2, 2);
  const problems: string[] = [];
  await dialog.getByRole('radio', { name: /^Partir de zéro/ }).click();
  await review(page, dialog, 'wizard-1-start', problems);
  const steps = ['worktrees', 'launch', 'kanban', 'account', 'integrations', 'summary'];
  for (const [i, step] of steps.entries()) {
    await page.setViewportSize(SIZES[2]);
    await dialog.getByRole('button', { name: 'Suivant', exact: true }).click();
    await expect(dialog.locator(`.page[data-step="${step}"]`)).toBeVisible();
    if (step === 'launch') await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
    await review(page, dialog, `wizard-${i + 2}-${step}`, problems);
  }
  expect(problems).toEqual([]);
});

test('the labels of the settings keep their width on every page, from 1024 × 728 to 1440 × 900', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const win = page.getByRole('dialog', { name: 'Réglages' });
  await page.mouse.move(2, 2);
  const problems: string[] = [];
  // Each page of the bar, the project's unfolded first; « À propos » by its name, not its version.
  await openSettingsPage(win, 'Projet');
  const pages = await win.getByRole('navigation').locator('button:not([aria-expanded]) .lbl').allTextContents();
  expect(pages.length).toBeGreaterThan(14);
  for (const name of pages.map((s) => s.trim())) {
    await openSettingsPage(win, name);
    await review(page, win, `settings-${name.toLowerCase().replace(/\W+/g, '-')}`, problems);
  }
  expect(problems).toEqual([]);
});
