// The extensions of Open VSX in the VS Code view (ESC-481, A5), on a fake Open VSX (`fake-open-vsx.ts`, named to the app
// by ESCOUADE_TEST_OPEN_VSX): searched, installed after the question of trust, a theme and a grammar seen applied, an
// extension refused by the allowed list a policy imposes (nothing of it downloaded), extensions with code said waiting,
// an update proposed then installed, an extension turned off and removed. The page never reaches Open VSX: the core does.
// ESCOUADE_E2E_SHOTS=<folder>: captures, dark and light.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { code, grammar, startFakeOpenVsx, theme, type FakeOpenVsx } from './fake-open-vsx';
import { addProject, expect, test } from './fixture';
import { editorLines, openInExplorer, workbench } from './vscode';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const VIEWPORT = process.env.ESCOUADE_E2E_VIEWPORT ?? 'default';
let fake: FakeOpenVsx;

test.use({
  // Dark, whatever the machine's theme: the captures say which one they show (light at the end).
  preStart: { run: () => ({ theme: 'dark' }) },
  appEnv: async ({}, use) => {
    const port = process.env.ESCOUADE_PORT_BASE ? Number(process.env.ESCOUADE_PORT_BASE) + 3 : 0;
    fake = await startFakeOpenVsx(port, [
      theme('acme', 'night', '1.0.0', 'Acme Night', '#3a1f5d'),
      grammar('acme', 'lang', '1.0.0'),
      code('acme', 'web', '1.0.0', true),
      code('acme', 'node', '1.0.0', false),
      theme('blocked', 'theme', '1.0.0', 'Blocked Theme', '#00ff00'),
    ]);
    // A policy of this device imposes the allowed list: everything but the publisher `blocked`.
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-vsx-'));
    const policies = path.join(dir, 'policies.json');
    fs.writeFileSync(policies, JSON.stringify({ 'extensions.allowed': { '*': true, blocked: false } }));
    await use({ ESCOUADE_TEST_OPEN_VSX: fake.url, ESCOUADE_TEST_POLICIES: policies });
    await fake.close();
    fs.rmSync(dir, { recursive: true, force: true });
  },
});

/** The background of the view's editor, as drawn. */
const editorBackground = (page: Page) =>
  page
    .locator('#vscode-layer .editor-instance .monaco-editor-background')
    .first()
    .evaluate((el) => getComputedStyle(el).backgroundColor);

async function shot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.screenshot({ path: path.join(SHOTS, `${name}-${VIEWPORT}.png`) });
}

test('the extensions of Open VSX: searched, checked and installed by the core, a theme and a grammar applied, the allowed list, code waiting, update, removal', async ({
  app,
}) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(300_000);
  const { page } = app;
  fs.mkdirSync(path.join(app.repo, 'src'), { recursive: true });
  fs.writeFileSync(path.join(app.repo, 'src', 'x.acme'), 'fonction calcul()\n  retour 42 # la réponse\n');
  execFileSync('git', ['add', '-A'], { cwd: app.repo });
  execFileSync('git', ['commit', '-qm', 'acme'], { cwd: app.repo });
  // Nothing of the page goes to Open VSX: what it asks is the core's.
  const fromPage: string[] = [];
  page.on('request', (r) => {
    if (r.url().startsWith(fake.url)) fromPage.push(r.url());
  });
  page.on('pageerror', (e) => console.log(`pageerror: ${e.stack ?? e}`.slice(0, 1500)));
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') console.log(`console.${m.type()}: ${m.text().slice(0, 400)}`);
  });

  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  const view = page.getByTestId('vscode-view');
  const wb = workbench(page);
  await openInExplorer(page, 'src/x.acme');
  const lines = editorLines(page);
  await expect(lines).toContainText('fonction calcul()');
  const before = await editorBackground(page);
  // Nothing asked of Open VSX to open the view.
  expect(fake.hits).toEqual([]);

  // 1. The panel, beside the workbench, which keeps its place; the list imposed by the device's policy said.
  // By its accessible name: in a narrow bar, the button keeps its icon alone.
  await view.getByRole('button', { name: 'Extensions', exact: true }).click();
  const panel = page.getByTestId('vscode-ext-panel');
  await expect(panel).toBeVisible();
  await expect(panel.getByTestId('vscode-ext-imposed')).toContainText('Liste permise imposée par la stratégie de ce poste.');
  await expect
    .poll(() =>
      view.locator('.vscode-place').evaluate((place) => {
        const host = document.querySelector<HTMLElement>('#vscode-layer .vscode-host')!.getBoundingClientRect();
        const p = place.getBoundingClientRect();
        const aside = document.querySelector('[data-testid="vscode-ext-panel"]')!.getBoundingClientRect();
        return Math.abs(host.width - p.width) <= 1 && host.right <= aside.left + 1;
      }),
    )
    .toBe(true);

  // 2. Searched by the core; a card; installed after the question of trust of its publisher.
  const search = panel.getByTestId('vscode-ext-search');
  await search.fill('acme');
  await search.press('Enter');
  await expect(panel.getByTestId('vscode-ext-result')).toHaveCount(4);
  await panel.locator('[data-testid="vscode-ext-result"][data-id="acme.night"]').click();
  const card = panel.getByTestId('vscode-ext-card');
  await expect(card).toContainText('Acme Night');
  await expect(card.getByTestId('vscode-ext-verified')).toBeVisible();
  // Its icon by the core (a data: URL); its README's remote image left, its address said.
  await expect(card.locator('.logo img')).toHaveAttribute('src', /^data:image\/png;base64,/, { timeout: 15_000 });
  expect(fake.hits).toContain('/files/acme.night-1.0.0/icon.png');
  await expect(card.getByTestId('vscode-ext-readme')).toContainText('Une extension d’essai.');
  await expect(card.getByTestId('vscode-ext-readme').locator('img')).toHaveCount(0);
  await shot(page, 'vscode-ext-card-dark');
  await card.getByTestId('vscode-ext-install').click();
  const trust = panel.getByTestId('vscode-ext-trust');
  await expect(trust).toContainText('Faites-vous confiance à l’éditeur « acme Inc. » ?');
  await expect(trust).toContainText('Open VSX a vérifié son espace de noms « acme ».');
  // Not downloaded before the answer.
  expect(fake.hits.filter((h) => h.startsWith('/files/acme.night-1.0.0.vsix'))).toEqual([]);
  await trust.getByTestId('vscode-ext-trust-yes').click();
  const installedItem = (id: string) => panel.locator(`[data-testid="vscode-ext-item"][data-id="${id}"]`);
  await card.getByTestId('vscode-ext-install').waitFor({ state: 'detached', timeout: 30_000 });
  // The VSIX, its sum, its signature and the key: each asked by the core.
  for (const f of [
    '/files/acme.night-1.0.0.vsix',
    '/files/acme.night-1.0.0.sha256',
    '/files/acme.night-1.0.0.sigzip',
    '/api/-/public-key/fake',
  ])
    expect(fake.hits).toContain(f);
  const extDir = path.join(app.data, 'extensions', 'acme.night-1.0.0');
  expect(fs.existsSync(path.join(extDir, 'themes', 'theme.json'))).toBe(true);

  // 3. The theme, chosen on purpose: it wins over Escouade's; back to Escouade's in one gesture.
  await card.getByRole('button', { name: 'Retour à la liste' }).click();
  await expect(installedItem('acme.night')).toHaveAttribute('data-status', 'active');
  await expect(editorBackground(page)).resolves.toBe(before);
  await installedItem('acme.night').getByTestId('vscode-ext-use-theme').click();
  await expect.poll(() => editorBackground(page)).toBe('rgb(58, 31, 93)');
  await expect(panel.getByTestId('vscode-ext-theme-on')).toContainText('La vue a le thème « Acme Night ».');
  await shot(page, 'vscode-ext-theme-dark');
  await panel.getByTestId('vscode-ext-theme-off').click();
  await expect.poll(() => editorBackground(page)).toBe(before);

  // 4. A grammar (and a language, snippets): the file `x.acme` coloured, its language named, no code run.
  await search.fill('acme.lang');
  await search.press('Enter');
  await panel.locator('[data-testid="vscode-ext-result"][data-id="acme.lang"]').click();
  await expect(card.getByTestId('vscode-ext-unverified')).toBeVisible();
  await card.getByTestId('vscode-ext-install').click();
  // The publisher trusted already: not asked again.
  await card.getByTestId('vscode-ext-install').waitFor({ state: 'detached', timeout: 30_000 });
  await expect(panel.getByTestId('vscode-ext-trust')).toHaveCount(0);
  await card.getByRole('button', { name: 'Retour à la liste' }).click();
  await expect(installedItem('acme.lang')).toHaveAttribute('data-status', 'active');
  await expect(installedItem('acme.lang')).toContainText('1 grammaire, 1 lot d’extraits, 1 langage');
  await expect(wb.locator('.statusbar-item', { hasText: /^Acme$/ })).toBeVisible({ timeout: 30_000 });
  // The tokens of every line shown (Monaco lays its lines in any order in the DOM).
  const tokenClasses = () =>
    lines
      .locator('.view-line span > span')
      .evaluateAll((els) => els.map((e) => `${(e.textContent ?? '').replace(/ /g, ' ').trim()}|${e.className}`));
  await expect
    .poll(async () => {
      const t = await tokenClasses();
      const kw = t.find((x) => x.startsWith('fonction|'))?.split('|')[1];
      const rest = t.find((x) => x.startsWith('calcul|'))?.split('|')[1];
      return !!kw && !!rest && kw !== rest;
    })
    .toBe(true);

  // 5. Extensions with code: installed, said waiting for their host; nothing of their code ran in the page.
  for (const id of ['acme.web', 'acme.node']) {
    await search.fill(id);
    await search.press('Enter');
    await panel.locator(`[data-testid="vscode-ext-result"][data-id="${id}"]`).click();
    await card.getByTestId('vscode-ext-install').click();
    await card.getByTestId('vscode-ext-install').waitFor({ state: 'detached', timeout: 30_000 });
    await card.getByRole('button', { name: 'Retour à la liste' }).click();
  }
  await expect(installedItem('acme.web')).toHaveAttribute('data-status', 'waitsHost');
  await expect(installedItem('acme.web')).toContainText('En attente de l’hôte d’extensions');
  await expect(installedItem('acme.node')).toHaveAttribute('data-status', 'needsNode');
  await expect(installedItem('acme.node')).toContainText('Demande l’hôte Node, pas encore dans Escouade');
  expect(await page.evaluate(() => (globalThis as { ranAcme?: boolean }).ranAcme ?? false)).toBe(false);

  // 6. Outside the allowed list: refused before anything of it is downloaded.
  await search.fill('blocked');
  await search.press('Enter');
  await panel.locator('[data-testid="vscode-ext-result"][data-id="blocked.theme"]').click();
  await expect(card.getByTestId('vscode-ext-card-refused')).toContainText('n’est pas permise par la stratégie de ce poste');
  await expect(card.getByTestId('vscode-ext-install')).toBeDisabled();
  // Its card shows (what Open VSX says of it), its archive is never asked: no VSIX, sum nor signature.
  expect(fake.hits.filter((h) => /^\/files\/blocked\.theme-1\.0\.0\.(vsix|sha256|sigzip)$/.test(h))).toEqual([]);
  await shot(page, 'vscode-ext-refused-dark');
  await card.getByRole('button', { name: 'Retour à la liste' }).click();

  // 7. An update: proposed, installed only when asked, checked again; the theme's new colour.
  fake.versions.push(theme('acme', 'night', '1.1.0', 'Acme Night', '#1f5d3a'));
  await panel.getByTestId('vscode-ext-check').click();
  const up = panel.locator('[data-testid="vscode-ext-update-item"][data-id="acme.night"]');
  await expect(up).toContainText('1.0.0 → 1.1.0');
  expect(fake.hits).not.toContain('/files/acme.night-1.1.0.vsix');
  await up.getByTestId('vscode-ext-update-one').click();
  await expect(installedItem('acme.night')).toContainText('1.1.0', { timeout: 30_000 });
  expect(fake.hits).toContain('/files/acme.night-1.1.0.sigzip');
  expect(fs.existsSync(extDir)).toBe(false);
  await installedItem('acme.night').getByTestId('vscode-ext-use-theme').click();
  await expect.poll(() => editorBackground(page)).toBe('rgb(31, 93, 58)');

  // 8. Turned off: its theme goes with it, Escouade's again; removed: its folder too.
  await installedItem('acme.night').getByTestId('vscode-ext-toggle').click();
  await expect(installedItem('acme.night')).toHaveAttribute('data-status', 'off');
  await expect.poll(() => editorBackground(page)).toBe(before);
  await installedItem('acme.night').getByTestId('vscode-ext-remove').click();
  await expect(installedItem('acme.night')).toHaveCount(0);
  expect(fs.existsSync(path.join(app.data, 'extensions', 'acme.night-1.1.0'))).toBe(false);

  // Light: the panel and the view in Escouade's light theme.
  if (SHOTS) {
    await page.evaluate(() => (document.documentElement.dataset.theme = 'light'));
    await page.waitForTimeout(1200);
    await shot(page, 'vscode-ext-light');
    await page.evaluate(() => (document.documentElement.dataset.theme = 'dark'));
  }

  // The page itself never asked Open VSX anything.
  expect(fromPage).toEqual([]);
});
