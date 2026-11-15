// The « Plan » banner of the 2.0 design (ESC-77, lot 5): in the thread's column, a bar of a segment for each task, the
// title, the subagents running, the tasks done and the share; each task with its state, its subagent and its model, its
// step, what it waits for; the handle under the list. Compared to the preview (design/app-redesign/index.html, `#plan`
// and `#plan-closed`) in both themes at 1440 × 900 and at CI's 1028 × 779; the English labels in their room; the motion
// stopped by prefers-reduced-motion. The captures of the app and of the preview go to ESCOUADE_E2E_SHOTS when it is set.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

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

/** A plan run that waits on its third task (tests/fixtures/superpowers-run.md): two tasks done, one subagent running. */
async function waitingPlan(page: Page) {
  await send(page, 'plan superpowers');
  await expect(page.getByTestId('question-pending')).toContainText('Quelle base de données ?');
  const plan = page.locator('main.conv .plan');
  await expect(plan.locator('.head')).toContainText('2/3');
  return plan;
}

/** Opens the banner's list if the window folded it. */
async function unfold(plan: Locator) {
  const fold = plan.locator('.fold');
  if ((await fold.getAttribute('aria-expanded')) !== 'true') await fold.click();
  await expect(plan.locator('.list')).toBeVisible();
  // The chevron turns as the banner opens (0.25 s): measured midway, its 14 px square tilted takes 16 and overflows the head
  // by a pixel. Its finite animations ended (the subagents' dots breathe for ever).
  await plan.evaluate((el) =>
    Promise.all(
      el
        .getAnimations({ subtree: true })
        .filter((a) => a.effect?.getTiming().iterations !== Infinity)
        .map((a) => a.finished),
    ),
  );
}

test('the plan looks as the preview draws it, open and folded, in both themes, at both sizes', async ({ app }) => {
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await addProject(page, app.repo);
  const plan = await waitingPlan(page);
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
        await page.mouse.move(size.width / 2, 20);
        await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
        await preview.mouse.move(size.width / 2, 20);
        await unfold(plan);
        const where = `${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#plan');

        // The box across the conversation, right under the header (whose height is the header's, lot 4: its content
        // differs); its rows in the thread's column. What follows is placed from the top of the box.
        const [bx, by, bw] = await place(plan);
        const [pbx, pby, pbw] = await place(pv);
        const [, hy, , hh] = await place(page.locator('main.conv header.head'));
        const [, phy, , phh] = await place(preview.locator('#view-agents .conv > .head').first());
        expect.soft([bx, by - hy - hh, bw], `banner's place, ${where}`).toEqual([pbx, pby - phy - phh, pbw]);
        expect.soft(await lookBut(plan, 'height'), `banner, ${where}`).toLookLike(await lookBut(pv, 'height'));
        expect
          .soft(await lookBut(plan.locator('.inner'), 'height'), `its column, ${where}`)
          .toLookLike(await lookBut(pv.locator('.plan-inner'), 'height'));
        const [ix, , iw] = await place(plan.locator('.inner'));
        const [pix, , piw] = await place(pv.locator('.plan-inner'));
        expect.soft([ix, iw], `its column's place, ${where}`).toEqual([pix, piw]);

        // The head: « Plan », the title, the subagents running, the tasks done, the share, the chevron.
        const head = plan.locator('.fold');
        const phead = pv.locator('.plan-head');
        expect.soft((await place(head))[3], `head's height, ${where}`).toBe((await place(phead))[3]);
        expect
          .soft(await lookBut(head.locator('.badge'), 'width'), `« Plan », ${where}`)
          .toLookLike(await lookBut(phead.locator('.badge'), 'width'));
        expect
          .soft(await lookBut(head.locator('.title'), 'width'), `title, ${where}`)
          .toLookLike(await lookBut(phead.locator('.ttl'), 'width'));
        expect
          .soft(await lookBut(head.locator('.pct'), 'width'), `share, ${where}`)
          .toLookLike(await lookBut(phead.locator('.pct'), 'width'));
        expect.soft(await look(head.locator('.chev svg')), `chevron, ${where}`).toLookLike(await look(phead.locator('.chev')));
        const count = head.locator('.count');
        const pcount = phead.locator('.cnt');
        expect.soft(await count.isVisible(), `tasks' count shown, ${where}`).toBe(await pcount.isVisible());
        if (await pcount.isVisible())
          expect.soft(await lookBut(count, 'width'), `tasks' count, ${where}`).toLookLike(await lookBut(pcount, 'width'));
        const subs = plan.locator('.head .subs');
        const psubs = phead.locator('.subs');
        expect.soft(await subs.isVisible(), `subagents shown, ${where}`).toBe(await psubs.isVisible());
        if (await psubs.isVisible()) {
          expect.soft(await lookBut(subs, 'width'), `subagents, ${where}`).toLookLike(await lookBut(psubs, 'width'));
          expect
            .soft(await look(subs.locator('.dot').first()), `a subagent's dot, ${where}`)
            .toLookLike(await look(psubs.locator('.dots i').first()));
        }
        // Right edges: the chevron ends where the preview's does.
        const [cx, , cw] = await place(head.locator('.chev svg'));
        const [pcx, , pcw] = await place(phead.locator('.chev'));
        expect.soft(cx + cw, `chevron's right edge, ${where}`).toBe(pcx + pcw);

        // The bar: a segment for each task.
        const seg = plan.locator('.segs .seg');
        expect
          .soft(await lookBut(seg.last(), 'width'), `a segment to do, ${where}`)
          .toLookLike(await lookBut(pv.locator('.bar7 i:not(.done):not(.run)').last(), 'width'));
        expect
          .soft(await lookBut(seg.first().locator('.fill'), 'width'), `a segment done, ${where}`)
          .toLookLike(await lookBut(pv.locator('.bar7 i.done').first(), 'width'));
        const [sx, sy, sw] = await place(plan.locator('.segs'));
        const [psx, psy, psw] = await place(pv.locator('.bar7'));
        expect.soft([sx, sy - by, sw], `bar's place, ${where}`).toEqual([psx, psy - pby, psw]);

        // The list: the rows of the tasks done, the one to look at.
        const row = plan.locator('.row.done').first();
        const prow = pv.locator('.plan-list li.done').first();
        const [rx, ry, rw, rh] = await place(row);
        const [prx, pry, prw, prh] = await place(prow);
        expect.soft([rx, ry - by, rw, rh], `a row's place, ${where}`).toEqual([prx, pry - pby, prw, prh]);
        expect.soft(await look(row), `a row done, ${where}`).toLookLike(await look(prow));
        expect
          .soft(await lookBut(row.locator('.n'), 'width'), `its rank, ${where}`)
          .toLookLike(await lookBut(prow.locator('.no'), 'width'));
        expect.soft(await look(row.locator('.ico svg')), `its icon, ${where}`).toLookLike(await look(prow.locator('.st-ic svg')));
        expect
          .soft(await lookBut(row.locator('.rt'), 'width'), `its title, ${where}`)
          .toLookLike(await lookBut(prow.locator('.rt'), 'width'));
        expect.soft(await look(row.locator('.rbar')), `its bar, ${where}`).toLookLike(await look(prow.locator('.rbar')));
        expect
          .soft(await lookBut(row.locator('.rpct'), 'width'), `its share, ${where}`)
          .toLookLike(await lookBut(prow.locator('.rpct'), 'width'));
        expect
          .soft(await lookBut(plan.locator('.row.focus'), 'width', 'height'), `the row to look at, ${where}`)
          .toLookLike(await lookBut(pv.locator('.plan-list li.focus'), 'width', 'height'));
        expect
          .soft(await look(plan.locator('.row.focus .rt')).then((l) => l.font), `its title, ${where}`)
          .toBe(await look(pv.locator('.plan-list li.focus .rt')).then((l) => l.font));
        const tag = plan.locator('.subtag').first();
        if (await tag.isVisible())
          expect
            .soft(await lookBut(tag, 'width'), `a subagent's tag, ${where}`)
            .toLookLike(await lookBut(pv.locator('.plan-list .subtag').first(), 'width'));

        // The handle under the list.
        const grip = plan.getByRole('separator');
        const pgrip = pv.locator('.grip');
        const [gx, , gw, gh] = await place(grip);
        const [pgx, , pgw, pgh] = await place(pgrip);
        expect.soft([gx, gw, gh], `handle's place, ${where}`).toEqual([pgx, pgw, pgh]);
        const handle = (el: Locator) =>
          el.evaluate((node) => {
            const s = getComputedStyle(node, node.getAttribute('role') ? '::after' : null);
            return [s.width, s.height, s.backgroundColor, s.borderRadius];
          });
        expect.soft(await handle(grip), `handle, ${where}`).toLookLike(await handle(pgrip.locator('i')));

        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `plan${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }

        // Folded (`#plan-closed`): the head and the bar, no list, no handle; the chevron turned.
        await head.click();
        await preview.locator('#plan-toggle').click();
        await page.mouse.move(size.width / 2, 20);
        await expect(plan.locator('.list')).toHaveCount(0);
        await expect(grip).toHaveCount(0);
        const [, , , fh] = await place(plan);
        const [, , , pfh] = await place(pv);
        expect.soft(fh, `folded banner's height, ${where}`).toBe(pfh);
        const turn = (el: Locator) => el.evaluate((node) => getComputedStyle(node).transform);
        expect.soft(await turn(head.locator('.chev')), `folded chevron, ${where}`).toBe(await turn(phead.locator('.chev')));
        if (SHOTS) {
          const n = `plan-closed${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }
        await head.click();
      }
  } finally {
    await browser.close();
  }
});

test('the spinner of a task and the dots of the subagents stop with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  // A busy superpowers agent: a plan of 40 tasks, task after task under way with its subagents.
  await send(page, 'flood-real 60s');
  const plan = page.locator('main.conv .plan');
  await unfold(plan);
  const spin = plan.locator('.row.run .spin').first();
  const dot = plan.locator('.subs .dot').first();
  await expect(spin).toBeVisible({ timeout: 20_000 });
  await expect(dot).toBeAttached({ timeout: 20_000 });
  const motion = () => Promise.all([spin, dot].map((el) => el.evaluate((e) => getComputedStyle(e).animationName)));
  // Their keyframes, named by Svelte after the component.
  const [spins, breathes] = await motion();
  expect(spins).toMatch(/spin/);
  expect(breathes).toMatch(/breathe/);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  expect(await motion()).toLookLike(['none', 'none']);
  // Still told: the spinner is a whole ring of the project's colour.
  expect(await spin.evaluate((e) => getComputedStyle(e).borderTopColor)).toBe(
    await spin.evaluate((e) => getComputedStyle(e).borderLeftColor),
  );
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.getByRole('button', { name: 'Stop' }).click();
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the banner fit in their place, at both sizes', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    await send(page, 'plan superpowers');
    await expect(page.getByTestId('question-pending')).toBeVisible();
    const plan = page.locator('main.conv .plan');
    await expect(plan.locator('.head')).toContainText('2/3 tasks');
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await unfold(plan);
      await expect(plan.locator('.row.block')).toContainText('Waiting for your answer');
      await expect(plan.locator('.row.block .rpct')).toHaveText('Blocked');
      // What is cut short, or overflows its box: none of it, but the agent's own words (titles, steps), cut by design.
      const cut = await plan.evaluate((root) =>
        [...root.querySelectorAll<HTMLElement>('*')]
          .filter(
            (el) =>
              el.offsetParent &&
              el.children.length === 0 &&
              !el.closest('.title, .rt, .step, .sname, .sr') &&
              (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
          )
          .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
      );
      expect(cut, `${size.width}×${size.height}`).toEqual([]);
      // The head holds on one line, the rows keep their height.
      expect(await plan.locator('.head').evaluate((h) => h.scrollWidth <= h.clientWidth + 1 && h.getBoundingClientRect().height < 30)).toBe(
        true,
      );
      for (const box of await plan.locator('.row').all()) expect((await box.boundingBox())!.height).toBeLessThanOrEqual(31);
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `plan${size.name}-en.app.png`) });
      }
    }
  });
});
