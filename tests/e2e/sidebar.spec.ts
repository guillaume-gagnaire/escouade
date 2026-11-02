// The agents' sidebar of the 2.0 design (ESC-73, lot 3): Agents | Kanban, « Nouvel agent » and its menu « Harness », the
// cards, Lancement and Terminaux, the folder and the branch, as the preview draws them (design/app-redesign/index.html),
// read in both themes at 1440 × 900 and at CI's 1028 × 779; a question answered from the list; the English labels in
// their room; the motion stopped by prefers-reduced-motion. The captures of the app and of the preview go to
// ESCOUADE_E2E_SHOTS when it is set, to be looked at side by side.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const PREVIEW = path.join(ROOT, 'design', 'app-redesign', 'index.html');
/** The preview loads Inter from the site's packages: without them, its text is not the app's. */
const PREVIEW_FONT = path.join(ROOT, 'website', 'node_modules', '@fontsource-variable', 'inter', 'opsz.css');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900, name: '' },
  { width: 1028, height: 779, name: '-small' },
];
/** The colour both are tinted with: the preview's first project's, the app's first project's. */
const TINT = 'oklch(0.72 0.12 48)';

/** The look of an element, as the browser computes it: what the app and the preview must share. */
function look(el: Locator) {
  return el.evaluate((node) => {
    const s = getComputedStyle(node);
    const r = node.getBoundingClientRect();
    return {
      width: Math.round(r.width),
      height: Math.round(r.height),
      background: s.backgroundColor,
      color: s.color,
      shadow: s.boxShadow,
      radius: s.borderRadius,
      padding: s.padding,
      font: `${s.fontWeight} ${s.fontSize} ${s.fontFamily.split(',')[0]}`,
    };
  });
}

/** Its look without some of its keys: what differs by content (a width, a height). */
const lookBut = (el: Locator, ...keys: string[]) =>
  look(el).then((l) => Object.fromEntries(Object.entries(l).filter(([k]) => !keys.includes(k))));

/** Where an element is, rounded. */
const place = (el: Locator) =>
  el.evaluate((node) => {
    const r = node.getBoundingClientRect();
    return [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)];
  });

/** Tints a page with `TINT` at once, nothing gliding nor breathing, for both to be read alike. */
const still = (page: Page) =>
  page.evaluate((p) => {
    if (!document.getElementById('still')) {
      const style = document.createElement('style');
      style.id = 'still';
      style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
      document.head.append(style);
    }
    document.documentElement.style.setProperty('--p', p);
  }, TINT);

/** The theme of the app, set as « Thème » would (its effect without its dialog). */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

test('the sidebar looks as the preview draws it, in both themes, at both sizes', async ({ app }) => {
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  await expect(side.locator('.agent')).toHaveCount(1);
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}`);
        await preview.evaluate(() => document.fonts.ready);
        await theme(page, t);
        await still(page);
        await still(preview);
        await page.evaluate(() => document.fonts.ready);
        await page.mouse.move(size.width / 2, size.height / 2);
        await preview.mouse.move(size.width / 2, size.height / 2);
        const where = `${t} ${size.width}×${size.height}`;
        const ps = preview.locator('aside.sidebar');

        // Agents | Kanban: the segments, the one shown, their places.
        expect.soft(await place(side.locator('.seg')), `segments' place, ${where}`).toEqual(await place(ps.locator('.seg')));
        expect.soft(await look(side.locator('.seg')), `segments, ${where}`).toEqual(await look(ps.locator('.seg')));
        // ESC-65 added a fourth view, « PR », chosen on its preview: the four share the row at the size of every
        // `segmented` (12.5px), 3px on each side; the rest of the look, weight and face included, stays the preview's.
        const shown = side.locator('.seg button[aria-pressed="true"]');
        const drawn = ps.locator('.seg button[aria-selected="true"]');
        expect
          .soft(await lookBut(shown, 'width', 'padding', 'font'), `Agents, ${where}`)
          .toEqual(await lookBut(drawn, 'width', 'padding', 'font'));
        const face = (l: Record<string, unknown>) => String(l.font).replace(/ [\d.]+px /, ' ');
        expect.soft(face(await look(shown)), `Agents' face, ${where}`).toEqual(face(await look(drawn)));

        // « Nouvel agent Ctrl N » and the arrow of its menu « Harness ».
        expect.soft(await place(side.locator('.new-agent')), `new agent's place, ${where}`).toEqual(await place(ps.locator('.new-agent')));
        expect.soft(await look(side.locator('.new-agent')), `new agent, ${where}`).toEqual(await look(ps.locator('.new-agent')));
        expect.soft(await look(side.locator('.new-agent kbd')), `its key, ${where}`).toEqual(await look(ps.locator('.new-agent kbd')));
        expect.soft(await place(side.locator('.new-more')), `its arrow, ${where}`).toEqual(await place(ps.locator('#new-more')));

        // A card: the agent never written to, as « fix-login »; the one open in relief, as « pagination-users ».
        const card = side.locator('.agent').first();
        const idle = ps.locator('.agent.idle').last();
        for (const part of ['.a-top', '.a-meta'])
          expect
            .soft(await lookBut(card.locator(part), 'width'), `card ${part}, ${where}`)
            .toEqual(await lookBut(idle.locator(part), 'width'));
        expect
          .soft(await lookBut(card.locator('.name'), 'width'), `card's name, ${where}`)
          .toEqual(await lookBut(idle.locator('.a-name'), 'width'));
        expect
          .soft(await lookBut(card.locator('.a-state'), 'width'), `card's state, ${where}`)
          .toEqual(await lookBut(idle.locator('.a-state'), 'width'));
        expect.soft(await look(card.locator('.state-dot')), `card's dot, ${where}`).toEqual(await look(idle.locator('.state-dot')));
        expect.soft(await lookBut(card, 'height'), `the card open, ${where}`).toEqual(await lookBut(ps.locator('#ag-pag'), 'height'));
        expect
          .soft((await place(card)).slice(0, 3), `the card's place, ${where}`)
          .toEqual((await place(ps.locator('#ask-agent'))).slice(0, 3));

        // Lancement, Terminaux, the folder and the branch at the bottom: the headers fold in the short window.
        expect.soft(await place(side.locator('.side-foot')), `footer's place, ${where}`).toEqual(await place(ps.locator('.side-foot')));
        expect
          .soft(await lookBut(side.locator('.side-foot .foot-row').first(), 'width'), `folder, ${where}`)
          .toEqual(await lookBut(ps.locator('.side-foot .foot-row').first(), 'width'));
        for (const [name, sel] of [
          ['Lancement', '.runs'],
          ['Terminaux', '.terms'],
        ]) {
          const a = side.locator(`${sel} .sec-h`);
          const p = ps.locator(`section[aria-label="${name}"] .sec-h`);
          expect.soft(await lookBut(a, 'width'), `${name}'s header, ${where}`).toEqual(await lookBut(p, 'width'));
          if (size.height < 820) expect.soft(await place(a), `${name}'s place, ${where}`).toEqual(await place(p));
        }
        if (size.height < 820) {
          await expect(side.locator('.runs .rows')).toBeHidden();
          await expect(side.locator('.terms .rows')).toBeHidden();
        } else {
          await expect(side.locator('.runs .rows')).toBeVisible();
        }

        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `sidebar${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }
      }
  } finally {
    await browser.close();
  }
});

test('the menu « Harness » opens on Ctrl+Shift+N, as the preview draws it, and makes the agent of its number', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  await expect(side.locator('.agent')).toHaveCount(1);
  await page.keyboard.press('Control+Shift+N');
  const menu = page.getByRole('menu');
  await expect(menu).toBeVisible();
  await expect(menu.getByText('Harness', { exact: true })).toBeVisible();
  const claude = menu.getByRole('menuitemradio', { name: /Claude Code/ });
  await expect(claude).toHaveAttribute('aria-checked', 'true');
  await expect(claude).toContainText('Celui du projet');
  await expect(claude.locator('kbd')).toHaveText('1');
  await expect(menu.getByRole('menuitem', { name: /Nouvel agent dans un worktree/ })).toBeVisible();
  // Under « Worktree », the agent of a worktree that starts from another branch (ESC-160), with its icon.
  const from = menu.getByRole('menuitem', { name: 'Nouvel agent depuis une autre branche…' });
  await expect(from.locator('[data-icon="merge"]')).toHaveCount(1);
  const tops = await menu.locator('[role^="menuitem"]').evaluateAll((els) => els.map((e) => e.textContent?.trim() ?? ''));
  expect(tops.findIndex((x) => x.startsWith('Nouvel agent depuis'))).toBe(
    tops.findIndex((x) => x.startsWith('Nouvel agent sur une branche')) + 1,
  );
  await expect(menu.getByRole('menuitem', { name: /Harness par défaut du projet/ })).toBeVisible();
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'new-agent.app.png') });
  await page.keyboard.press('1');
  await expect(menu).toBeHidden();
  await expect(side.locator('.agent')).toHaveCount(2);
});

test('a question is answered from the list, the conversation open staying where it is', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  await send(page, 'question');
  const asking = side.locator('.agent.waiting');
  await expect(asking).toContainText('Quelle base de données ?');
  const asker = await asking.getAttribute('data-agent');
  // Another agent, open: the one asking stays first, its choices on its card.
  await page.keyboard.press('Control+N');
  await expect(side.locator('.agent')).toHaveCount(2);
  await expect(side.locator('.agent').first()).toHaveClass(/waiting/);
  // Ctrl+N opens the new agent: read once it is the one open.
  await expect(side.locator('.agent[aria-current="true"]')).not.toHaveAttribute('data-agent', asker!);
  const open = await side.locator('.agent[aria-current="true"]').getAttribute('data-agent');
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'waiting.app.png') });
  await asking.getByRole('button', { name: '2 SQLite' }).click();
  await expect(side.locator('.agent.waiting')).toHaveCount(0);
  expect(await side.locator('.agent[aria-current="true"]').getAttribute('data-agent')).toBe(open);
  // The answer reached the agent, which went on.
  await side.locator(`.agent[data-agent="${asker}"]`).click();
  await expect(page.getByText('Choix retenu : SQLite')).toBeVisible();
});

test('Ctrl+J and the keys 1, 2… answer a permission and two questions from the list, the conversation open staying', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  // A permission, then two questions, by two agents; a third one open.
  await send(page, 'permission');
  const allowing = side.locator('.agent.waiting');
  await expect(allowing).toContainText('rm -rf build');
  const allower = (await allowing.getAttribute('data-agent'))!;
  await page.keyboard.press('Control+N');
  await expect(side.locator('.agent')).toHaveCount(2);
  await expect(page.getByText('Agent prêt')).toBeVisible();
  await send(page, 'deux questions');
  await expect(side.locator('.agent.waiting')).toHaveCount(2);
  const asker = (await side.locator('.agent.waiting', { hasText: 'Question 1 sur 2' }).getAttribute('data-agent'))!;
  await page.keyboard.press('Control+N');
  await expect(side.locator('.agent')).toHaveCount(3);
  const open = (await side.locator('.agent[aria-current="true"]').getAttribute('data-agent'))!;
  const card = (id: string) => side.locator(`.agent[data-agent="${id}"]`);

  // The permission: what « always » allows on its card, numbered.
  const perm = card(allower);
  await expect(perm.getByRole('group', { name: /^Réponses à / }).getByRole('button')).toHaveText([
    '1Autoriser',
    '2Toujours autoriser',
    '3Refuser',
  ]);
  if (SHOTS) {
    await theme(page, 'dark');
    await page.screenshot({ path: path.join(SHOTS, 'list-answers.app.png') });
    await theme(page, 'light');
    await page.screenshot({ path: path.join(SHOTS, 'list-answers-light.app.png') });
  }
  // Ctrl+J gives the focus to the card of the agent waiting the longest: no conversation opens.
  // The keys wait half a second once a request is shown, for a key meant for something else not to answer it unread.
  await page.waitForTimeout(600);
  await page.keyboard.press('Control+j');
  await expect(perm).toBeFocused();
  expect(await side.locator('.agent[aria-current="true"]').getAttribute('data-agent')).toBe(open);
  await page.keyboard.press('2');
  // Sent off again: a brief green flash, and it goes back to its place.
  await expect(perm).toHaveClass(/flash/);
  await expect(perm).not.toHaveClass(/waiting/);
  await expect(page.getByText('« Toujours autoriser »', { exact: false }).first()).toBeVisible();

  // The two questions, one after the other, from anywhere but a field.
  await page.keyboard.press('Control+j');
  const ask = card(asker);
  await expect(ask).toBeFocused();
  await page.locator('body').evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press('2');
  await expect(ask).toContainText('Question 2 sur 2');
  await expect(ask).toContainText('Quel ORM ?');
  await page.keyboard.press('1');
  await expect(side.locator('.agent.waiting')).toHaveCount(0);
  expect(await side.locator('.agent[aria-current="true"]').getAttribute('data-agent')).toBe(open);

  // The answers reached the agents, which went on.
  await card(asker).click();
  // Its answers by question, in the order the backend keeps them.
  await expect(page.getByText(/Choix retenu : (SQLite, Prisma|Prisma, SQLite)/)).toBeVisible();
  await card(allower).click();
  await expect(page.getByText('Commande exécutée (règle enregistrée).')).toBeVisible();
});

test('the ring of the agent that asks stops with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'question');
  const dot = page.locator('aside.side .agent.waiting .state-dot');
  await expect(dot).toBeVisible();
  const ring = () => dot.evaluate((el) => getComputedStyle(el, '::after').animationName);
  expect(await ring()).toMatch(/ring/);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  expect(await ring()).toBe('none');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the sidebar fit in their place at 1028 × 779, its menu « Harness » open', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    await page.setViewportSize({ width: 1028, height: 779 });
    const side = page.locator('aside.side');
    await expect(side.locator('.new-agent')).toContainText('New agent');
    await expect(side.locator('.runs .sec-h')).toContainText('Launch');
    await expect(side.locator('.terms .sec-h')).toContainText('Terminals');
    await side.locator('.new-more').click();
    await expect(page.getByRole('menu')).toBeVisible();
    // What is cut short, or overflows its box: none of it.
    const cut = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLElement>('aside.side *, [role="menu"] *')]
        .filter(
          (el) =>
            el.offsetParent &&
            el.children.length === 0 &&
            !el.closest('.mono, .name, .model') &&
            (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
        )
        .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
    );
    expect(cut).toEqual([]);
    const box = (await page.getByRole('menu').boundingBox())!;
    expect(box.x + box.width).toBeLessThanOrEqual(1028);
    expect(box.y + box.height).toBeLessThanOrEqual(779);
    if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'new-agent-small-en.app.png') });
  });

  test('a permission answered from the list fits its card at 1028 × 779', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    await page.setViewportSize({ width: 1028, height: 779 });
    const side = page.locator('aside.side');
    await send(page, 'permission');
    const choices = side.locator('.agent.waiting').getByRole('group', { name: /^Answers to / });
    await expect(choices.getByRole('button')).toHaveText(['1Allow', '2Always allow', '3Deny']);
    const cut = await choices.evaluate((group) =>
      [...group.querySelectorAll<HTMLElement>('button')]
        .filter((b) => b.scrollWidth > b.clientWidth + 1 || b.getBoundingClientRect().right > group.getBoundingClientRect().right + 1)
        .map((b) => b.textContent),
    );
    expect(cut).toEqual([]);
    if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'list-answers-small-en.app.png') });
  });
});

// What the agent does, on its card (ESC-74): its turn over while its work goes on, its tokens and its cost, its pull
// request under it, as the preview's #waiting and #pr draw them; then a turn running, its tool and what it acts on.
test.describe('what the agent does, on its card', () => {
  let github: { url: string; close: () => Promise<void> };
  const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

  test.beforeAll(async () => {
    github = await startFakeTrackers(0);
    // Read by the app the fixture starts next: github.com's API is the fake one, its pull requests read at each selection.
    process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
    process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
  });

  test.afterAll(async () => {
    await github?.close();
    delete process.env.ESCOUADE_GITHUB_API;
    delete process.env.ESCOUADE_PR_READ_EVERY_MS;
  });

  /** Connects github.com's account (the fake one's) in Réglages › Intégrations. */
  async function connectGithub(page: Page) {
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await settings.getByRole('tab', { name: 'Intégrations' }).click();
    await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
    const form = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
    await form.getByLabel(/Jeton/).fill('ghp_jeton');
    await form.getByRole('button', { name: 'Connecter' }).click();
    await expect(settings.getByText('Connecté · @ada').first()).toBeVisible();
    await settings.getByRole('button', { name: 'Enregistrer' }).click();
    await expect(settings).toHaveCount(0);
  }

  test('looks as the preview’s #waiting and #pr draw it, in both themes, at both sizes, then says what it runs', async ({ app }) => {
    const exe = chromium.executablePath();
    test.skip(
      !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
      'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
    );
    const { page } = app;
    git(app.repo, 'remote', 'add', 'origin', 'https://github.com/acme/demo.git');
    await addProject(page, app.repo, { worktrees: true });
    const side = page.locator('aside.side');
    const card = side.locator('.agent').first();
    await expect(side.locator('.agent')).toHaveCount(1, { timeout: 60_000 });
    await connectGithub(page);
    // A pull request on the branch of the agent's worktree, its CI failed: under its card once it is read again. The
    // repository's own worktree comes first in the list.
    const worktrees = () =>
      git(app.repo, 'worktree', 'list', '--porcelain')
        .split('\n')
        .filter((l) => l.startsWith('worktree '))
        .map((l) => l.slice('worktree '.length))
        .slice(1);
    await expect.poll(() => worktrees().length, { timeout: 30_000 }).toBe(1);
    const branch = git(worktrees()[0], 'branch', '--show-current');
    await fetch(`${github.url}/github/repos/acme/demo/pulls`, {
      method: 'POST',
      body: JSON.stringify({ title: 'Connexion par jeton', body: '', head: branch, base: 'main' }),
    });
    await fetch(`${github.url}/__github/pulls/101`, { method: 'POST', body: JSON.stringify({ checks: 'FAILURE' }) });
    await page.getByRole('button', { name: /^Kanban/ }).click();
    await page.getByRole('button', { name: 'Agents', exact: true }).click();
    await card.click();
    const prs = side.getByRole('group', { name: /^Pull requests de / });
    await expect(prs.locator('.prnum')).toHaveText('#101');
    await expect(prs.locator('.prnum [data-icon="close"]')).toHaveCount(1);

    // Its turn over, a command left running: « Attend », and what goes on in the place of what it did.
    await send(page, 'commande de fond 600s');
    await expect(card.locator('[data-pending]')).toHaveText('Attend', { timeout: 30_000 });
    const act = card.locator('.a-act');
    await expect(act.locator('.tgt')).toHaveText('Tour fini · 1 tâche de fond');
    await expect(act.locator('[data-icon="check"]')).toHaveCount(1);
    await expect(act.locator('.spend')).toHaveText(/^[\d,]+(\sk)? · [\d,]+\s\$$/);

    const browser = await chromium.launch();
    const preview = await browser.newPage();
    /** The preview at `route`, in theme `t`, at `size`, the app beside it alike. */
    const both = async (route: string, t: 'dark' | 'light', size: (typeof SIZES)[number]) => {
      await page.setViewportSize(size);
      await preview.setViewportSize(size);
      // Another page first: from one route to another, only the hash would change, and the preview reads it once.
      await preview.goto('about:blank');
      await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#${route}`);
      await preview.evaluate(() => document.fonts.ready);
      await theme(page, t);
      await still(page);
      await still(preview);
      await page.evaluate(() => document.fonts.ready);
      await page.mouse.move(size.width / 2, size.height / 2);
      await preview.mouse.move(size.width / 2, size.height / 2);
    };
    const shot = async (name: string) => {
      if (!SHOTS) return;
      fs.mkdirSync(SHOTS, { recursive: true });
      await page.screenshot({ path: path.join(SHOTS, `${name}.app.png`) });
      await preview.screenshot({ path: path.join(SHOTS, `${name}.preview.png`) });
    };
    /** Where `parts` stand in the card `c`: from its left and its top. */
    const within = async (c: Locator, parts: Locator[]) => {
      const [x, y] = await place(c);
      return Promise.all(parts.map(async (p) => ((r) => [r[0] - x, r[1] - y])(await place(p))));
    };
    /** Where the group `g` stands under the card `c`: from its left, and from its bottom. */
    const under = async (c: Locator, g: Locator) => {
      const card = await c.evaluate((n) => n.getBoundingClientRect().toJSON());
      const group = await g.evaluate((n) => n.getBoundingClientRect().toJSON());
      return [Math.round(group.x - card.x), Math.round(group.y - card.bottom)];
    };
    try {
      for (const t of ['dark', 'light'] as const)
        for (const size of SIZES) {
          const where = `${t} ${size.width}×${size.height}`;
          const light = t === 'light' ? '-light' : '';

          // #waiting: « refacto-auth », its turn over, a subagent and a background task going on.
          await both('waiting', t, size);
          const pc = preview.locator('#ag-refacto');
          const pact = pc.locator('.a-act');
          expect
            .soft(await lookBut(card.locator('.a-state'), 'width'), `« Attend », ${where}`)
            .toEqual(await lookBut(pc.locator('.a-state'), 'width'));
          expect.soft(await lookBut(act, 'width'), `the line of what it did, ${where}`).toEqual(await lookBut(pact, 'width'));
          expect.soft(await look(act.locator('.ok svg')), `its check, ${where}`).toEqual(await look(pact.locator('svg.ok')));
          expect
            .soft(await lookBut(act.locator('.tgt'), 'width'), `« Tour fini · … », ${where}`)
            .toEqual(await lookBut(pact.locator('.tgt'), 'width'));
          expect
            .soft(await lookBut(act.locator('.spend'), 'width'), `its tokens and cost, ${where}`)
            .toEqual(await lookBut(pact.locator('.spend'), 'width'));
          expect
            .soft(await within(card, [act.locator('.ok svg'), act.locator('.tgt')]), `its parts' place, ${where}`)
            .toEqual(await within(pc, [pact.locator('svg.ok'), pact.locator('.tgt')]));
          // Its background task's long command is cut short in the sidebar, never past its edge (ESC-84).
          const sideRight = await page.locator('aside.side').evaluate((n) => n.getBoundingClientRect().right);
          for (const row of await page.locator('aside.side .subl .sub').all())
            expect
              .soft(await row.evaluate((n) => n.getBoundingClientRect().right), `a row under the card, ${where}`)
              .toBeLessThanOrEqual(sideRight);
          await shot(`card-waiting${size.name}${light}`);

          // #pr: the pull request under the card, its CI failed, « L’agent corrige ».
          await both('pr', t, size);
          const pp = preview.locator('#prs-refacto');
          expect.soft(await lookBut(prs, 'width', 'height'), `the group, ${where}`).toEqual(await lookBut(pp, 'width', 'height'));
          expect
            .soft(await lookBut(prs.locator('.prnum'), 'width'), `its number, ${where}`)
            .toEqual(await lookBut(pp.locator('.prnum'), 'width'));
          expect.soft(await look(prs.locator('.prnum svg')), `its icon, ${where}`).toEqual(await look(pp.locator('.prnum svg')));
          expect
            .soft(await lookBut(prs.locator('.prfix'), 'width'), `its correction, ${where}`)
            .toEqual(await lookBut(pp.locator('.prfix'), 'width'));
          expect
            .soft(await under(card, prs), `its place under the card, ${where}`)
            .toEqual(await under(preview.locator('#ag-refacto'), pp));
          await shot(`card-pr${size.name}${light}`);
        }

      // A turn running: its tool and what it acts on, as « pagination-users » says « Bash npm test ».
      await send(page, 'commande premier plan 60s');
      await expect(act.locator('b')).toHaveText('Lance', { timeout: 30_000 });
      await expect(act.locator('.tgt')).toContainText('for i in');
      for (const t of ['dark', 'light'] as const)
        for (const size of SIZES) {
          const where = `${t} ${size.width}×${size.height}`;
          await both('', t, size);
          const pa = preview.locator('#ag-pag .a-act');
          expect.soft(await lookBut(act.locator('b'), 'width'), `its verb, ${where}`).toEqual(await lookBut(pa.locator('b'), 'width'));
          expect
            .soft(await lookBut(act.locator('.tgt'), 'width'), `what it acts on, ${where}`)
            .toEqual(await lookBut(pa.locator('.tgt'), 'width'));
          expect
            .soft(await lookBut(act.locator('.spend'), 'width'), `its spending, ${where}`)
            .toEqual(await lookBut(pa.locator('.spend'), 'width'));
          await shot(`card-running${size.name}${t === 'light' ? '-light' : ''}`);
        }
    } finally {
      await browser.close();
    }
  });
});
