// The captures of the assistant of the features (F-2, ESC-559): each feature's place in the real app, on the made-up demo of
// demo.ts, in both themes and both languages. `npm run site-shots -- --grep "features"` takes them again (after
// `npx tauri build --debug --no-bundle` and `sh scripts/plugins-wasm.sh`, as every site shot): the window at 1440 × 810,
// brought down to 960 × 540, JPEG quality 82, into public/features/<fr|en>/<feature>-<dark|light>.jpg, which the app
// shows (FeatureTour.svelte). They change with the interface: `git status` shows them.

import fs from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import path from 'node:path';
import { chromium, type Page } from '@playwright/test';
import { en } from '../../src/lib/i18n/en';
import { fr } from '../../src/lib/i18n/fr';
import { FAKE_CODEX, FAKE_CURSOR, expect, test as base } from '../e2e/fixture';
import { PROJECTS, makeDemo, textOf } from './demo';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const OUT = process.env.ESCOUADE_SHOTS_OUT ?? path.join(ROOT, 'public', 'features');
const WINDOW = { w: 1440, h: 810 };
const SHOT = { w: 960, h: 540 };
const FEATURES = ['kanban', 'editor', 'docs', 'cost', 'stats', 'workflows', 'scheduled', 'overseer', 'voice'] as const;
type Feature = (typeof FEATURES)[number];

/** A spec, a decision and a workflow in demo-api, and a scheduled task: their places are not empty in the pictures. */
function enrich(root: string, lang: 'fr' | 'en') {
  const api = path.join(root, PROJECTS.api.name);
  const spec = path.join(api, '.escouade', 'spec', 'api');
  fs.mkdirSync(spec, { recursive: true });
  const fr_ = lang === 'fr';
  fs.writeFileSync(
    path.join(spec, 'pagination.md'),
    `---\ntitle: ${fr_ ? 'Pagination' : 'Pagination'}\n---\n\n## ${fr_ ? 'Ce que c’est' : 'What it is'}\n\n${
      fr_ ? 'Chaque liste de l’API se lit page par page, par un curseur.' : 'Every list of the API is read page by page, with a cursor.'
    }\n`,
  );
  fs.writeFileSync(
    path.join(spec, 'auth.md'),
    `---\ntitle: ${fr_ ? 'Authentification' : 'Authentication'}\n---\n\n${fr_ ? 'Un jeton par session.' : 'One token per session.'}\n`,
  );
  const agents = path.join(api, '.claude', 'agents');
  fs.mkdirSync(agents, { recursive: true });
  fs.writeFileSync(
    path.join(agents, 'reviewer.md'),
    `---\nname: reviewer\ndescription: ${fr_ ? 'Relit chaque PR avant le merge.' : 'Reviews each PR before the merge.'}\n---\n\n${
      fr_ ? 'Relis la PR.' : 'Review the PR.'
    }\n`,
  );
  const file = path.join(root, 'data', 'state.json');
  const state = JSON.parse(fs.readFileSync(file, 'utf8'));
  state.scheduled = [
    {
      id: 's1',
      projectId: PROJECTS.api.id,
      title: fr_ ? 'Revue des dépendances' : 'Dependency review',
      description: fr_ ? 'Mets à jour les dépendances.' : 'Update the dependencies.',
      rhythm: { kind: 'weekly', days: [1], hour: 9, minute: 0 },
      createdAt: Date.now(),
    },
  ];
  fs.writeFileSync(file, JSON.stringify(state));
}

/** The window brought down to the assistant's size, as JPEG. */
async function compose(png: Buffer, file: string) {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: SHOT.w, height: SHOT.h }, deviceScaleFactor: 1 });
  await page.setContent(`<body style="margin:0;overflow:hidden">
    <img src="data:image/png;base64,${png.toString('base64')}" style="display:block;width:${SHOT.w}px;height:${SHOT.h}px"></body>`);
  await page.evaluate(() => document.images[0].decode());
  fs.mkdirSync(path.dirname(file), { recursive: true });
  await page.screenshot({ path: file, type: 'jpeg', quality: 82 });
  await browser.close();
}

const test = base.extend({
  appEnv: async ({ language }, use) => {
    // No update to offer: no banner in the pictures.
    const updates = http.createServer((_, res) =>
      res.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify({ version: '0.0.1', notes: '', platforms: {} })),
    );
    await new Promise<void>((resolve) => updates.listen(0, '127.0.0.1', resolve));
    const { port } = updates.address() as net.AddressInfo;
    await use({
      ESCOUADE_UPDATE_ENDPOINT: `http://127.0.0.1:${port}/latest.json`,
      ESCOUADE_CODEX: FAKE_CODEX,
      ESCOUADE_CURSOR: FAKE_CURSOR,
      FAKE_CLAUDE_DEMO: path.join(process.env.ESCOUADE_E2E_ROOT!, 'demo-turns.json'),
      FAKE_CLAUDE_REMOTE_MESSAGE: textOf(language).remote,
    });
    await new Promise((resolve) => updates.close(resolve));
  },
  preStart: async ({ language }, use) => {
    await use({
      run: (root: string) => {
        makeDemo(root, language, 0);
        enrich(root, language);
        return { theme: 'dark', sound: false, overseer: { enabled: true } };
      },
    });
  },
});

test('the captures of the assistant of the features', async ({ app, language }) => {
  test.setTimeout(900_000);
  const L = language === 'fr' ? fr : en;
  const t = textOf(language);
  const page: Page = app.page;
  await page.setViewportSize({ width: WINDOW.w, height: WINDOW.h });
  const tabs = page.getByRole('group', { name: L.nav.sidebar.viewLabel });
  const card = (name: string) => page.locator('aside.side .agent', { hasText: name }).first();
  const home = async () => {
    await page.keyboard.press('Escape');
    await page
      .locator('nav.rail')
      .getByRole('tab', { name: new RegExp(PROJECTS.api.name) })
      .first()
      .click();
    await tabs.getByRole('button', { name: /^Agents/ }).click();
    await card(t.agents.pagination).click();
  };
  const theme = (th: 'dark' | 'light') =>
    page.evaluate((th) => {
      document.documentElement.dataset.theme = th;
      document.documentElement.style.colorScheme = th;
    }, th);
  const shot = async (f: Feature) => {
    await page.mouse.move(WINDOW.w - 2, WINDOW.h / 2);
    for (const th of ['dark', 'light'] as const) {
      await theme(th);
      await page.waitForTimeout(700);
      await compose(await page.screenshot({ animations: 'disabled', caret: 'hide' }), path.join(OUT, language, `${f}-${th}.jpg`));
    }
    await theme('dark');
  };
  /** Each place reached as well as it can be: a step that fails leaves the picture on what is there, said in the log. */
  const at = async (f: Feature, go: () => Promise<void>) => {
    try {
      await home();
      await go();
    } catch (e) {
      console.warn(`[features-tour] ${language} ${f}: ${String(e).split('\n')[0]}`);
    }
    await shot(f);
  };
  await expect(card(t.agents.pagination)).toBeVisible();
  // A conversation at work: its turns, its cost and its cache's degrees.
  await card(t.agents.pagination).click();
  const box = page.locator('.composer textarea');
  await box.fill(t.pagination.prompt);
  await box.press('Enter');
  await expect(page.getByText('npm run test:e2e').first()).toBeVisible({ timeout: 60_000 });

  await at('kanban', async () => {
    await tabs.getByRole('button', { name: /^Kanban/ }).click();
    await expect(page.getByRole('region', { name: L.board.columns.review })).toBeVisible();
  });
  await at('editor', async () => {
    await page.getByRole('button', { name: L.nav.titleBar.openEditor }).click();
    await page.waitForTimeout(3000);
    await page.keyboard.press('Control+p');
    await page.keyboard.type('paginate');
    await page.getByText('paginate.ts', { exact: true }).first().click({ timeout: 10_000 });
    await page.waitForTimeout(3000);
  });
  await page
    .getByRole('button', { name: /^Conversation/ })
    .first()
    .click()
    .catch(() => {});
  await at('docs', async () => {
    await tabs.getByRole('button', { name: /^Docs/ }).click();
    await page.getByText('Pagination').first().click({ timeout: 10_000 });
  });
  await at('cost', async () => {
    await page.locator('.fig-files').first().waitFor({ timeout: 5_000 });
  });
  await at('workflows', async () => {
    const files = page.locator('button.fig-files').first();
    if ((await files.getAttribute('aria-pressed')) !== 'true') await files.click();
    await page
      .getByRole('tab', { name: /^Workflow/ })
      .first()
      .click({ timeout: 10_000 });
    await page.waitForTimeout(800);
  });
  // The side panel closed again: the next pictures show the sidebar and the conversation.
  await page
    .getByRole('button', { name: L.nav.sidePanel.close })
    .first()
    .click({ timeout: 5_000 })
    .catch(() => {});
  await at('scheduled', async () => {
    const section = page
      .locator('aside.side')
      .getByRole('button', { name: new RegExp(L.scheduled.section.title) })
      .first();
    await section.click({ timeout: 10_000 });
    await page
      .locator('aside.side')
      .getByText(language === 'fr' ? 'Revue des dépendances' : 'Dependency review')
      .first()
      .scrollIntoViewIfNeeded({ timeout: 5_000 });
  });
  await at('overseer', async () => {
    await page.getByRole('button', { name: L.overseer.panel.openTitle }).click({ timeout: 10_000 });
    await page.waitForTimeout(1500);
  });
  await at('voice', async () => {
    await page.locator('.composer').getByRole('button', { name: L.voice.button.label }).first().hover({ timeout: 5_000 });
    await page.waitForTimeout(1200);
  });
  await at('stats', async () => {
    await page.getByRole('button', { name: L.nav.rail.stats, exact: true }).first().click();
  });
});
