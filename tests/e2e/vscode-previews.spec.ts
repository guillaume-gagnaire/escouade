// The previews of the VS Code view (ESC-473), one test per family: an image, a Markdown file (its image of the web not
// loaded), a PDF, a Word file; each opened from the explorer and from a link of a conversation, the trapped files doing
// nothing (a script in an SVG, in a Markdown file, in a Word file). ESCOUADE_E2E_SHOTS=<folder>: a capture of each
// preview, in the light and the dark theme (the test « the previews, light and dark »).

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { docx, pdf, png, TRAPPED_MD, TRAPPED_SVG, wav } from './preview-files';
import { editorLines, explorerRow, unfold, vscodeTabs, workbench } from './vscode';

test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

function writeProject(repo: string) {
  const write = (file: string, data: string | Buffer) => {
    fs.mkdirSync(path.dirname(path.join(repo, file)), { recursive: true });
    fs.writeFileSync(path.join(repo, file), data);
  };
  write('src/app.ts', 'export const answer = 42;\nexport const second = 2;\n');
  write('assets/photo.png', png(64, 48));
  write('assets/piege.svg', TRAPPED_SVG);
  write('assets/son.wav', wav(2));
  write('docs/guide.md', TRAPPED_MD);
  write('docs/rapport.pdf', pdf(3));
  write('docs/rapport.docx', docx());
  write('docs/slides.pptx', Buffer.from('PK\u0003\u0004 pas un vrai pptx'));
  write('data/table.csv', 'nom;valeur\n"a; b";1\nc;2\n');
  const git = (...args: string[]) => execFileSync('git', args, { cwd: repo });
  git('add', '-A');
  git('commit', '-qm', 'Projet de test');
}

/** What the page tried to load of the web, and its errors: none may name an address of the traps. */
function watch(page: Page) {
  const out: string[] = [];
  const errors: string[] = [];
  page.on('request', (r) => out.push(r.url()));
  page.on('console', (m) => {
    if (m.type() === 'error') out.push(m.text());
  });
  page.on('pageerror', (e) => {
    errors.push(String(e));
    console.log(`pageerror: ${e.stack ?? e}`.slice(0, 1500));
  });
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') console.log(`console.${m.type()}: ${m.text().slice(0, 400)}`);
  });
  return {
    /** Nothing was loaded from the web, nothing was even tried (the policy of the window would have refused it). */
    nothingRemote: () => expect(out.filter((u) => /tracker\.invalid|example\.invalid/.test(u))).toEqual([]),
    // VS Code's own race of its explorer, let through by the view's e2e too (vscode-view.spec.ts).
    noErrors: () => expect(errors.filter((e) => e !== 'TypeError: e.getId is not a function')).toEqual([]),
  };
}

/** No script of a document ran: the window's title (which VS Code sets to its tab's) has no mark of a trap, its globals none. */
async function noScriptRan(page: Page) {
  expect(await page.title()).not.toMatch(/pwned/);
  expect(await page.evaluate(() => (window as unknown as { __pwned?: boolean }).__pwned)).toBeUndefined();
}

/** The preview of a family shown in the view (the one of the active group). */
const preview = (page: Page, kind: string) =>
  workbench(page).locator(`.escouade-preview[data-kind="${kind}"]`).locator('visible=true').first();

async function start(page: Page, repo: string) {
  writeProject(repo);
  await addProject(page, repo);
  await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });
  // What the policy of the window refused, said with where it comes from.
  await page.evaluate(() =>
    document.addEventListener('securitypolicyviolation', (e) =>
      console.warn(
        `csp: ${e.violatedDirective} ${e.blockedURI} ${e.sample} at ${e.sourceFile}:${e.lineNumber} in ${(e.target as Element | null)?.outerHTML?.slice(0, 200) ?? String(e.target)}`,
      ),
    ),
  );
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  await expect(workbench(page)).toBeVisible({ timeout: 60_000 });
}

/** The agent edits `rel` (`lien-apercu`), and the link of its tool is clicked: the file opens in the view. */
async function openFromConversation(page: Page, rel: string) {
  await page.getByTestId('vscode-back').click();
  // The conversation's field (the view's editors, hidden, are text boxes too).
  const box = page.locator('.composer textarea');
  await box.fill(`lien-apercu ${rel} ligne 3`);
  await box.press('Enter');
  const name = rel
    .split('/')
    .pop()!
    .replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const link = page.getByRole('button', { name: new RegExp(`^Ouvrir .*${name} dans l’éditeur$`) }).last();
  await expect(link).toBeVisible({ timeout: 30_000 });
  await link.click();
  await expect(workbench(page)).toBeVisible();
}

/** « Ouvrir un fichier » (Ctrl+P) from the view, on `query`. */
async function openByPalette(page: Page, query: string) {
  await workbench(page).locator('.explorer-folders-view').click();
  await page.keyboard.press('Control+P');
  const palette = page.getByRole('dialog', { name: 'Ouvrir un fichier' });
  await expect(palette).toBeVisible();
  await page.keyboard.type(query);
  await expect(palette.getByRole('option', { name: new RegExp(query.replace('.', '\\.')) }).first()).toBeVisible();
  await page.keyboard.press('Enter');
  await expect(palette).toBeHidden();
}

/** Opens `file` from the explorer: its folders unfolded, its row clicked, its tab (`tab`, its name by default) shown. */
async function openInExplorer(page: Page, file: string, tab = file.split('/').pop()!) {
  const parts = file.split('/');
  for (const dir of parts.slice(0, -1)) await unfold(explorerRow(page, dir));
  await explorerRow(page, parts.at(-1)!).click();
  await expect.poll(() => vscodeTabs(page)).toContain(tab);
}

/** Closes the tab `name` by VS Code’s Ctrl+W. */
async function closeTab(page: Page, name: string) {
  await page.locator('#vscode-layer .tabs-container .tab', { hasText: name }).first().click();
  await page.keyboard.press('Control+W');
  await expect.poll(() => vscodeTabs(page)).not.toContain(name);
}

async function shot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.screenshot({ path: path.join(SHOTS, `${name}.png`) });
}

test('an image opens in its preview: from the explorer, Ctrl+P and a conversation; an SVG with a script does nothing', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const w = watch(page);
  await start(page, app.repo);

  // From the explorer: the image, its size, its zoom; not VS Code's warning of a binary file.
  await openInExplorer(page, 'assets/photo.png');
  const img = preview(page, 'image');
  await expect(img.locator('img.pv-image')).toBeVisible();
  await expect(img.locator('.pv-info')).toContainText('64 × 48 px');
  await expect(workbench(page).getByText(/c’est un fichier binaire/)).toHaveCount(0);
  expect(await img.locator('img.pv-image').evaluate((i: HTMLImageElement) => i.src.startsWith('blob:') && i.naturalWidth)).toBe(64);
  // « Taille réelle », then a step of zoom: 100 %, 125 %.
  await img.getByRole('button', { name: 'Taille réelle' }).click();
  await expect(img.locator('.pv-zoom')).toHaveText('100 %');
  await img.getByRole('button', { name: 'Zoom avant' }).click();
  await expect(img.locator('.pv-zoom')).toHaveText('125 %');
  expect(await img.locator('img.pv-image').evaluate((i) => (i as HTMLElement).style.width)).toBe('80px');

  // By Ctrl+P: the trapped SVG, as an image (an `<img>` of it), never in the page.
  await openByPalette(page, 'piege.svg');
  await expect.poll(() => vscodeTabs(page)).toContain('piege.svg');
  const svg = preview(page, 'image');
  await expect(svg.locator('.pv-info')).toContainText('120 × 80 px');
  await expect(workbench(page).locator('.escouade-preview svg')).toHaveCount(0);

  // From a link of a conversation: the same preview.
  await closeTab(page, 'piege.svg');
  await openFromConversation(page, 'assets/piege.svg');
  await expect.poll(() => vscodeTabs(page)).toContain('piege.svg');
  await expect(preview(page, 'image').locator('.pv-info')).toContainText('120 × 80 px');

  // A sound: its player, read by the webview, never played by itself.
  await openInExplorer(page, 'assets/son.wav');
  const audio = preview(page, 'audio').locator('audio');
  await expect.poll(() => audio.evaluate((a: HTMLAudioElement) => a.duration)).toBeCloseTo(2, 1);
  expect(await audio.evaluate((a: HTMLAudioElement) => a.paused)).toBe(true);

  // A format without preview says so; « Ouvrir comme texte » is VS Code's text editor.
  await openInExplorer(page, 'docs/slides.pptx');
  const none = preview(page, 'none');
  await expect(none).toContainText('Pas d’aperçu pour les fichiers .pptx');
  await none.getByRole('button', { name: 'Ouvrir comme texte' }).click();
  await expect(none).toBeHidden();

  await page.waitForTimeout(500);
  await noScriptRan(page);
  w.nothingRemote();
  w.noErrors();
});

test('a Markdown file opens in its preview, its source beside: its typing and scroll followed, nothing of the web loaded, no script run', async ({
  app,
}) => {
  test.setTimeout(300_000);
  const { page } = app;
  const w = watch(page);
  await start(page, app.repo);

  await openInExplorer(page, 'docs/guide.md', 'Aperçu guide.md');
  const md = preview(page, 'markdown');
  await expect(md.locator('h1')).toHaveText('Guide de l’essai');
  await expect(md.locator('table td').first()).toHaveText('1');
  // The image of the web is not loaded: its text, its domain and « Afficher l'image », which a click alone would load.
  await expect(md.getByText('Le pixel de suivi · Image de tracker.invalid, non chargée')).toBeVisible();
  await expect(md.getByRole('button', { name: 'Afficher l’image' })).toBeVisible();
  // The image of the source is read by the core.
  const local = md.locator('img[alt="Le logo local"]');
  await expect(local).toHaveJSProperty('naturalWidth', 64);
  // The traps: no script, no style, nothing over the app.
  await expect(md.locator('script, style, [style*="fixed"]')).toHaveCount(0);
  await expect(page.locator('body')).toBeVisible();
  await shot(page, 'markdown');

  // « Source à côté »: the source in the group beside, the preview kept.
  await md.getByRole('button', { name: 'Source à côté' }).click();
  await expect.poll(() => vscodeTabs(page)).toEqual(expect.arrayContaining(['Aperçu guide.md', 'guide.md']));
  await expect(editorLines(page)).toContainText('Guide de l’essai');
  // Typing in the source: the preview is drawn again.
  await editorLines(page).click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.press('End');
  await page.keyboard.type(', modifié');
  await expect(md.locator('h1')).toHaveText('Guide de l’essai, modifié');
  // Scrolling the source to its end: the preview follows it.
  await page.keyboard.press('Control+End');
  await expect.poll(() => md.locator('.pv-scroll').evaluate((s) => s.scrollTop), { timeout: 10_000 }).toBeGreaterThan(200);
  // Saved: going back to the conversation asks nothing.
  await page.keyboard.press('Control+s');
  await expect.poll(() => vscodeTabs(page)).toContain('guide.md');

  // A link to a file of the project opens it in the view, at its line.
  await md.getByRole('link', { name: 'lien vers le code' }).click();
  await expect.poll(() => vscodeTabs(page)).toContain('app.ts');
  await expect(editorLines(page)).toContainText('export const answer = 42;');

  // From the source, « Ouvrir l’aperçu sur le côté » (Ctrl+K V) as in VS Code.
  await page
    .locator('#vscode-layer .tabs-container .tab', { hasText: /^guide\.md/ })
    .first()
    .click();
  await page.keyboard.press('Control+K');
  await page.keyboard.press('v');
  await expect(preview(page, 'markdown').locator('h1')).toBeVisible();

  // From a link of a conversation: the preview, brought to the line of the change.
  await openFromConversation(page, 'docs/guide.md');
  await expect(preview(page, 'markdown').locator('h1')).toBeVisible();

  await page.waitForTimeout(500);
  await noScriptRan(page);
  w.nothingRemote();
  w.noErrors();
});

test('a PDF is read in a tab: its pages, its zoom, its search, from the explorer and a conversation', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const w = watch(page);
  await start(page, app.repo);

  await openInExplorer(page, 'docs/rapport.pdf');
  const p = preview(page, 'pdf');
  await expect(p.locator('.pdfViewer .page')).toHaveCount(3, { timeout: 30_000 });
  await expect(p.locator('.pv-pages')).toContainText('sur 3');
  // Its text, selectable, in the text layer.
  await expect(p.locator('.textLayer').first()).toContainText('Escouade apercu', { timeout: 30_000 });
  // The zoom.
  const zoom = async () => Number((await p.locator('.pv-zoom').textContent())!.replace(/\D/g, ''));
  await expect.poll(zoom).toBeGreaterThan(0);
  const before = await zoom();
  await p.getByRole('button', { name: 'Zoom avant' }).click();
  await expect.poll(zoom).toBeGreaterThan(before);
  // The search, by Ctrl+F in the preview: three matches, one a page; Entrée the next, Maj+Entrée the one before.
  await p.locator('.pv-pdf-scroll').click();
  await page.keyboard.press('Control+F');
  await expect(p.locator('.pv-find')).toBeFocused();
  await page.keyboard.type('Escouade');
  await expect(p.locator('.pv-found')).toHaveText('1 sur 3', { timeout: 15_000 });
  await page.keyboard.press('Enter');
  await expect(p.locator('.pv-found')).toHaveText('2 sur 3');
  await page.keyboard.press('Shift+Enter');
  await expect(p.locator('.pv-found')).toHaveText('1 sur 3');
  // A page asked: shown.
  await p.locator('.pv-page-input').fill('3');
  await p.locator('.pv-page-input').press('Enter');
  await expect.poll(() => p.locator('.pv-pdf-scroll').evaluate((s) => s.scrollTop)).toBeGreaterThan(500);
  await shot(page, 'pdf');

  // From a link of a conversation.
  await closeTab(page, 'rapport.pdf');
  await openFromConversation(page, 'docs/rapport.pdf');
  await expect(preview(page, 'pdf').locator('.pdfViewer .page')).toHaveCount(3, { timeout: 30_000 });

  await noScriptRan(page);
  w.nothingRemote();
  w.noErrors();
});

test('a Word file is shown readable, in read only: its titles, list, table and image, a trapped link doing nothing', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const w = watch(page);
  await start(page, app.repo);

  await openInExplorer(page, 'docs/rapport.docx');
  const d = preview(page, 'docx');
  await expect(d.locator('h1')).toHaveText('Rapport d’essai', { timeout: 30_000 });
  await expect(d.locator('h2')).toHaveText('Les étapes');
  await expect(d.locator('ul li')).toHaveText(['Première étape', 'Deuxième étape']);
  await expect(d.locator('table tr')).toHaveCount(2);
  await expect(d.locator('img')).toHaveJSProperty('naturalWidth', 40);
  await expect(d.locator('.pv-info')).toHaveText('Lecture seule');
  // The link of the web is a link; the trapped one is text; the script written in it is text.
  await expect(d.getByRole('link', { name: 'le site' })).toHaveAttribute('href', 'https://example.invalid/');
  expect(await d.getByText('le lien piégé').evaluate((e) => e.closest('a')?.getAttribute('href') ?? null)).toBeNull();
  await expect(d).toContainText("<script>document.title='pwned'</script>");
  await expect(d.locator('script')).toHaveCount(0);
  await shot(page, 'docx');

  // From a link of a conversation.
  await closeTab(page, 'rapport.docx');
  await openFromConversation(page, 'docs/rapport.docx');
  await expect(preview(page, 'docx').locator('h1')).toHaveText('Rapport d’essai', { timeout: 30_000 });

  await noScriptRan(page);
  w.nothingRemote();
  w.noErrors();
});

test('the previews follow the theme and the tint of the project live, and speak Escouade’s language', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const w = watch(page);
  await start(page, app.repo);
  await page.evaluate(() => {
    document.documentElement.dataset.theme = 'light';
    document.documentElement.style.colorScheme = 'light';
  });
  await openInExplorer(page, 'docs/guide.md', 'Aperçu guide.md');
  const md = preview(page, 'markdown');
  await expect(md.locator('h1')).toBeVisible();
  // Its texts are Escouade's, in French.
  await expect(md.getByRole('button', { name: 'Source à côté' })).toBeVisible();
  const look = () =>
    md.evaluate((el) => ({
      background: getComputedStyle(el).backgroundColor,
      focus: getComputedStyle(el).getPropertyValue('--vscode-focusBorder').trim(),
    }));
  const light = await look();
  // The dark theme, the view open: the preview takes it, nothing reloaded.
  await page.evaluate(() => {
    document.documentElement.dataset.theme = 'dark';
    document.documentElement.style.colorScheme = 'dark';
  });
  await expect.poll(async () => (await look()).background, { timeout: 15_000 }).not.toBe(light.background);
  const dark = await look();
  // Another tint of the project: its accent follows.
  await page.evaluate(() => document.documentElement.style.setProperty('--p', 'oklch(0.62 0.2 145)'));
  await expect.poll(async () => (await look()).focus, { timeout: 15_000 }).not.toBe(dark.focus);
  await expect(md.locator('h1')).toBeVisible();
  // Its command, in VS Code's palette, by its French name.
  await openInExplorer(page, 'data/table.csv');
  await page.keyboard.press('Control+Shift+P');
  await page.keyboard.type('Ouvrir l’aperçu en tableau');
  await expect(workbench(page).locator('.quick-input-widget .monaco-list-row').first()).toContainText('Ouvrir l’aperçu en tableau');
  await page.keyboard.press('Enter');
  await expect(preview(page, 'csv').locator('.pv-info')).toHaveText('2 lignes');
  w.noErrors();
});

for (const scheme of ['light', 'dark'] as const) {
  test(`the previews, ${scheme === 'light' ? 'light' : 'dark'}: a capture of each`, async ({ app }) => {
    test.skip(!SHOTS, 'captures only (ESCOUADE_E2E_SHOTS)');
    test.setTimeout(300_000);
    const { page } = app;
    writeProject(app.repo);
    await addProject(page, app.repo);
    await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });
    // The theme, before the view starts: it reads it then (following it live is A4's).
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
      document.documentElement.style.colorScheme = t;
    }, scheme);
    await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
    await expect(workbench(page)).toBeVisible({ timeout: 60_000 });
    const cases: [string, string, () => Promise<void>][] = [
      ['assets/photo.png', 'image', () => expect(preview(page, 'image').locator('img.pv-image')).toBeVisible()],
      ['docs/guide.md', 'markdown', () => expect(preview(page, 'markdown').locator('h1')).toBeVisible()],
      ['docs/rapport.pdf', 'pdf', () => expect(preview(page, 'pdf').locator('.textLayer').first()).toContainText('Escouade')],
      ['docs/rapport.docx', 'docx', () => expect(preview(page, 'docx').locator('h1')).toBeVisible()],
      ['data/table.csv', 'csv', async () => {}],
    ];
    for (const [file, kind, ready] of cases) {
      await openInExplorer(page, file, kind === 'markdown' ? 'Aperçu guide.md' : undefined);
      if (kind === 'csv') {
        await page.keyboard.press('Control+Shift+P');
        await page.keyboard.type('Ouvrir l’aperçu en tableau');
        await page.keyboard.press('Enter');
        await expect(preview(page, 'csv').locator('table td').first()).toHaveText('a; b');
      }
      await ready();
      await page.waitForTimeout(400);
      await page.screenshot({ path: path.join(SHOTS!, `apercu-${kind}-${scheme}.png`) });
    }
  });
}

/**
 * What each preview costs in memory, written out (`ESCOUADE_E2E_MEASURE=1`): the page's JavaScript heap (DevTools'
 * metrics) and the working set of all the WebView2 processes of the test's window, after the view started, then after
 * each preview opened (and the tabs before it closed), then after all were closed.
 */
test('the memory of the previews, measured', async ({ app }) => {
  test.skip(!process.env.ESCOUADE_E2E_MEASURE, 'a measure (ESCOUADE_E2E_MEASURE=1)');
  test.setTimeout(300_000);
  const { page } = app;
  await start(page, app.repo);
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Performance.enable');
  const folder = path.join(app.root, 'webview').replace(/'/g, "''");
  const measure = async (what: string) => {
    await page.waitForTimeout(1500);
    await cdp.send('HeapProfiler.collectGarbage');
    const { metrics } = await cdp.send('Performance.getMetrics');
    const heap = metrics.find((m) => m.name === 'JSHeapUsedSize')!.value;
    const ws = execFileSync(
      'powershell',
      [
        '-NoProfile',
        '-Command',
        `(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -like '*${folder}*' } | Measure-Object -Property WorkingSetSize -Sum).Sum`,
      ],
      { encoding: 'utf8' },
    ).trim();
    const mb = (n: number) => (n / 1024 / 1024).toFixed(1);
    console.log(`MEASURE ${what}: heap ${mb(heap)} Mo, WebView2 ${mb(Number(ws))} Mo`);
  };
  await openInExplorer(page, 'src/app.ts');
  await measure('view started, a text file open');
  const cases: [string, string, string, () => Promise<void>][] = [
    ['assets/photo.png', 'photo.png', 'image', () => expect(preview(page, 'image').locator('img.pv-image')).toBeVisible()],
    ['docs/guide.md', 'Aperçu guide.md', 'markdown', () => expect(preview(page, 'markdown').locator('h1')).toBeVisible()],
    ['docs/rapport.pdf', 'rapport.pdf', 'pdf', () => expect(preview(page, 'pdf').locator('.textLayer').first()).toContainText('Escouade')],
    ['docs/rapport.docx', 'rapport.docx', 'docx', () => expect(preview(page, 'docx').locator('h1')).toBeVisible()],
  ];
  for (const [file, tab, kind, ready] of cases) {
    await openInExplorer(page, file, tab);
    await ready();
    await measure(`${kind} open`);
    await closeTab(page, tab);
  }
  await measure('all previews closed');
});
