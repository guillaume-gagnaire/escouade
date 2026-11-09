// What a narrow column does to the headers it holds (ESC-166): the agent's header (name and state, path and branch,
// figures, actions) folds by the width of the conversation, not the window's, and nothing in it overflows nor overlaps,
// the side panel (Git, diff, commits) open or not, in the classic layout and side by side, at 1028, 1280 and 1440 px,
// in French and in English. The composer, the plan banner, a question's card, Git's tabs and the headers of the agent's
// terminal view, of a terminal and of a launch's log are measured the same way.
//
// The side panel has one width per layout (340 px beside the classic one, 44 % of the work but never under 340 px side by
// side): its default width in one, its widest in the other.

import { type Locator, type Page } from '@playwright/test';
import { addProject, expect, send, test, type Language } from './fixture';

const SIZES = [
  { width: 1028, height: 779 },
  { width: 1280, height: 800 },
  { width: 1440, height: 900 },
];

/**
 * What overflows in `root`: itself or a box that clips its content (but an ellipsis, which is meant), a box out of `root`,
 * two of the boxes of `rows` (the children of each) that overlap, a line of text that went on two lines.
 */
function overflows(root: Locator, rows: string[] = []) {
  return root.evaluate((top, rows) => {
    const out: string[] = [];
    const name = (el: Element) =>
      `${el.tagName.toLowerCase()}${
        el.className && typeof el.className === 'string'
          ? '.' +
            el.className
              .trim()
              .split(/\s+/)
              .filter((c) => !c.startsWith('svelte-'))
              .join('.')
          : ''
      }${el.textContent?.trim() ? ` « ${el.textContent.trim().slice(0, 30)} »` : ''}`;
    const shown = (el: Element) => {
      const r = el.getBoundingClientRect();
      const s = getComputedStyle(el);
      return r.width > 1 && r.height > 1 && s.visibility !== 'hidden';
    };
    const box = top.getBoundingClientRect();
    for (const el of [top, ...top.querySelectorAll('*')]) {
      if (!shown(el) || (el.closest('svg') !== null && el.tagName !== 'svg')) continue;
      const s = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      const clips = s.overflowX === 'hidden' || s.overflowX === 'clip';
      if (el.scrollWidth > el.clientWidth + 1 && (el === top || clips) && s.textOverflow !== 'ellipsis')
        out.push(`${name(el)} cut: ${el.scrollWidth} > ${el.clientWidth}`);
      if (el !== top && (r.left < box.left - 1 || r.right > box.right + 1))
        out.push(`${name(el)} out: ${Math.round(r.left)}–${Math.round(r.right)} of ${Math.round(box.left)}–${Math.round(box.right)}`);
      // One line of text: a word that went to the next one.
      for (const node of el.childNodes) {
        if (node.nodeType !== Node.TEXT_NODE || !node.textContent?.trim()) continue;
        const range = document.createRange();
        range.selectNodeContents(node);
        const tops = new Set([...range.getClientRects()].filter((q) => q.width > 0).map((q) => Math.round(q.top)));
        if (tops.size > 1 && s.whiteSpace === 'nowrap') out.push(`${name(el)} on ${tops.size} lines`);
        // A tooltip's text goes on several lines by design (the cache's detail, ESC-398).
        if (tops.size > 1 && el.closest('header, .utabs, .side-top, .actions') && !el.closest('[role="tooltip"]'))
          out.push(`${name(el)} on ${tops.size} lines`);
      }
    }
    for (const sel of rows)
      for (const row of top.querySelectorAll(sel)) {
        const kids = [...row.children].filter(shown).map((el) => [el, el.getBoundingClientRect()] as const);
        for (let i = 0; i < kids.length; i++)
          for (let j = i + 1; j < kids.length; j++) {
            const [a, ra] = kids[i];
            const [b, rb] = kids[j];
            const x = Math.min(ra.right, rb.right) - Math.max(ra.left, rb.left);
            const y = Math.min(ra.bottom, rb.bottom) - Math.max(ra.top, rb.top);
            if (x > 1 && y > 1) out.push(`${name(a)} over ${name(b)}`);
          }
      }
    return out;
  }, rows);
}

/** The width of an element, in px. */
const widthOf = (el: Locator) => el.evaluate((n) => Math.round(n.getBoundingClientRect().width));

/** The layouts measured: the panel closed, beside the classic layout (its default width), side by side (its widest). */
const LAYOUTS = ['closed', 'panel', 'split'] as const;

const WORDS = {
  fr: {
    closeFiles: 'Fermer les fichiers',
    classic: 'Disposition classique',
    split: 'Conversation et fichiers côte à côte',
    history: 'Historique',
  },
  en: { closeFiles: 'Close the files', classic: 'Classic layout', split: 'Conversation and files side by side', history: 'History' },
} as const;

/** Two frames: the container queries of a new width apply. */
const settle = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));

/** What is wrong with the agent's header, as it is: what overflows, and its name, state or main button out of sight. */
async function headerProblems(page: Page, where: string) {
  const head = page.locator('main.conv:not(.subview) header.head');
  const problems = (await overflows(head, ['.head', '.h-title .row', '.crumbs', '.figures', '.actions'])).map(
    (p) => `header, ${where}: ${p}`,
  );
  // Its name in full, or at least 80 px of it before its ellipsis.
  const [shown, full] = await head.locator('.name').evaluate((n) => [Math.round(n.getBoundingClientRect().width), n.scrollWidth]);
  if (shown < Math.min(full, 80)) problems.push(`header, ${where}: the name has ${shown} px of ${full}`);
  const state = head.locator('.state');
  if (!(await state.isVisible()) || (await state.evaluate((n) => n.scrollWidth > n.clientWidth + 1)))
    problems.push(`header, ${where}: the state is not seen in full`);
  if (!(await head.locator('.actions .btn.test, .actions .btn.openpr').first().isVisible()))
    problems.push(`header, ${where}: no main button`);
  // The branch kept (some of it) while the project's name gives way.
  if ((await widthOf(head.locator('.crumbs .branch'))) < 40) problems.push(`header, ${where}: the branch is gone`);
  return problems;
}

/** The headers of a conversation and what `what` names, measured in each window and layout. */
async function measureConversation(page: Page, lang: Language, what: string[]) {
  const conv = page.locator('main.conv:not(.subview)');
  const problems: string[] = [];
  for (const size of SIZES) {
    await page.setViewportSize(size);
    for (const l of LAYOUTS) {
      await setLayout(page, lang, l);
      await page.evaluate(() => document.fonts.ready);
      await settle(page);
      const where = `${lang} ${size.width} px, ${l} (conversation ${await widthOf(conv)} px)`;
      problems.push(...(await headerProblems(page, where)));
      for (const sel of what) {
        const el = page.locator(sel).first();
        if (!(await el.isVisible())) continue;
        for (const p of await overflows(el, [':scope', ':scope > *'])) problems.push(`${sel}, ${where}: ${p}`);
      }
    }
  }
  return problems;
}

/** The layout `l`: « Fichiers » of the header opens the panel beside the classic layout, Ctrl+Shift+L puts it side by side. */
async function setLayout(page: Page, lang: Language, l: (typeof LAYOUTS)[number]) {
  const docked = page.locator('aside.git-side.docked');
  if ((l === 'split') !== (await docked.isVisible())) await page.keyboard.press('Control+Shift+L');
  if (l === 'split') return expect(docked).toBeVisible();
  await expect(docked).toBeHidden();
  const side = page.locator('aside.git-side');
  if ((l === 'panel') === (await side.isVisible())) return;
  if (l === 'closed') await side.getByRole('button', { name: lang === 'fr' ? 'Fermer' : 'Close' }).click();
  else await page.locator('main.conv header.head .fig.files').evaluate((b: HTMLElement) => b.click());
  await expect(side).toBeVisible({ visible: l === 'panel' });
}

for (const lang of ['fr', 'en'] as const)
  test.describe(lang, () => {
    test.use({ language: lang });

    test(`the agent's header and what a narrow column holds never overflow, the side panel open or not (${lang})`, async ({ app }) => {
      test.setTimeout(240_000);
      const { page } = app;
      // A worktree per agent: « Préparer le lancement », its widest button; then a turn that edited a file (cost, files,
      // duration), and a plan stopped on its question (the banner, the card).
      await addProject(page, app.repo, { worktrees: true, lang });
      await send(page, 'edit');
      await expect(page.locator('main.conv header.head .btn.test')).toBeVisible();
      // A long name, as agents have: its branch follows it.
      const name = 'pagination-par-curseur-des-utilisateurs';
      await page.locator('main.conv header.head .name').dblclick();
      await page.getByRole('dialog').getByRole('textbox').fill(name);
      await page.getByRole('dialog').getByRole('textbox').press('Enter');
      await expect(page.locator('main.conv header.head .name')).toHaveText(name);
      await expect(page.locator('main.conv header.head .branch')).toContainText(name);
      // The agent in Claude Code's own view keeps the header, its view named « Claude Code », wider than « Escouade ».
      await page.setViewportSize(SIZES[2]);
      await settle(page);
      await page.locator('main.conv header.head').locator('.vseg button').nth(1).click();
      await expect(page.locator('main.conv .xterm')).toBeVisible();
      await expect(page.locator('main.conv header.head').locator('.vseg button').nth(1)).toHaveAttribute('aria-pressed', 'true');
      const inView: string[] = [];
      await setLayout(page, lang, 'panel');
      for (let width = 1000; width <= 1960; width += 32) {
        await page.setViewportSize({ width, height: 800 });
        await settle(page);
        inView.push(...(await headerProblems(page, `${lang} ${width} px, panel, Claude Code's view`)));
      }
      await setLayout(page, lang, 'closed');
      for (let width = 1000; width <= 1960; width += 32) {
        await page.setViewportSize({ width, height: 800 });
        await settle(page);
        inView.push(...(await headerProblems(page, `${lang} ${width} px, closed, Claude Code's view`)));
      }
      expect(inView).toEqual([]);
      await page.setViewportSize(SIZES[2]);
      await settle(page);
      await page.locator('main.conv header.head .vseg button').first().click();
      await expect(page.locator('main.conv .xterm')).toHaveCount(0);
      await send(page, 'plan superpowers');
      await expect(page.getByTestId('question-pending')).toBeVisible();
      await expect(page.locator('main.conv .plan .head')).toContainText('2/3');

      const problems = await measureConversation(page, lang, [
        'main.conv .composer',
        'main.conv .plan .head',
        '[data-testid="question-pending"]',
        'aside.git-side .utabs',
        'aside.git-side .side-top',
      ]);
      // Git's history at the panel's narrowest.
      await page.setViewportSize(SIZES[0]);
      await setLayout(page, lang, 'panel');
      await page.locator('aside.git-side').getByRole('tab', { name: WORDS[lang].history }).click();
      for (const p of await overflows(page.locator('aside.git-side .side-top').first(), [':scope']))
        problems.push(`history, ${lang} 1028 px: ${p}`);
      for (const p of await overflows(page.locator('aside.git-side .utabs'), [':scope']))
        problems.push(`tabs, history, ${lang} 1028 px: ${p}`);

      // Every width between, the panel open beside the classic layout: the conversation from 324 to 1244 px.
      for (let width = 1000; width <= 1960; width += 16) {
        await page.setViewportSize({ width, height: 800 });
        await settle(page);
        problems.push(...(await headerProblems(page, `${lang} ${width} px, panel`)));
      }
      expect(problems).toEqual([]);

      // Too narrow for them, the layouts and « Fichiers » are in « … », which switches them.
      await page.setViewportSize(SIZES[0]);
      await settle(page);
      const head = page.locator('main.conv header.head');
      await expect(head.locator('.layout-seg')).toBeHidden();
      await head.locator('.more').click();
      const menu = page.getByRole('menu');
      await expect(menu.getByRole('menuitem', { name: WORDS[lang].closeFiles })).toBeVisible();
      await menu.getByRole('menuitem', { name: WORDS[lang].split }).click();
      await expect(page.locator('aside.git-side.docked')).toBeVisible();
      await head.locator('.more').click();
      await expect(menu.getByRole('menuitem', { name: WORDS[lang].closeFiles })).toHaveCount(0);
      await menu.getByRole('menuitem', { name: WORDS[lang].classic }).click();
      await expect(page.locator('aside.git-side.docked')).toBeHidden();
      await expect(page.locator('aside.git-side')).toBeVisible();
    });

    // The projects' rail widened by its edge (ESC-168) takes its room from the rest: chosen at 300 px, it is drawn as
    // wide as each window lets it (80 px at 1028, 300 from 1248), and nothing of the conversation overflows.
    test(`with the projects' rail at its widest, the agent's header and what a narrow column holds never overflow (${lang})`, async ({
      app,
    }) => {
      test.setTimeout(240_000);
      const { page } = app;
      await addProject(page, app.repo, { worktrees: true, lang });
      await send(page, 'edit');
      await expect(page.locator('main.conv header.head .btn.test')).toBeVisible();
      await page.setViewportSize(SIZES[2]);
      await settle(page);
      const edge = page.locator('nav.rail [role="separator"]');
      await edge.focus();
      await page.keyboard.press('End');
      await expect(edge).toHaveAttribute('aria-valuenow', '300');
      await send(page, 'plan superpowers');
      await expect(page.getByTestId('question-pending')).toBeVisible();

      const problems = await measureConversation(page, lang, [
        'nav.rail',
        'main.conv .composer',
        'main.conv .plan .head',
        '[data-testid="question-pending"]',
        'aside.git-side .utabs',
        'aside.git-side .side-top',
      ]);
      await setLayout(page, lang, 'panel');
      for (let width = 1000; width <= 1960; width += 32) {
        await page.setViewportSize({ width, height: 800 });
        await settle(page);
        const rail = await widthOf(page.locator('nav.rail'));
        if (rail !== Math.max(52, Math.min(300, width - 948))) problems.push(`${lang} ${width} px: the rail is ${rail} px`);
        problems.push(...(await headerProblems(page, `${lang} ${width} px, panel, rail ${rail} px`)));
      }
      expect(problems).toEqual([]);
    });
  });
