// The ticket's form with a very long description (ESC-189): every field under it and the window's buttons stay in reach,
// with the mouse and with the keyboard, when the ticket is added and when it is edited, at the runner's 1024 × 728 and
// lower, in French and in English, and with a height forced on the description far past the window (as the field that
// grew with its text set). The captures of the window go to ESCOUADE_E2E_SHOTS when it is set, in both themes.

import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const LONG = Array.from(
  { length: 200 },
  (_, i) => `Ligne ${i + 1} : une phrase assez longue pour remplir la ligne du champ et obliger à défiler`,
).join('\n');

const WORDS = {
  fr: {
    plus: 'Nouveau ticket',
    autopilot: 'Pilote auto',
    title: 'Titre du ticket',
    criteria: "Critères d'acceptation",
    groups: ['Boucles max', 'Harness', 'Branche', 'Cible', 'Après', 'Compte', 'Modèle', 'Effort'],
    attach: 'Joindre un fichier',
    add: 'Ajouter',
    save: 'Enregistrer',
    cancel: 'Annuler',
    edit: (key: string) => `Modifier ${key}`,
  },
  en: {
    plus: 'New ticket',
    autopilot: 'Autopilot',
    title: 'Ticket title',
    criteria: 'Acceptance criteria',
    groups: ['Max loops', 'Harness', 'Branch', 'Target', 'After', 'Account', 'Model', 'Effort'],
    attach: 'Attach a file',
    add: 'Add',
    save: 'Save',
    cancel: 'Cancel',
    edit: (key: string) => `Edit ${key}`,
  },
};
type Words = (typeof WORDS)['fr'];

/** Whether a click at the middle of `el` lands on it: drawn, inside the part of the window's body that shows, not covered. */
const hit = (el: Locator) =>
  el.evaluate((node) => {
    const r = node.getBoundingClientRect();
    const at = document.elementFromPoint(r.x + r.width / 2, r.y + Math.min(r.height / 2, 12));
    return r.width > 0 && r.height > 0 && !!at && (at === node || node.contains(at));
  });

/**
 * Whether `el` lies clear of the description: drawn over it, a field would still take a click, but over lines of the
 * description, which shows through or under it (what the user saw).
 */
const clear = async (el: Locator, description: Locator) => {
  const [a, b] = await Promise.all([description.boundingBox(), el.boundingBox()]);
  return !!a && !!b && (b.y >= a.y + a.height - 1 || b.y + b.height <= a.y + 1 || b.x >= a.x + a.width - 1 || b.x + b.width <= a.x + 1);
};

/** What the dialog must let the user reach under the description, in the order of its fields. */
async function below(dialog: Locator, w: Words) {
  const out: [string, Locator][] = [[w.attach, dialog.getByRole('button', { name: w.attach, exact: true })]];
  out.push([w.criteria, dialog.getByRole('textbox', { name: w.criteria })]);
  for (const name of w.groups) {
    const group = dialog.getByRole('group', { name, exact: true });
    // « Harness » only with several harnesses installed, « Après » only with another ticket to wait for.
    if (!(await group.count())) continue;
    out.push([name, group.getByRole('button').first()]);
  }
  return out;
}

/**
 * Every field under the description, reached with the mouse (the window's body brought to it, then a click that lands on
 * it), then with the keyboard (Tab from the description, each field seen where it gets the focus); the buttons of the
 * window always shown, without scrolling.
 */
async function reachAll(page: Page, dialog: Locator, w: Words, primary: string, label: string) {
  const buttons = [dialog.getByRole('button', { name: w.cancel, exact: true }), dialog.getByRole('button', { name: primary, exact: true })];
  const body = dialog.locator('.body');
  const fields = await below(dialog, w);
  const description = dialog.getByRole('textbox', { name: 'Description' });
  expect(fields.length, label).toBeGreaterThanOrEqual(7);
  for (const b of buttons) expect(await hit(b), `${await b.textContent()} at once, ${label}`).toBe(true);
  // With the mouse: the wheel over the window's body, then a click.
  for (const [name, el] of fields) {
    await body.evaluate((b) => (b.scrollTop = 0));
    const box = (await body.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height - 20);
    for (let i = 0; i < 80 && !(await hit(el)); i++) await page.mouse.wheel(0, 120);
    expect(await hit(el), `${name} reached with the wheel, ${label}`).toBe(true);
    expect(await clear(el, description), `${name} clear of the description, ${label}`).toBe(true);
    for (const b of buttons) expect(await hit(b), `${await b.textContent()} beside ${name}, ${label}`).toBe(true);
  }
  // With the keyboard: Tab from the description goes through every field, each one seen when it takes the focus.
  await body.evaluate((b) => (b.scrollTop = 0));
  await description.focus();
  // A menu greyed out (« Effort » of a model that takes none) is no stop of Tab.
  const enabled: string[] = [];
  for (const [name, el] of fields) if (await el.isEnabled()) enabled.push(name);
  const seen = new Set<string>();
  for (let i = 0; i < 40 && seen.size < enabled.length; i++) {
    await page.keyboard.press('Tab');
    for (const [name, el] of fields) {
      if (seen.has(name) || !(await el.evaluate((n) => n === document.activeElement))) continue;
      expect(await hit(el), `${name} seen at its focus, ${label}`).toBe(true);
      expect(await clear(el, description), `${name} clear of the description at its focus, ${label}`).toBe(true);
      seen.add(name);
    }
  }
  expect([...seen].sort(), `fields reached with Tab, ${label}`).toEqual(enabled.sort());
  for (const b of buttons) expect(await hit(b), `${await b.textContent()} after Tab, ${label}`).toBe(true);
}

/** The board of a project, the autopilot off (the tickets stay « À faire », to be edited), with a first ticket to wait for. */
async function board(page: Page, repo: string, w: Words, lang: 'fr' | 'en') {
  await addProject(page, repo, { firstAgent: false, lang });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('switch', { name: w.autopilot }).click();
  await page.getByRole('button', { name: w.plus }).click();
  await page.getByRole('textbox', { name: w.title }).fill('Le premier');
  await page.keyboard.press('Control+Enter');
  await expect(page.getByRole('button', { name: /DEM-1/ })).toBeVisible();
}

/** Its window, risen (its rise scales it a little). */
async function risen(dialog: Locator) {
  await expect(dialog).toBeVisible();
  await dialog.evaluate((e) => Promise.all(e.getAnimations().map((a) => a.finished)));
}

const SIZES = [
  { width: 1024, height: 728 },
  { width: 1024, height: 600 },
  { width: 900, height: 520 },
];

test('a ticket with a very long description: every field and the buttons stay in reach, added and edited', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  const w = WORDS.fr;
  await page.setViewportSize(SIZES[0]);
  await board(page, app.repo, w, 'fr');
  await page.getByRole('button', { name: w.plus }).click();
  const dialog = page.getByRole('dialog', { name: w.plus });
  await risen(dialog);
  await dialog.getByRole('textbox', { name: w.title }).fill('Un ticket très long');
  // Pasted, as the user did.
  const description = dialog.getByRole('textbox', { name: 'Description' });
  await description.focus();
  await page.keyboard.insertText(LONG);
  await expect(description).toHaveValue(LONG);
  for (const size of SIZES) {
    await page.setViewportSize(size);
    await reachAll(page, dialog, w, w.add, `new ticket, ${size.width}×${size.height}`);
  }
  await page.setViewportSize(SIZES[0]);
  if (SHOTS) {
    fs.mkdirSync(SHOTS, { recursive: true });
    for (const theme of ['dark', 'light'] as const) {
      await page.evaluate((t) => {
        document.documentElement.dataset.theme = t;
        document.documentElement.style.colorScheme = t;
      }, theme);
      await dialog.locator('.body').evaluate((b) => (b.scrollTop = 0));
      await page.screenshot({ path: path.join(SHOTS, `esc189-new-top-${theme}.png`) });
      await dialog.locator('.body').evaluate((b) => (b.scrollTop = b.scrollHeight));
      await page.screenshot({ path: path.join(SHOTS, `esc189-new-bottom-${theme}.png`) });
    }
    await page.evaluate(() => {
      document.documentElement.dataset.theme = 'dark';
      document.documentElement.style.colorScheme = 'dark';
    });
  }
  // A criterion typed in the field under it, and the ticket added by its button.
  const criteria = dialog.getByRole('textbox', { name: w.criteria });
  await criteria.click();
  await criteria.fill('Tout reste atteignable');
  await dialog.getByRole('button', { name: w.add, exact: true }).click();
  await expect(dialog).toBeHidden();
  await page.getByRole('button', { name: /DEM-2/ }).click();
  const edit = page.getByRole('dialog', { name: w.edit('DEM-2') });
  await risen(edit);
  await expect(edit.getByRole('textbox', { name: 'Description' })).toHaveValue(LONG);
  await expect(edit.getByRole('textbox', { name: w.criteria })).toHaveValue('Tout reste atteignable');
  for (const size of SIZES) {
    await page.setViewportSize(size);
    await reachAll(page, edit, w, w.save, `edited ticket, ${size.width}×${size.height}`);
  }
  await page.setViewportSize(SIZES[0]);
  // A height forced on the description far past the window, as the field that grew with its text set: still everything
  // in reach.
  await edit.getByRole('textbox', { name: 'Description' }).evaluate((ta) => {
    ta.style.height = '3000px';
    ta.style.minHeight = '3000px';
  });
  await reachAll(page, edit, w, w.save, 'description forced to 3000 px');
  const field = edit.getByRole('textbox', { name: w.criteria });
  await field.click();
  await field.fill('Tout reste atteignable\nMême en modification');
  await edit.getByRole('button', { name: w.save, exact: true }).click();
  await expect(edit).toBeHidden();
  await page.getByRole('button', { name: /DEM-2/ }).click();
  const again = page.getByRole('dialog', { name: w.edit('DEM-2') });
  await expect(again.getByRole('textbox', { name: w.criteria })).toHaveValue('Tout reste atteignable\nMême en modification');
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('a ticket with a very long description: every field and the buttons stay in reach', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    const w = WORDS.en;
    await page.setViewportSize(SIZES[0]);
    await board(page, app.repo, w, 'en');
    await page.getByRole('button', { name: w.plus }).click();
    const dialog = page.getByRole('dialog', { name: w.plus });
    await risen(dialog);
    await dialog.getByRole('textbox', { name: w.title }).fill('A very long ticket');
    await dialog.getByRole('textbox', { name: 'Description' }).focus();
    await page.keyboard.insertText(LONG);
    await reachAll(page, dialog, w, w.add, 'English, 1024×728');
    await dialog.getByRole('button', { name: w.add, exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(page.getByRole('button', { name: /DEM-2/ })).toBeVisible();
  });
});
