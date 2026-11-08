// Réglages › Projets › Kanban › Workflow (ESC-195): the reading of the frozen copy of GSD (tests/fixtures/workflows/gsd, its
// commands, subagents and included workflows in Principal's Claude Code folder, its .planning in the project), its
// sources, « Relire », the workflows found and the steps, kept in .escouade/workflows/ of the repository and never
// committed; a step renamed, saved, and found again after a restart. In the
// runner's window (1024 × 728), in both themes, in French and in English; compared to the preview
// (design/app-redesign/index.html, `#loops-settings` and `#loops-settings-steps`) in both themes. The captures of the
// app and of the preview go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const GSD = path.join(ROOT, 'tests', 'fixtures', 'workflows', 'gsd');
const PREVIEW = path.join(ROOT, 'design', 'app-redesign', 'index.html');
/** The preview loads Inter from the site's packages: without them, its text is not the app's. */
const PREVIEW_FONT = path.join(ROOT, 'website', 'node_modules', '@fontsource-variable', 'inter', 'opsz.css');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZE = { width: 1024, height: 728 };
/** The colour both are tinted with: the preview's first project's. */
const TINT = 'oklch(0.72 0.12 48)';

const WORDS = {
  fr: {
    settings: 'Réglages (Ctrl+,)',
    dialog: 'Réglages',
    page: 'Kanban',
    readAt: /^Lu par les règles à /,
    reread: 'Relire',
    gsd: 'Corriger les étapes de GSD',
    counts: '4 étapes',
    fixed: 'corrigé',
    step: (n: number) => `Nom de l’étape ${n}`,
    renamed: 'Discuter',
    repo: 'Dans le dépôt',
    dev: 'Corriger les étapes de plan',
    local: 'sur cette machine (hors du dépôt)',
    inRepo: 'dans le dépôt',
  },
  en: {
    settings: 'Settings (Ctrl+,)',
    dialog: 'Settings',
    page: 'Kanban',
    readAt: /^Read by the rules at /,
    reread: 'Read again',
    gsd: 'Correct the steps of GSD',
    counts: '4 steps',
    fixed: 'corrected',
    step: (n: number) => `Name of step ${n}`,
    renamed: 'Discuss',
    repo: 'In the repository',
    dev: 'Correct the steps of plan',
    local: 'on this machine (outside the repository)',
    inRepo: 'in the repository',
  },
} as const;

/** The look of an element, as the browser computes it: what the app and the preview must share. */
function look(el: Locator) {
  return el.evaluate((node) => {
    const s = getComputedStyle(node);
    const r = node.getBoundingClientRect();
    return {
      width: Math.round(r.width),
      height: Math.round(r.height),
      background: s.backgroundColor,
      backgroundImage: s.backgroundImage,
      color: s.color,
      shadow: s.boxShadow,
      radius: s.borderRadius,
      padding: s.padding,
      gap: s.gap,
      columns: s.display === 'grid' ? s.gridTemplateColumns.split(' ').length : 0,
      font: `${s.fontWeight} ${s.fontSize} ${s.fontFamily.split(',')[0]}`,
    };
  });
}

const lookBut = (el: Locator, ...keys: string[]) =>
  look(el).then((l) => Object.fromEntries(Object.entries(l).filter(([k]) => !keys.includes(k))));

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

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function previewAt(preview: Page, route: string, t: 'dark' | 'light') {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await still(preview);
}

/** What the section has, the app's beside the preview's: [what, the app's selector, the preview's, the keys left out]. */
const PARTS: [string, string, string, string[]][] = [
  ['a source read', '.wf-src .gate', '.wf-src .gate', ['width']],
  ['its count', '.wf-src .gate .n', '.wf-src .gate .n', ['width']],
  ['« Relire »', '.sr .ctl .btn', '.sr .ctl .btn', ['width']],
  ['a title in the group', '.sub-h b', '.sub-h b', ['width']],
  ['what it says', '.sub-h span', '.sub-h span', ['width', 'height']],
  ['a workflow found', '.rrow.ok', '.rrow.ok', ['width']],
  ['its name', '.rrow.ok .rn', '.rrow.ok .rn', ['width']],
  ['its counts', '.rrow.ok .rb', '.rrow.ok .rb', ['width']],
  ['its state', '.rrow.ok .st', '.rrow.ok .st', ['width']],
  ['a command alone', '.rrow.idle', '.rrow.idle', ['width']],
  ['a step', '.wf-er:not(.loop):not(.guess)', '.wf-er:not(.loop):not(.guess)', ['width']],
  ['its number', '.wf-er .no', '.wf-er .no', ['width']],
  ['its name', '.wf-er .inp', '.wf-er .inp', ['width']],
  ['what tells it', '.wf-er .by .gate', '.wf-er .by .gate', ['width']],
  ['« Fusionner »', '.wf-er .acts .icon-btn', '.wf-er .acts .icon-btn', []],
  ['its ways back', '.wf-er.loop', '.wf-er.loop', ['width', 'height']],
  ['« Ajouter une étape »', '.add-line', '.add-line', ['width']],
];

async function compare(section: Locator, preview: Page, where: string) {
  const psection = preview.locator('#sg-workflow');
  for (const [what, sel, psel, skip] of PARTS) {
    const a = section.locator(sel).first();
    const p = psection.locator(psel).first();
    await expect(a, `${what}, ${where}`).toBeVisible();
    expect.soft(await lookBut(a, ...skip), `${what}, ${where}`).toEqual(await lookBut(p, ...skip));
  }
}

for (const lang of ['fr', 'en'] as const) {
  test.describe(lang, () => {
    test.use({
      language: lang,
      // GSD in Principal's Claude Code folder (the app's CLAUDE_CONFIG_DIR is the test's `claude`).
      preStart: { run: (root: string) => fs.cpSync(path.join(GSD, 'home', '.claude'), path.join(root, 'claude'), { recursive: true }) },
    });

    test(`the workflow read, corrected and found again (${lang})`, async ({ app }) => {
      test.setTimeout(240_000);
      const w = WORDS[lang];
      let page = app.page;
      await page.setViewportSize(SIZE);
      // The project has GSD's trace, and a command of its own that is no workflow.
      fs.cpSync(path.join(GSD, 'project', '.planning'), path.join(app.repo, '.planning'), { recursive: true });
      fs.cpSync(path.join(GSD, '..', 'none', 'project', '.claude'), path.join(app.repo, '.claude'), { recursive: true });
      // And skills of its own: /dev runs the plan, the implementation and the review.
      fs.cpSync(path.join(GSD, '..', 'dev', 'project', '.claude'), path.join(app.repo, '.claude'), { recursive: true });
      for (const args of [
        ['add', '-A'],
        ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qm', 'gsd'],
      ])
        execFileSync('git', args, { cwd: app.repo, stdio: 'pipe' });
      await addProject(page, app.repo, { lang });

      const open = async () => {
        await page.getByRole('button', { name: w.settings }).click();
        const dialog = page.getByRole('dialog', { name: w.dialog });
        await openSettingsPage(dialog, w.page);
        const section = dialog.locator('#sg-workflow');
        await section.scrollIntoViewIfNeeded();
        return { dialog, section };
      };
      let { dialog, section } = await open();
      await expect(section.getByText(w.readAt)).toBeVisible();
      await expect(section.locator('.wf-src .gate', { hasText: '~/.claude/commands/gsd' })).toBeVisible();
      await expect(section.locator('.wf-src .gate', { hasText: '.planning/STATE.md' })).toBeVisible();
      const gsd = section.getByRole('button', { name: w.gsd });
      await expect(gsd).toContainText(w.counts);
      await expect(section.getByRole('textbox', { name: w.step(1) })).toHaveValue('gsd:discuss-phase');
      await expect(section.locator('.wf-er .by').nth(1)).toContainText('gsd-planner');
      await section.getByRole('button', { name: w.reread }).click();
      await expect(section.getByRole('button', { name: w.reread })).toBeEnabled();
      await expect(section.getByRole('radio', { name: w.repo })).toHaveAttribute('aria-checked', 'true');
      // The repository's own workflow (its /dev skill) is kept in it, for the team: written, never committed.
      // GSD, of Principal's own folder, stays on this machine, never in the repository.
      const shared = path.join(app.repo, '.escouade', 'workflows', 'claude-plan.yml');
      expect(fs.existsSync(shared)).toBe(true);
      expect(fs.readFileSync(shared, 'utf8')).toContain('name: implement');
      expect(fs.existsSync(path.join(app.repo, '.escouade', 'workflows', 'claude-gsd.yml'))).toBe(false);
      expect(execFileSync('git', ['log', '--oneline'], { cwd: app.repo, encoding: 'utf8' }).trim().split(/\r?\n/)).toHaveLength(2);
      await expect(section.getByRole('button', { name: w.gsd })).toContainText(w.local);
      await expect(section.getByRole('button', { name: w.dev })).toContainText(w.inRepo);

      // In both themes, nothing out of the window; in French, as the preview draws it.
      const exe = chromium.executablePath();
      const withPreview = lang === 'fr' && fs.existsSync(exe) && fs.existsSync(PREVIEW_FONT);
      const browser = withPreview ? await chromium.launch() : null;
      const preview = browser ? await browser.newPage() : null;
      try {
        await preview?.setViewportSize(SIZE);
        for (const t of ['dark', 'light'] as const) {
          await theme(page, t);
          await still(page);
          await page.mouse.move(2, 400);
          await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
          const out = await section.evaluate((s) => {
            const body = s.closest('.s-body')!.getBoundingClientRect();
            return [...s.querySelectorAll('*')]
              .filter((el) => {
                const r = el.getBoundingClientRect();
                return r.width > 0 && (r.left < body.left - 1 || r.right > body.right + 1);
              })
              .map((el) => el.className);
          });
          expect.soft(out, `nothing out of the settings at ${SIZE.width} px, ${t}`).toEqual([]);
          if (SHOTS) {
            fs.mkdirSync(SHOTS, { recursive: true });
            await page.screenshot({ path: path.join(SHOTS, `app-workflow-settings-${lang}-${t}.png`) });
            await section.locator('.wf-edit').scrollIntoViewIfNeeded();
            await page.screenshot({ path: path.join(SHOTS, `app-workflow-settings-steps-${lang}-${t}.png`) });
            await section.scrollIntoViewIfNeeded();
          }
          if (preview) {
            await previewAt(preview, 'loops-settings-steps', t);
            if (SHOTS) await preview.screenshot({ path: path.join(SHOTS, `preview-loops-settings-steps-${t}.png`) });
            await compare(section, preview, `${t} ${SIZE.width}×${SIZE.height}`);
          }
        }
      } finally {
        await browser?.close();
      }

      // A step renamed: saved with the project, read again ahead of the reading.
      await theme(page, 'dark');
      const first = section.getByRole('textbox', { name: w.step(1) });
      await first.fill(w.renamed);
      await expect(gsd).toContainText(w.fixed);
      await closeSettings(dialog);
      page = await app.restart();
      await page.setViewportSize(SIZE);
      ({ dialog, section } = await open());
      await expect(section.getByRole('textbox', { name: w.step(1) })).toHaveValue(w.renamed);
      await expect(section.getByRole('button', { name: w.gsd })).toContainText(w.fixed);
      await expect(section.getByRole('textbox', { name: w.step(2) })).toHaveValue('gsd:plan-phase');
      // The correction is in the file of the workflow, beside what was read: GSD's, on this machine.
      const gsdFile = path.join(app.data, 'projects');
      const found = fs
        .readdirSync(gsdFile)
        .map((id) => path.join(gsdFile, id, 'workflows', 'claude-gsd.yml'))
        .find((p) => fs.existsSync(p));
      expect(found).toBeTruthy();
      expect(fs.readFileSync(found!, 'utf8')).toContain(`name: ${w.renamed}`);
    });
  });
}
