// The other windows with a field that can be made taller (ESC-189): the settings of a project, the « Nouveau projet »
// assistant and the commit window. A field dragged far past the window by its corner never covers what comes under it:
// the window scrolls to it, with the wheel and with Tab, and its buttons stay shown, at the runner's 1024 × 728.

import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, openNewProject, test, openSettingsPage, closeSettings } from './fixture';

/** Whether a click at the middle of `el` lands on it: drawn, inside the part of its window that shows, not covered. */
const hit = (el: Locator) =>
  el.evaluate((node) => {
    const r = node.getBoundingClientRect();
    const at = document.elementFromPoint(r.x + r.width / 2, r.y + Math.min(r.height / 2, 12));
    return r.width > 0 && r.height > 0 && !!at && (at === node || node.contains(at));
  });

/** Whether `el` lies clear of `field`, neither over it nor under it. */
const clear = async (el: Locator, field: Locator) => {
  const [a, b] = await Promise.all([field.boundingBox(), el.boundingBox()]);
  return !!a && !!b && (b.y >= a.y + a.height - 1 || b.y + b.height <= a.y + 1 || b.x >= a.x + a.width - 1 || b.x + b.width <= a.x + 1);
};

/**
 * `field` made 2000 px tall, as its corner dragged down would: `next`, what comes under it, is reached with the wheel over
 * the window and with Tab from the field, clear of it; `buttons` stay shown all along.
 */
async function tallField(page: Page, dialog: Locator, field: Locator, next: Locator, buttons: Locator[], label: string) {
  await field.evaluate((ta) => (ta.style.height = '2000px'));
  expect((await field.boundingBox())!.height, label).toBeGreaterThan(1900);
  for (const b of buttons) expect(await hit(b), `${await b.textContent()} at once, ${label}`).toBe(true);
  const box = (await dialog.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  for (let i = 0; i < 40 && !(await hit(next)); i++) await page.mouse.wheel(0, 150);
  expect(await hit(next), `under the field, reached with the wheel, ${label}`).toBe(true);
  expect(await clear(next, field), `under the field, clear of it, ${label}`).toBe(true);
  for (const b of buttons) expect(await hit(b), `${await b.textContent()} beside it, ${label}`).toBe(true);
  await page.mouse.wheel(0, -20_000);
  // The wheel scrolls on the compositor, and `mouse.wheel` returns before the page has it. On a slow runner the scroll
  // landed after the focus and the Tab below, its delta added to where the focus had scrolled: back at the top, the
  // focused button out of sight (ESC-393). Up again first, the field's top in the window.
  await expect
    .poll(async () => (await field.boundingBox())!.y >= (await dialog.boundingBox())!.y, { message: `back up, ${label}` })
    .toBe(true);
  await field.focus();
  for (let i = 0; i < 10 && !(await next.evaluate((n) => n === document.activeElement)); i++) await page.keyboard.press('Tab');
  await expect(next, label).toBeFocused();
  expect(await hit(next), `under the field, seen at its focus, ${label}`).toBe(true);
  for (const b of buttons) expect(await hit(b), `${await b.textContent()} after Tab, ${label}`).toBe(true);
}

test.beforeEach(async ({ app }) => {
  await app.page.setViewportSize({ width: 1024, height: 728 });
});

test('the settings of a project: the files copied into the worktrees, dragged tall', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Git et worktrees');
  await tallField(
    page,
    dialog,
    dialog.getByRole('textbox', { name: 'Fichiers copiés dans les worktrees' }),
    dialog.getByRole('group', { name: 'Commit' }).getByRole('button').first(),
    // No foot (ESC-380): « Fermer », in the header, stays shown.
    [dialog.getByRole('button', { name: 'Fermer', exact: true })],
    'settings',
  );
});

test('the « Nouveau projet » assistant: the files copied into the worktrees, dragged tall', async ({ app }) => {
  const { page } = app;
  await openNewProject(page);
  const dialog = page.getByRole('dialog', { name: 'Nouveau projet' });
  await dialog.getByPlaceholder('C:\\chemin\\vers\\le\\projet').fill(app.repo);
  await expect(dialog.getByText(/Dépôt git détecté/)).toBeVisible();
  await dialog.getByRole('radio', { name: /^Partir de zéro/ }).click();
  await dialog.getByRole('button', { name: 'Suivant', exact: true }).click();
  await tallField(
    page,
    dialog,
    dialog.getByRole('textbox', { name: 'Fichiers copiés dans les worktrees' }),
    dialog.getByRole('button', { name: 'Remplir automatiquement' }),
    [dialog.getByRole('button', { name: 'Retour', exact: true }), dialog.getByRole('button', { name: 'Suivant', exact: true })],
    'assistant',
  );
});

test('the commit window: its message dragged tall', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true });
  // A direct commit, its message proposed, for the window to open.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Git et worktrees');
  await settings.getByRole('button', { name: 'Direct, avec un message proposé' }).click();
  await closeSettings(settings);
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0)).toBe(1);
  fs.writeFileSync(path.join(worktrees, fs.readdirSync(worktrees)[0], 'notes.md'), 'Des notes\n');
  await page.getByRole('button', { name: 'Conversation et fichiers côte à côte' }).click();
  const side = page.locator('aside.git-side.docked');
  await expect(side.locator('.file')).toHaveCount(1);
  await side.getByRole('button', { name: 'Commit…' }).click();
  const dialog = page.getByRole('dialog', { name: 'Commit' });
  await expect(dialog).toBeVisible();
  const message = dialog.getByRole('textbox', { name: 'Message' });
  await message.fill('docs: des notes');
  // The message is its last field: its bottom edge (the corner that drags it) is reached with the wheel, the buttons shown.
  const buttons = [dialog.getByRole('button', { name: 'Annuler', exact: true }), dialog.getByRole('button', { name: 'Commiter' })];
  await message.evaluate((ta) => (ta.style.height = '2000px'));
  for (const b of buttons) expect(await hit(b), `${await b.textContent()} at once`).toBe(true);
  const body = dialog.locator('.body');
  const shown = (await body.boundingBox())!;
  await page.mouse.move(shown.x + 20, shown.y + shown.height / 2);
  const bottom = async () => {
    const f = (await message.boundingBox())!;
    return f.y + f.height <= shown.y + shown.height;
  };
  for (let i = 0; i < 40 && !(await bottom()); i++) await page.mouse.wheel(0, 150);
  expect(await bottom(), 'the bottom edge of the message, reached with the wheel').toBe(true);
  for (const b of buttons) expect(await hit(b), `${await b.textContent()} beside it`).toBe(true);
  await expect(buttons[1]).toBeEnabled();
  await buttons[1].click();
  await expect(dialog).toBeHidden();
});
