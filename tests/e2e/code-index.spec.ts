// « Index du code » in the app (ESC-254): offered, off, at the last step of « Nouveau projet », with what it does and
// that nothing leaves the machine; turned on there, the project is read and Réglages › Projets says what it holds; the
// model of the search by meaning comes from a fake server of the test's (`ESCOUADE_VECTORS_MODEL_URL`, on a port of
// `ESCOUADE_PORT_BASE` when it is set), its progress shown, « Annuler » keeping what came for « Reprendre »; the
// statistics have their part for the index. `ESCOUADE_E2E_SHOTS`: screenshots in both themes.

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, openNewProject, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
const PORT = base ? base + 8 : 20_000 + Math.floor(Math.random() * 9_000);
const MODEL_URL = `http://127.0.0.1:${PORT}/vectors`;
/** The model's first file, as the catalog has it: served slowly, its bytes not the model's. */
const MODEL_SIZE = 278_184_162;

let server: http.Server;
/** What the fake server was asked: its path and its Range. */
const asked: { url: string; range: string }[] = [];

test.beforeAll(async () => {
  server = http.createServer((req, res) => {
    asked.push({ url: req.url ?? '', range: String(req.headers.range ?? '') });
    const from = Number(/bytes=(\d+)-/.exec(String(req.headers.range ?? ''))?.[1] ?? 0);
    res.writeHead(from ? 206 : 200, { 'Content-Length': String(MODEL_SIZE - from) });
    // About 10 MB/s: the whole would take half a minute, the test calls it off well before.
    const chunk = Buffer.alloc(512 * 1024, 7);
    let sent = from;
    const next = () => {
      if (res.destroyed) return;
      if (sent >= MODEL_SIZE) return res.end();
      const piece = chunk.subarray(0, Math.min(chunk.length, MODEL_SIZE - sent));
      sent += piece.length;
      res.write(piece);
      setTimeout(next, 50);
    };
    next();
  });
  await new Promise<void>((r) => server.listen(PORT, '127.0.0.1', r));
});

test.afterAll(async () => {
  server.closeAllConnections();
  await new Promise((r) => server.close(r));
});

test.use({ appEnv: { ESCOUADE_VECTORS_MODEL_URL: MODEL_URL } });

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

test('the index is offered off at the adding, turned on, shown in the settings, its model downloaded and called off', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await page.setViewportSize({ width: 1280, height: 860 });
  fs.mkdirSync(path.join(app.repo, 'src'), { recursive: true });
  fs.writeFileSync(path.join(app.repo, 'src', 'cart.ts'), 'export function placeOrder() { return checkout(); }\nfunction checkout() {}\n');

  // « Nouveau projet », its last step: the offer, off.
  await openNewProject(page);
  const wizard = page.getByRole('dialog', { name: 'Nouveau projet' });
  await wizard.getByPlaceholder(/chemin.vers.le.projet/).fill(app.repo);
  await expect(wizard.getByText('Dépôt git détecté')).toBeVisible();
  await wizard.getByRole('radio', { name: /^Partir de zéro/ }).click();
  await wizard.getByRole('button', { name: /Récapitulatif/ }).click();
  const offer = wizard.locator('[data-wizard-code-index]');
  await expect(offer.getByText(/trouvent le code plus vite/)).toBeVisible();
  await expect(offer.getByText(/Tout reste sur cette machine/)).toBeVisible();
  const sw = offer.getByRole('switch', { name: 'Indexer le code' });
  await expect(sw).toHaveAttribute('aria-checked', 'false');
  await sw.click();
  await expect(sw).toHaveAttribute('aria-checked', 'true');
  await wizard.getByRole('switch', { name: 'Créer un premier agent' }).click();
  await offer.scrollIntoViewIfNeeded();
  await shots(page, 'assistant');
  await wizard.getByRole('button', { name: 'Créer le projet' }).click();
  await expect(wizard).toBeHidden();

  // Réglages › Projets: on, the project read.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Projets' }).click();
  const group = settings.locator('section[data-anchor="code-index"]');
  await group.scrollIntoViewIfNeeded();
  await expect(group.getByRole('switch', { name: 'Indexer ce projet' })).toHaveAttribute('aria-checked', 'true');
  await expect(group.locator('[data-code-index-status]')).toContainText(/\d+ fichiers? · \d+ symboles?/, { timeout: 30_000 });
  await expect(group.getByText(/Tout reste sur cette machine/)).toBeVisible();

  // « Recherche par le sens »: its download from the fake server, its progress, « Annuler ».
  await group.getByRole('button', { name: /^Télécharger \(\d+ Mo\)$/ }).click();
  const bar = group.getByRole('progressbar', { name: 'Téléchargement du modèle' });
  await expect(bar).toBeVisible();
  await expect.poll(async () => Number(await bar.getAttribute('aria-valuenow')), { timeout: 30_000 }).toBeGreaterThan(0);
  await shots(page, 'reglages-telechargement');
  expect(asked[0].url).toBe('/vectors/onnx/model_int8.onnx');
  await group.getByRole('button', { name: 'Annuler' }).click();
  await expect(bar).toBeHidden({ timeout: 15_000 });
  // What came is kept: « Reprendre » goes on from it, by a Range.
  const resume = group.getByRole('button', { name: /^Reprendre \(\d+ %\)$/ });
  await expect(resume).toBeVisible();
  const part = path.join(app.root, 'data', 'vectors', 'multilingual-e5-base', 'model.onnx.part');
  expect(fs.statSync(part).size).toBeGreaterThan(0);
  await resume.click();
  await expect(bar).toBeVisible();
  await expect.poll(() => asked.length).toBe(2);
  expect(asked[1].range).toMatch(/^bytes=\d+-$/);
  await group.getByRole('button', { name: 'Annuler' }).click();
  await expect(resume).toBeVisible({ timeout: 15_000 });
  await shots(page, 'reglages');

  // Off: the index erased.
  await group.getByRole('switch', { name: 'Indexer ce projet' }).click();
  await expect(group.getByRole('switch', { name: 'Indexer ce projet' })).toHaveAttribute('aria-checked', 'false');
  await expect(page.getByText('Index du code coupé : son index est effacé.')).toBeVisible();
  await group.getByRole('switch', { name: 'Indexer ce projet' }).click();
  await expect(group.locator('[data-code-index-status]')).toContainText(/fichiers?/, { timeout: 30_000 });
  await page.keyboard.press('Escape');
  await expect(settings).toBeHidden();

  // The statistics: the index's part, nothing answered yet.
  await page.getByRole('button', { name: 'Stats' }).click();
  const st = page.locator('main.stats');
  await expect(st.getByRole('heading', { name: 'Index du code' })).toBeVisible();
  await expect(st.getByText('Aucune réponse de l’index sur la période.')).toBeVisible();
  await st.getByRole('heading', { name: 'Index du code' }).scrollIntoViewIfNeeded();
  await shots(page, 'stats');
});
