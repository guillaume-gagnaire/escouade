// The phone's Kanban (MO-7, docs/MOBILE.md § 3.7), at 360 × 780 and 430 × 932, against an `escouade-server` of the
// test's whose tickets the fake `claude` works on: the project with its space, the column in segments, its tickets;
// « Valider » by the swipe then the system's lock (the merge done on the server), « Renvoyer » with a word by the
// buttons of a line touched (the swipe's accessible way; its agent gets the word), « Lancer » a ticket to do under the
// lock. In the dark theme, then in the light one.

import fs from 'node:fs';
import path from 'node:path';
import { expect, test as base, type Page, type TestInfo } from '@playwright/test';
import { kanbanProject } from '../scripts/local-space.mjs';
import { ROOT, startSpace, type TestSpace } from './harness';

const sizeOf = (info: TestInfo) => (info.project.name.endsWith('360') ? '360' : '430');

const test = base.extend<{ space: TestSpace }>({
  space: async ({}, use, info) => {
    const space = await startSpace();
    await use(space);
    // A failure says what the server said.
    if (info.status !== info.expectedStatus) {
      // Whether it still answers on its own port (the forward aside).
      const alive = await Promise.race([
        space.call('space_glance', {}).then(
          () => 'answers',
          (e) => `fails: ${String(e)}`,
        ),
        new Promise((r) => setTimeout(() => r('no answer in 5 s'), 5000)),
      ]);
      fs.writeFileSync(
        info.outputPath('escouade-server.log'),
        `${space.server.log()}
--- after the failure: ${alive}
`,
      );
    }
    await space.stop();
  },
});

interface Line {
  id: string;
  key: string;
  column: string;
  iteration: number;
}
interface Board {
  counts: Record<string, number>;
  tickets: Line[];
}

/** With ESCOUADE_MOBILE_SHOTS=1, the screen in design/mobile-2-3/app/ (`<name>[-clair]-<size>.png`). */
async function shot(page: Page, info: TestInfo, name: string) {
  if (!process.env.ESCOUADE_MOBILE_SHOTS) return;
  const theme = await page.evaluate(() => document.documentElement.dataset.theme);
  const dir = path.join(ROOT, 'design', 'mobile-2-3', 'app');
  fs.mkdirSync(dir, { recursive: true });
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({
    path: path.join(dir, `${name}${theme === 'light' ? '-clair' : ''}-${sizeOf(info)}.png`),
    animations: 'disabled',
  });
}

/** The project of `kanbanProject` (two tickets « À tester », one « À faire »), and how to read its board. */
async function project(space: TestSpace) {
  const made = await kanbanProject(space.call, space.root).catch((e: Error) => {
    // What the server said, for the reason.
    throw new Error(`${e.message}
--- escouade-server ---
${space.server.log().slice(-6000)}`);
  });
  const { project: p, tickets } = made as { project: { id: string }; tickets: Line[] };
  const [avoirs, fec, relances] = tickets;
  const board = () => space.call<Board>('board_glance', { projectId: p.id });
  const columnOf = async (id: string) => (await board()).tickets.find((x) => x.id === id)?.column;
  return { avoirs, fec, relances, board, columnOf };
}

test('the Kanban of a project on the server: validate by the swipe, send back with a word, start, in both themes', async ({
  page,
  space,
}, info) => {
  test.setTimeout(180_000);
  const { avoirs, fec, relances, columnOf } = await project(space);
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.addInitScript(
    (given) => ((window as unknown as { __ESCOUADE_SPACES__: unknown }).__ESCOUADE_SPACES__ = given),
    [{ id: 'travail', name: 'Travail', url: space.url, token: space.token }],
  );
  await page.goto('/');
  await page.getByRole('tablist').getByRole('tab', { name: 'Kanban' }).click();

  // The project with its space, the columns in segments, « À tester » first.
  await expect(page.getByRole('button', { name: 'facturation, sur Travail : changer de projet' })).toBeVisible();
  const segments = page.getByRole('radiogroup', { name: 'Les colonnes' });
  await expect(segments.getByRole('radio', { name: 'À tester, 2 tickets' })).toHaveAttribute('aria-checked', 'true');
  await expect(segments.getByRole('radio', { name: 'À faire, 1 ticket' })).toBeVisible();
  const first = page.getByRole('button', { name: `${avoirs.key} Avoirs partiels sur une facture payée [ok]` });
  await expect(first).toContainText('boucle 1/5');
  await expect(page.getByRole('img', { name: '1 critère rempli sur 1' }).first()).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize()!.width);
  await shot(page, info, 'kanban');

  // « Valider » by the swipe: the line slides to the left and uncovers « Renvoyer » and « Valider ».
  const line = page.locator(`[data-ticket="${avoirs.key}"]`);
  const box = (await line.locator('.front').boundingBox())!;
  const y = box.y + box.height / 2;
  await page.mouse.move(box.x + box.width - 30, y);
  await page.mouse.down();
  for (const dx of [20, 60, 120, 180, 230]) await page.mouse.move(box.x + box.width - 30 - dx, y);
  await page.mouse.up();
  const approve = line.locator('.side').getByRole('button', { name: 'Valider' });
  await expect(approve).toBeVisible();
  await expect(line.locator('.side').getByRole('button', { name: 'Renvoyer' })).toBeVisible();
  // The uncovered gestures stay inside the screen.
  const side = (await line.locator('.side').boundingBox())!;
  expect(side.x + side.width).toBeLessThanOrEqual(page.viewportSize()!.width + 0.5);
  await shot(page, info, 'kanban-glisse');
  await approve.click();
  // Under the system's lock, which names the ticket and the space; then merged on the server.
  await expect
    .poll(() => page.evaluate(() => (window as unknown as { __ESCOUADE_UNLOCKS__?: string[] }).__ESCOUADE_UNLOCKS__ ?? []))
    .toEqual([`Valider ${avoirs.key} ? · Escouade · Travail`]);
  await expect.poll(() => columnOf(avoirs.id), { timeout: 60_000 }).toBe('done');
  await expect(segments.getByRole('radio', { name: 'À tester, 1 ticket' })).toBeVisible();
  await expect(segments.getByRole('radio', { name: 'Terminé, 1 ticket' })).toBeVisible();

  // The light theme, followed as the system changes it.
  await page.emulateMedia({ colorScheme: 'light' });
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe('light');

  // « Renvoyer » with a word, by the buttons of the line touched (where the swipe is impossible): no lock.
  const second = page.getByRole('button', { name: `${fec.key} Export comptable au format FEC [ok]` });
  await second.click();
  await expect(second).toHaveAttribute('aria-expanded', 'true');
  const gestures = page.getByRole('group', { name: `Les gestes de ${fec.key}` });
  await gestures.getByRole('button', { name: 'Renvoyer' }).click();
  const why = page.getByRole('textbox', { name: 'Ce qu’il faut changer' });
  await expect(why).toBeFocused();
  await why.fill('Ajoute le cas des avoirs sans TVA');
  await shot(page, info, 'kanban-renvoyer');
  await page.getByRole('button', { name: 'Renvoyer à l’agent' }).click();
  // Its agent gets the word (its ticket starts again, at loop 1).
  const stdin = path.join(space.root, 'fake-claude.stdin.jsonl');
  await expect
    .poll(() => (fs.existsSync(stdin) ? fs.readFileSync(stdin, 'utf8') : ''), { timeout: 60_000 })
    .toContain('Ajoute le cas des avoirs sans TVA');
  expect(await page.evaluate(() => (window as unknown as { __ESCOUADE_UNLOCKS__?: string[] }).__ESCOUADE_UNLOCKS__?.length)).toBe(1);

  // « Lancer » a ticket to do, under the lock.
  await segments.getByRole('radio', { name: 'À faire, 1 ticket' }).click();
  await shot(page, info, 'kanban-a-faire');
  const third = page.getByRole('button', { name: `${relances.key} Relances automatiques à J+15 [ok]` });
  await third.click();
  await page
    .getByRole('group', { name: `Les gestes de ${relances.key}` })
    .getByRole('button', { name: 'Lancer' })
    .click();
  await expect
    .poll(() => page.evaluate(() => (window as unknown as { __ESCOUADE_UNLOCKS__?: string[] }).__ESCOUADE_UNLOCKS__?.at(-1)))
    .toBe(`Lancer ${relances.key} ? · Escouade · Travail`);
  await expect.poll(() => columnOf(relances.id), { timeout: 60_000 }).not.toBe('todo');
  await expect(segments.getByRole('radio', { name: 'À faire, 0 ticket' })).toBeVisible();
});

test('a lock cancelled starts nothing, and the project is chosen in its sheet', async ({ page, space }, info) => {
  const { relances, columnOf } = await project(space);
  await page.emulateMedia({ colorScheme: 'light' });
  await page.addInitScript(
    (given) => {
      const w = window as unknown as { __ESCOUADE_SPACES__: unknown; __ESCOUADE_LOCK__: string };
      w.__ESCOUADE_SPACES__ = given;
      w.__ESCOUADE_LOCK__ = 'cancel';
    },
    [{ id: 'travail', name: 'Travail', url: space.url, token: space.token }],
  );
  await page.goto('/');
  await page.getByRole('tablist').getByRole('tab', { name: 'Kanban' }).click();
  await page.getByRole('button', { name: 'facturation, sur Travail : changer de projet' }).click();
  const sheet = page.getByRole('dialog', { name: 'Les projets' });
  await expect(sheet.getByRole('button', { name: 'facturation' })).toHaveAttribute('aria-current', 'true');
  await shot(page, info, 'kanban-projets');
  await sheet.getByRole('button', { name: 'facturation' }).click();
  await expect(sheet).toBeHidden();
  await page.getByRole('radio', { name: 'À faire, 1 ticket' }).click();
  await page.getByRole('button', { name: new RegExp(`^${relances.key} `) }).click();
  await page
    .getByRole('group', { name: `Les gestes de ${relances.key}` })
    .getByRole('button', { name: 'Lancer' })
    .click();
  await expect
    .poll(() => page.evaluate(() => (window as unknown as { __ESCOUADE_UNLOCKS__?: string[] }).__ESCOUADE_UNLOCKS__?.length ?? 0))
    .toBe(1);
  // Nothing went: the ticket waits in « À faire ».
  await page.waitForTimeout(1500);
  expect(await columnOf(relances.id)).toBe('todo');
  await expect(page.getByRole('radio', { name: 'À faire, 1 ticket' })).toBeVisible();
});
