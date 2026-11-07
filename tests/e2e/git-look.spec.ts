// Git beside the conversation, in the 2.0 design (ESC-79, lot 7): underlined tabs « Non commités », « Historique » and the
// place of « PR et CI » after the 2.0; the scope « Cet agent | Tout le projet », the files with their letter and their
// counts, the diff in a well, « Voir le diff », « Commit… » and the merge; the history as a graph of the agents' branches.
// Compared to the preview (design/app-redesign/index.html, `#git`, `#history`, `#hosts`) in both themes at 1440 × 900
// and at CI's 1028 × 779; the English labels in their room; the motion stopped by prefers-reduced-motion. The captures of
// the app and of the preview go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

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
      border: `${s.borderBottomWidth} ${s.borderBottomStyle} ${s.borderBottomColor}`,
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

/** Tints a page with `TINT` at once, nothing gliding, for both to be read alike. */
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

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', ...args], { cwd, encoding: 'utf8' }).trim();

/**
 * An agent in its worktree as the preview's: two commits on its branch, a file modified (with lines removed and added)
 * and one added, uncommitted; main has gone on meanwhile. Returns the worktree's folder.
 */
async function agentAtWork(repo: string) {
  const worktrees = path.join(repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0)).toBe(1);
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  const write = (file: string, text: string) => {
    fs.mkdirSync(path.dirname(path.join(wt, file)), { recursive: true });
    fs.writeFileSync(path.join(wt, file), text);
  };
  write('src/routes/users.ts', "router.get('/users', async (req) => {\n  const users = await db.users.all();\n  return users;\n});\n");
  git(wt, 'add', '-A');
  git(wt, 'commit', '-qm', 'Routes /users : tests de base');
  write('src/db/cursor.ts', 'export const cursor = (id: string) => id;\n');
  git(wt, 'add', '-A');
  git(wt, 'commit', '-qm', 'paginate() : curseur opaque');
  write(
    'src/routes/users.ts',
    "router.get('/users', async (req) => {\n  const { limit = 50, cursor } = req.query;\n  const page = await paginate(db.users, { limit, cursor });\n  return { users: page.items, next: page.next };\n});\n",
  );
  write('src/db/paginate.ts', 'export async function paginate() {}\n');
  fs.writeFileSync(path.join(repo, 'src', 'app.ts'), 'const a = 2;\n');
  git(repo, 'commit', '-qam', 'Webhooks Stripe (#41)');
  return wt;
}

/** The preview at `#route`, in theme `t`: from a blank page, for its script to open the route. */
async function previewAt(preview: Page, route: string, t: 'dark' | 'light') {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await still(preview);
}

test('git beside the conversation looks as the preview draws it, in both themes, at both sizes', async ({ app }) => {
  test.setTimeout(240_000);
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true });
  await agentAtWork(app.repo);
  await page.getByRole('button', { name: 'Conversation et fichiers côte à côte' }).click();
  const side = page.locator('aside.git-side.docked');
  await expect(side.locator('.file')).toHaveCount(2);
  await expect(side.getByRole('region', { name: /^Diff de / })).toContainText('paginate');
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await theme(page, t);
        await still(page);
        await page.evaluate(() => document.fonts.ready);
        const where = `${t} ${size.width}×${size.height}`;

        // « Non commités » (`#git`).
        await side.getByRole('tab', { name: /^Non commités/ }).click();
        await page.mouse.move(size.width / 2, 20);
        await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
        await previewAt(preview, 'git', t);
        const pv = preview.locator('#side');
        expect.soft(await place(side), `panel's place, ${where}`).toEqual(await place(pv));
        expect.soft(await lookBut(side, 'width', 'height'), `panel, ${where}`).toEqual(await lookBut(pv, 'width', 'height'));

        // The tabs, underlined.
        const tabs = side.locator('.utabs');
        const ptabs = pv.locator('.utabs');
        expect.soft(await lookBut(tabs, 'width'), `tabs, ${where}`).toEqual(await lookBut(ptabs, 'width'));
        const tab = (el: Locator, sel: boolean) => el.locator(`[role="tab"][aria-selected="${sel}"]`).first();
        expect.soft(await lookBut(tab(tabs, true), 'width'), `the tab chosen, ${where}`).toEqual(await lookBut(tab(ptabs, true), 'width'));
        expect.soft(await lookBut(tab(tabs, false), 'width'), `a tab, ${where}`).toEqual(await lookBut(tab(ptabs, false), 'width'));
        const [tx, , , th] = await place(tab(tabs, true));
        const [ptx, , , pth] = await place(tab(ptabs, true));
        expect.soft([tx, th], `the first tab's place, ${where}`).toEqual([ptx, pth]);
        expect.soft(await lookBut(tabs.locator('.n'), 'width'), `the count, ${where}`).toEqual(await lookBut(ptabs.locator('.n'), 'width'));

        // The scope and the caption.
        const seg = side.locator('.scope');
        const pseg = pv.locator('#side-files .seg');
        expect.soft(await place(seg), `scope's place, ${where}`).toEqual(await place(pseg));
        expect.soft(await look(seg), `scope, ${where}`).toEqual(await look(pseg));
        expect
          .soft(await look(seg.locator('[aria-pressed="true"]')), `the scope chosen, ${where}`)
          .toEqual(await look(pseg.locator('[aria-selected="true"]')));
        const caption = side.locator('.files .caption');
        const pcaption = pv.locator('#side-files .caption');
        expect.soft(await lookBut(caption, 'width'), `caption, ${where}`).toEqual(await lookBut(pcaption, 'width'));
        expect.soft(await place(caption), `caption's place, ${where}`).toEqual(await place(pcaption));

        // The files: the one shown, another, their letters, names and counts.
        const file = side.locator('.file[aria-current="true"]');
        const pfile = pv.locator('#side-files .file[aria-current="true"]');
        expect.soft(await place(file), `the file shown, its place, ${where}`).toEqual(await place(pfile));
        expect.soft(await look(file), `the file shown, ${where}`).toEqual(await look(pfile));
        const other = side.locator('.file:not([aria-current])').first();
        expect
          .soft(await look(other), `another file, ${where}`)
          .toEqual(await look(pv.locator('#side-files .file:not([aria-current])').first()));
        for (const l of ['m', 'a'])
          expect
            .soft(await lookBut(side.locator(`.letter.${l}`).first(), 'width'), `letter ${l.toUpperCase()}, ${where}`)
            .toEqual(await lookBut(pv.locator(`#side-files .letter.${l}`).first(), 'width'));
        expect
          .soft(await lookBut(file.locator('.fn b'), 'width'), `a file's name, ${where}`)
          .toEqual(await lookBut(pfile.locator('.fn b'), 'width'));
        expect
          .soft(await lookBut(file.locator('.fn .dir'), 'width'), `its folder, ${where}`)
          .toEqual(await lookBut(pfile.locator('.fn span'), 'width'));
        for (const c of ['add', 'del'])
          expect
            .soft(await lookBut(file.locator(`.cnt .${c}`), 'width'), `its count ${c}, ${where}`)
            .toEqual(await lookBut(pfile.locator(`.cnt .${c}`), 'width'));

        // The diff, in a well; its lines added and removed.
        const well = side.locator('.side-diff .code');
        const pwell = pv.locator('.side-diff .code');
        const [wx, , ww] = await place(well);
        const [pwx, , pww] = await place(pwell);
        expect.soft([wx, ww], `the well's place, ${where}`).toEqual([pwx, pww]);
        expect.soft(await lookBut(well, 'width', 'height'), `the well, ${where}`).toEqual(await lookBut(pwell, 'width', 'height'));
        for (const [k, pk] of [
          ['add', 'a'],
          ['del', 'd'],
        ])
          expect
            .soft(await lookBut(well.locator(`.urow.${k}`).first(), 'width'), `a line ${k}, ${where}`)
            .toEqual(await lookBut(pwell.locator(`.ln.${pk}`).first(), 'width'));
        // Their number and their sign (the preview's are inline, as high as their text: the app's fill the line).
        expect
          .soft(await lookBut(well.locator('.urow .no').first(), 'width', 'height'), `a line's number, ${where}`)
          .toEqual(await lookBut(pwell.locator('.ln .lno').first(), 'width', 'height'));
        expect
          .soft(await lookBut(well.locator('.urow.add .sign').first(), 'width', 'height'), `a line's sign, ${where}`)
          .toEqual(await lookBut(pwell.locator('.ln.a .sig').first(), 'width', 'height'));
        const [nx] = await place(well.locator('.urow .no').first());
        const [pnx] = await place(pwell.locator('.ln .lno').first());
        expect.soft(nx, `a line's number, its place, ${where}`).toBe(pnx);

        // The foot: « Voir le diff », « Commit… », the merge; from the panel's bottom.
        const foot = side.locator('.foot');
        const pfoot = pv.locator('.side-foot2');
        const [fx, fy, fw, fh] = await place(foot);
        const [pfx, pfy, pfw, pfh] = await place(pfoot);
        expect.soft([fx, fy + fh, fw, fh], `foot's place, ${where}`).toEqual([pfx, pfy + pfh, pfw, pfh]);
        expect.soft(await lookBut(foot, 'width', 'height'), `foot, ${where}`).toEqual(await lookBut(pfoot, 'width', 'height'));
        const btn = (el: Locator, sel: string) => el.locator(sel).first();
        expect
          .soft(await lookBut(btn(foot, '.rowb .btn:not(.primary)'), 'width'), `« Voir le diff », ${where}`)
          .toEqual(await lookBut(btn(pfoot, '.rowb .btn:not(.primary)'), 'width'));
        expect
          .soft(await lookBut(btn(foot, '.btn.primary'), 'width'), `« Commit… », ${where}`)
          .toEqual(await lookBut(btn(pfoot, '.btn.primary'), 'width'));
        const [cx, , cw] = await place(btn(foot, '.btn.primary'));
        const [pcx, , pcw] = await place(btn(pfoot, '.btn.primary'));
        expect.soft(cx + cw, `« Commit… »'s right edge, ${where}`).toBe(pcx + pcw);
        expect
          .soft(await lookBut(btn(foot, '.btn.wide'), 'width'), `the merge, ${where}`)
          .toEqual(await lookBut(btn(pfoot, '.btn.wide'), 'width'));
        expect
          .soft(await lookBut(btn(foot, '.btn.wide b'), 'width'), `the merge's branch, ${where}`)
          .toEqual(await lookBut(btn(pfoot, '.btn.wide b'), 'width'));
        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `git${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }

        // « Historique » (`#history`): the caption, the commits on their lanes, the branches on them, who and when.
        await side.getByRole('tab', { name: 'Historique' }).click();
        await page.mouse.move(size.width / 2, 20);
        await previewAt(preview, 'history', t);
        const graph = side.locator('.graph');
        const pgraph = pv.locator('#side-history .graph');
        await expect(graph.locator('.commit')).toHaveCount(4);
        const hcap = side.locator('.caption');
        const phcap = pv.locator('#side-history .caption');
        expect.soft(await lookBut(hcap, 'width'), `history's caption, ${where}`).toEqual(await lookBut(phcap, 'width'));
        expect.soft(await place(hcap), `its place, ${where}`).toEqual(await place(phcap));
        const commit = graph.locator('.commit').first();
        const pcommit = pgraph.locator('.commit').first();
        expect.soft(await place(commit), `the first commit's place, ${where}`).toEqual(await place(pcommit));
        expect.soft(await look(commit), `a commit, ${where}`).toEqual(await look(pcommit));
        expect
          .soft(await lookBut(commit.locator('.subject'), 'width'), `its message, ${where}`)
          .toEqual(await lookBut(pcommit.locator('.msg span:first-child'), 'width'));
        expect
          .soft(await lookBut(commit.locator('.who'), 'width'), `who and when, ${where}`)
          .toEqual(await lookBut(pcommit.locator('.who'), 'width'));
        const ref = graph.locator('.ref').first();
        const pref = pgraph.locator('.ref').first();
        expect
          .soft(await lookBut(ref, 'width', 'shadow', 'background'), `a branch on it, ${where}`)
          .toEqual(await lookBut(pref, 'width', 'shadow', 'background'));
        // The dot of a commit: its size and its ring.
        const dot = (el: Locator) =>
          el
            .locator('circle')
            .first()
            .evaluate((c) => {
              const s = getComputedStyle(c);
              return [c.getAttribute('r'), s.strokeWidth, s.stroke];
            });
        expect.soft(await dot(commit), `a commit's dot, ${where}`).toEqual(await dot(pv.locator('#side-history svg.lanes')));
        if (SHOTS) {
          const n = `history${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }

        // « PR et CI »: in the strip, gone under 420 px for the × (choice 189). The tab is the PR's own view since
        // ESC-305, no longer the preview's placeholder (`#hosts`, its « dans la 2.1 » and its `.later`): not compared.
        const hosts = side.getByRole('tab', { name: /^PR et CI/ });
        expect.soft(await hosts.isVisible(), `« PR et CI » in the strip, ${where}`).toBe(size.width > 1100);
      }
  } finally {
    await browser.close();
  }
});

test('the diff window and the branch picker open still with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true });
  await agentAtWork(app.repo);
  await page.getByRole('button', { name: 'Conversation et fichiers côte à côte' }).click();
  const side = page.locator('aside.git-side.docked');
  await expect(side.locator('.file')).toHaveCount(2);
  const motion = (el: Locator) => el.evaluate((e) => getComputedStyle(e).animationName);
  await side.getByRole('button', { name: 'Voir le diff' }).click();
  const win = page.getByRole('dialog', { name: /^Modifications de / });
  await expect(win).toBeVisible();
  // Its keyframes, named by Svelte after the component.
  expect(await motion(win)).toMatch(/rise/);
  await page.keyboard.press('Escape');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await side.getByRole('button', { name: 'Voir le diff' }).click();
  await expect(win).toBeVisible();
  expect(await motion(win)).toBe('none');
  expect(await motion(page.locator('.scrim'))).toBe('none');
  // Its files lettered, drawn and counted, and its close button an icon.
  await expect(win.locator('.file .letter')).toHaveText(['M', 'A']);
  await expect(win.getByRole('button', { name: 'Fermer' }).locator('svg[data-icon="close"]')).toBeVisible();
  await page.keyboard.press('Escape');
  // The agent shown works in its worktree: the button opens its picker, which opens the project's folder's.
  await page.locator('footer .sync').click();
  const own = page.getByRole('dialog', { name: /^Branche de / });
  await expect(own).toBeVisible();
  expect(await motion(own)).toBe('none');
  await own.getByRole('button', { name: 'Dossier du projet…' }).click();
  const picker = page.getByRole('dialog', { name: 'Branches' });
  await expect(picker).toBeVisible();
  expect(await motion(picker)).toBe('none');
  await expect(picker.getByRole('option', { name: /^main/ }).locator('[data-icon="check"]')).toBeVisible();
  await page.emulateMedia({ reducedMotion: 'no-preference' });
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the panel fit in their place, at both sizes', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { worktrees: true, lang: 'en' });
    await agentAtWork(app.repo);
    await page.getByRole('button', { name: 'Conversation and files side by side' }).click();
    const side = page.locator('aside.git-side.docked');
    await expect(side.locator('.file')).toHaveCount(2);
    for (const size of SIZES) {
      await page.setViewportSize(size);
      for (const tab of [/^Uncommitted/, /^History/, /^PR and CI/]) {
        // Under 420 px, « PR and CI » leaves the strip for the × (choice 189).
        if (!(await side.getByRole('tab', { name: tab }).isVisible())) continue;
        await side.getByRole('tab', { name: tab }).click();
        await page.mouse.move(size.width / 2, 20);
        // What is cut short, or overflows its box: none of it, but the names of the user's files, branches, commits and
        // folders, cut by design.
        const cut = await side.evaluate((root) =>
          [...root.querySelectorAll<HTMLElement>('*')]
            .filter(
              (el) =>
                el.offsetParent &&
                el.children.length === 0 &&
                !el.closest('.caption, .fn, .ell, .subject, .ref, .who, .code, .graph svg') &&
                (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
            )
            .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
        );
        expect(cut, `${size.width}×${size.height} ${tab}`).toEqual([]);
        // The tabs hold on one line.
        for (const b of await side.locator('.utabs [role="tab"]:visible').all())
          expect((await b.boundingBox())!.height).toBeLessThanOrEqual(41);
        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          await page.screenshot({ path: path.join(SHOTS, `git-${tab.source.replace(/\W/g, '')}${size.name}-en.app.png`) });
        }
      }
      await side.getByRole('tab', { name: /^Uncommitted/ }).click();
      await expect(side.getByRole('button', { name: 'Show diff' })).toBeVisible();
      await expect(side.getByRole('button', { name: 'Commit…' })).toBeVisible();
      await expect(side.getByRole('button', { name: /^Merge .* → main…$/ })).toBeVisible();
    }
  });
});
